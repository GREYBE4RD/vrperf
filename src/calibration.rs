//! Per-hand rigid watch placement and persistence. All matrices are 3x4
//! column-vector rigid transforms in SteamVR's standing tracking space.
use crate::config::Mode;

pub type Matrix = [[f32; 4]; 3];
const DEFAULT_WATCH_WIDTH: f32 = 0.14;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchMount {
    pub transform: Matrix,
    pub width_m: f32,
}

impl Default for WatchMount {
    fn default() -> Self {
        // Uncalibrated: flat panel 7 cm above the tracked controller, as in
        // the earlier Above wrist placement. A saved calibration overrides it.
        // In the tested controller coordinate system, Above is negative Z.
        Self { transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -0.07],
        ], width_m: DEFAULT_WATCH_WIDTH }
    }
}

pub fn hand_index(mode: Mode) -> Option<usize> {
    match mode { Mode::Head => None, Mode::Left => Some(0), Mode::Right => Some(1) }
}

pub fn multiply(a: Matrix, b: Matrix) -> Matrix {
    let mut out = [[0.0; 4]; 3];
    for i in 0..3 {
        for j in 0..3 {
            out[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
        out[i][3] = a[i][3] + (0..3).map(|k| a[i][k] * b[k][3]).sum::<f32>();
    }
    out
}

pub fn inverse(m: Matrix) -> Matrix {
    let mut result = [[0.0; 4]; 3];
    // Valid SteamVR pose matrices are rigid transforms: R^-1 = transpose(R).
    for i in 0..3 {
        for j in 0..3 { result[i][j] = m[j][i]; }
        result[i][3] = -(0..3).map(|k| m[k][i] * m[k][3]).sum::<f32>();
    }
    result
}

pub fn valid(mount: &WatchMount) -> bool {
    let m = &mount.transform;
    if !mount.width_m.is_finite() || !(0.09..=0.26).contains(&mount.width_m)
        || !m.iter().flatten().all(|v| v.is_finite()) { return false; }
    let norm = |i: usize, j: usize| -> f32 { (0..3).map(|k| m[k][i] * m[k][j]).sum() };
    for i in 0..3 {
        if (norm(i, i) - 1.0).abs() > 0.015 { return false; }
        for j in i+1..3 { if norm(i, j).abs() > 0.015 { return false; } }
    }
    let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if (determinant - 1.0).abs() > 0.02 { return false; }
    let distance = (m[0][3] * m[0][3] + m[1][3] * m[1][3] + m[2][3] * m[2][3]).sqrt();
    distance <= 0.8
}

pub fn serialize(mount: &WatchMount) -> String {
    let mut result = format!("{:.8}", mount.width_m);
    for row in mount.transform {
        for v in row { result.push_str(&format!(" {v:.8}")); }
    }
    result.push('\n');
    result
}

pub fn deserialize(text: &str) -> Option<WatchMount> {
    let numbers: Vec<f32> = text.split_whitespace()
        .map(str::parse::<f32>).collect::<Result<_, _>>().ok()?;
    if numbers.len() != 13 { return None; }
    let mut transform = [[0.0; 4]; 3];
    for i in 0..3 { for j in 0..4 { transform[i][j] = numbers[1 + i * 4 + j]; } }
    let mount = WatchMount { width_m: numbers[0], transform };
    valid(&mount).then_some(mount)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn approx(a: f32, b: f32) { assert!((a-b).abs() < 0.0001, "{a} != {b}"); }
    #[test]
    fn inverse_and_multiply_preserve_pose() {
        let m = WatchMount::default().transform;
        let eye = multiply(inverse(m), m);
        for i in 0..3 { for j in 0..4 {
            approx(eye[i][j], if i==j {1.0} else {0.0});
        }}
        let eye = multiply(m, inverse(m));
        for i in 0..3 { for j in 0..4 {
            approx(eye[i][j], if i==j {1.0} else {0.0});
        }}
    }
    #[test]
    fn roundtrip_and_invalid_values() {
        let m = WatchMount::default();
        assert!(valid(&m));
        let decoded = deserialize(&serialize(&m)).unwrap();
        for i in 0..3 { for j in 0..4 { approx(decoded.transform[i][j], m.transform[i][j]); } }
        assert!(deserialize("bad data").is_none());
        assert!(deserialize(&serialize(&WatchMount { width_m: f32::NAN, ..m })).is_none());
        assert!(deserialize(&serialize(&WatchMount { width_m: 4.0, ..m })).is_none());
        let mut invalid = m;
        invalid.transform[0][0] = 0.4;
        assert!(!valid(&invalid));
    }
    #[test]
    fn uncalibrated_watch_starts_flat_above_controller() {
        let default = WatchMount::default();
        assert!(valid(&default));
        assert_eq!(default.transform, [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -0.07],
        ]);
    }
    #[test]
    fn each_hand_has_separate_index() {
        assert_eq!(hand_index(Mode::Head), None);
        assert_eq!(hand_index(Mode::Left), Some(0));
        assert_eq!(hand_index(Mode::Right), Some(1));
    }
    #[test]
    fn spatial_grab_moves_and_rotates_watch() {
        let watch = WatchMount::default().transform;
        let other = [[1.,0.,0.,0.2],[0.,1.,0.,0.1],[0.,0.,1.,0.0]];
        let anchor = multiply(inverse(other), watch);
        let moved = [[0.,-1.,0.,0.5],[1.,0.,0.,0.2],[0.,0.,1.,0.1]];
        let result = multiply(moved, anchor);
        let relative = multiply(inverse(moved), result);
        for i in 0..3 {for j in 0..4 { approx(relative[i][j],anchor[i][j]);}}
    }
}
