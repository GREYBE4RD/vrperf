//! Head display and controller-relative watch display. Watch placement uses
//! independently calibrated 3x4 rigid transforms for the left and right hand.
use crate::calibration::{hand_index, inverse, multiply, valid, Matrix, WatchMount};
use crate::config::{Config, Mode};
use openvr::TrackedDeviceIndex;

const WATCH_SHOW_COS: f32 = 0.766_044_44; // 40 degrees
const WATCH_HIDE_COS: f32 = 0.5; // 60 degrees
const GAZE_SHOW_COS: f32 = 0.573_576_45; // 55 degrees
const GAZE_HIDE_COS: f32 = 0.342_020_15; // 70 degrees
// The watch raster is 550/640 of its previous width. Scale the SteamVR
// physical width by the same ratio so glyphs keep their apparent size.
// Leave saved calibration widths untouched; the change is reversible.
const WATCH_DISPLAY_WIDTH_RATIO: f32 = crate::render::WRIST_WIDTH as f32 / 640.0;

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { (0..3).map(|i| a[i]*b[i]).sum() }
fn norm(v: [f32; 3]) -> f32 { dot(v,v).sqrt() }

// The saved matrix's +Z column is the front of the watch plane. This matches
// its actual calibrated orientation, instead of assuming a controller pitch.
fn watch_alignment(controller: Matrix, hmd: Matrix, mount: WatchMount) -> Option<(f32,f32)> {
    let world = multiply(controller, mount.transform);
    let normal = [world[0][2], world[1][2], world[2][2]];
    let toward = [hmd[0][3]-world[0][3],hmd[1][3]-world[1][3],hmd[2][3]-world[2][3]];
    let gaze = [-hmd[0][2],-hmd[1][2],-hmd[2][2]];
    let (dn,dw,dg) = (norm(normal),norm(toward),norm(gaze));
    if dn < 0.001 || dw < 0.01 || dg < 0.001 || !dn.is_finite() || !dw.is_finite() || !dg.is_finite() { return None; }
    let face = dot(normal,toward)/(dn*dw);
    let view = -dot(gaze,toward)/(dg*dw);
    if !(face.is_finite() && view.is_finite()) { return None; }
    Some((face.clamp(-1.0,1.0),view.clamp(-1.0,1.0)))
}
fn wants_visible(face: f32, gaze: f32, already: bool) -> bool {
    if already { face >= WATCH_HIDE_COS && gaze >= GAZE_HIDE_COS }
    else { face >= WATCH_SHOW_COS && gaze >= GAZE_SHOW_COS }
}
fn watch_visible(calibrating: bool, calibrated: bool,
    alignment: Option<(f32,f32)>, already: bool) -> bool {
    if calibrating || !calibrated { return true; }
    alignment.is_some_and(|(face,gaze)| wants_visible(face,gaze,already))
}

#[derive(Debug)]
struct Calibration {
    side: Mode,
    original: WatchMount,
    // Position of the watch relative to the moving opposite-hand controller.
    grab_anchor: Option<Matrix>,
    armed: bool,
}

pub struct Positioner {
    mounts: [WatchMount;2],
    calibrated: [bool;2],
    active: Option<Calibration>,
    device: Option<TrackedDeviceIndex>,
    last_mode: Option<Mode>,
    visible: bool,
    watch_target: bool,
    dirty: bool,
}

impl Positioner {
    pub fn new(saved_mounts: [Option<WatchMount>;2]) -> Self {
        let calibrated = saved_mounts.map(|mount| mount.is_some());
        let mounts = saved_mounts.map(|mount| mount.unwrap_or_default());
        Self { mounts, calibrated, active: None, device: None, last_mode: None,
            visible: false, watch_target: false, dirty: true }
    }
    pub fn mount(&self, mode: Mode) -> Option<WatchMount> { hand_index(mode).map(|i|self.mounts[i]) }
    pub fn calibrating(&self) -> Option<Mode> { self.active.as_ref().map(|c|c.side) }
    pub fn width(&self, mode: Mode, head_width: f32) -> f32 {
        self.mount(mode).map_or(head_width, |m| m.width_m * WATCH_DISPLAY_WIDTH_RATIO)
    }
    pub fn start_calibration(&mut self, side: Mode) -> Result<(), String> {
        let i=hand_index(side).ok_or("Select a watch hand to calibrate")?;
        if self.active.is_some() { self.cancel_calibration(); }
        self.active=Some(Calibration { side, original:self.mounts[i], grab_anchor:None, armed:false });
        self.last_mode=None;
        self.dirty=true;
        Ok(())
    }
    pub fn cancel_calibration(&mut self) {
        if let Some(c) = self.active.take() {
            if let Some(i)=hand_index(c.side) { self.mounts[i]=c.original; }
            self.dirty=true;
        }
    }
    pub fn save_calibration(&mut self) -> Option<(Mode,WatchMount)> {
        let cal=self.active.take()?;
        let i=hand_index(cal.side)?;
        if !valid(&self.mounts[i]) { self.mounts[i]=cal.original; return None; }
        self.calibrated[i]=true;
        self.dirty=true;
        Some((cal.side,self.mounts[i]))
    }
    pub fn adjust_width(&mut self, delta_m: f32) {
        if let Some(i)=self.calibrating().and_then(hand_index) {
            self.mounts[i].width_m=(self.mounts[i].width_m+delta_m).clamp(0.09,0.26);
            self.dirty=true;
        }
    }
    fn set_visible(&mut self, overlays: &mut openvr::Overlay,
        handle: openvr::overlay::OverlayHandle, show: bool) -> Result<(),String> {
        if show!=self.visible {
            overlays.set_visibility(handle,show).map_err(|e|format!("Overlay visibility: {e:?}"))?;
            self.visible=show;
        }
        Ok(())
    }
    fn tick_grab(&mut self,
        target_world: Matrix, other_world: Option<Matrix>, pressed: Option<bool>) {
        let Some(ref mut cal)=self.active else { return; };
        let Some(other_world)=other_world else { cal.grab_anchor=None; return; };
        let Some(pressed)=pressed else { cal.grab_anchor=None; return; };
        if !pressed { cal.armed=true; cal.grab_anchor=None; return; }
        if !cal.armed { return; } // Require a new grip after calibration starts.
        let Some(i)=hand_index(cal.side) else { return; };
        if cal.grab_anchor.is_none() {
            let panel_world=multiply(target_world,self.mounts[i].transform);
            cal.grab_anchor=Some(multiply(inverse(other_world),panel_world));
            eprintln!("vrperf: watch grabbed; move and rotate opposite controller, then release grip/trigger");
        }
        if let Some(anchor)=cal.grab_anchor {
            let panel_world=multiply(other_world,anchor);
            let local=multiply(inverse(target_world),panel_world);
            let candidate=WatchMount {transform:local, ..self.mounts[i]};
            if valid(&candidate) {
                self.mounts[i]=candidate;
                self.dirty=true;
            }
        }
    }
    pub fn update(&mut self, system: &openvr::System, overlays: &mut openvr::Overlay,
        handle: openvr::overlay::OverlayHandle, config: &Config, enabled: bool,
        grab: [Option<bool>; 2]) -> Result<(),String> {
        if !enabled && self.active.is_none() { return self.set_visible(overlays,handle,false); }
        let device=match config.mode {
            Mode::Head => Some(openvr::tracked_device_index::HMD),
            Mode::Left => system.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand),
            Mode::Right => system.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand),
        };
        let Some(device)=device else {
            self.device=None; self.last_mode=None;
            return self.set_visible(overlays,handle,false);
        };
        if config.mode == Mode::Head {
            if self.device!=Some(device) || self.last_mode!=Some(Mode::Head) || self.dirty {
                let [x,y,z]=config.head_position();
                let angle=y.atan2(-z);
                let (s,c)=angle.sin_cos();
                overlays.set_transform_tracked_device_relative(handle,device,&openvr::pose::Matrix3x4([
                    [1.,0.,0.,x],[0.,c,-s,y],[0.,s,c,z]]))
                    .map_err(|e|format!("Head positioning failed: {e:?}"))?;
                self.dirty=false;
            }
            self.device=Some(device);self.last_mode=Some(config.mode);
            return self.set_visible(overlays,handle,true);
        }
        let i=hand_index(config.mode).expect("wrist side");
        let poses=system.device_to_absolute_tracking_pose(openvr::TrackingUniverseOrigin::Standing,0.0);
        let controller=poses.get(device.0 as usize);
        let hmd=poses.get(openvr::tracked_device_index::HMD.0 as usize);
        if controller.is_none_or(|p|!p.pose_is_valid() || !p.device_is_connected())
            || hmd.is_none_or(|p|!p.pose_is_valid() || !p.device_is_connected()) {
            self.watch_target=false;
            return self.set_visible(overlays,handle,false);
        }
        let controller=controller.expect("checked pose");
        let hmd=hmd.expect("checked pose");
        if self.calibrating()==Some(config.mode) {
            let other_role=match config.mode { Mode::Left=>openvr::TrackedControllerRole::RightHand,
                Mode::Right=>openvr::TrackedControllerRole::LeftHand, Mode::Head=>unreachable!() };
            let other_device=system.tracked_device_index_for_controller_role(other_role);
            let other_world=other_device.and_then(|idx|poses.get(idx.0 as usize))
                .filter(|p|p.pose_is_valid() && p.device_is_connected())
                .map(|p|*p.device_to_absolute_tracking());
            let opposite_hand=if config.mode==Mode::Left {1} else {0};
            self.tick_grab(*controller.device_to_absolute_tracking(),other_world,grab[opposite_hand]);
        }
        if self.last_mode!=Some(config.mode) || self.device!=Some(device) {
            self.watch_target=false;
        }
        if self.device!=Some(device) || self.last_mode!=Some(config.mode) || self.dirty {
            overlays.set_transform_tracked_device_relative(handle,device,
                &openvr::pose::Matrix3x4(self.mounts[i].transform))
                .map_err(|e|format!("Watch positioning failed: {e:?}"))?;
            self.dirty=false;
            self.device=Some(device);self.last_mode=Some(config.mode);
        }
        // Before first SAVE, or while calibrating, keep the panel in view.
        let calibrating=self.calibrating()==Some(config.mode);
        let alignment=if calibrating || !self.calibrated[i] { None } else {
            watch_alignment(*controller.device_to_absolute_tracking(),
                *hmd.device_to_absolute_tracking(),self.mounts[i])
        };
        // Immediate threshold transitions; hysteresis prevents angle flicker.
        self.watch_target=watch_visible(calibrating,self.calibrated[i],alignment,self.watch_target);
        self.set_visible(overlays,handle,self.watch_target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_width_is_narrower_without_changing_saved_calibration() {
        let saved = WatchMount::default();
        let pos = Positioner::new([Some(saved), None]);
        assert!((pos.width(Mode::Left, 0.415) - saved.width_m * WATCH_DISPLAY_WIDTH_RATIO).abs() < 0.000001);
        assert!((pos.width(Mode::Right, 0.415) - saved.width_m * WATCH_DISPLAY_WIDTH_RATIO).abs() < 0.000001);
        assert_eq!(pos.width(Mode::Head, 0.415), 0.415);
        assert_eq!(pos.mount(Mode::Left), Some(saved));
        assert_eq!(pos.mount(Mode::Right), Some(saved));
    }
    #[test]
    fn alignment_follows_saved_watch_orientation() {
        let controller=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let hmd=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,1.]];
        let mount=WatchMount {transform: [[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.2]], width_m:0.14};
        let (face,_) = watch_alignment(controller,hmd,mount).unwrap();
        assert!(face>0.99);
        let flipped=WatchMount {transform: [[-1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,-1.,0.2]],width_m:0.14};
        assert!(watch_alignment(controller,hmd,flipped).unwrap().0 < -0.99);
    }
    #[test]
    fn calibration_requires_release_then_grab() {
        let mut pos=Positioner::new([None,None]);
        pos.start_calibration(Mode::Left).unwrap();
        let identity=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let other=[[1.,0.,0.,0.2],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let moved=[[1.,0.,0.,0.3],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let original=pos.mount(Mode::Left).unwrap();
        pos.tick_grab(identity,Some(other),Some(true));
        pos.tick_grab(identity,Some(moved),Some(true));
        assert_eq!(pos.mount(Mode::Left),Some(original));
        pos.tick_grab(identity,Some(other),Some(false));
        pos.tick_grab(identity,Some(other),Some(true));
        pos.tick_grab(identity,Some(moved),Some(true));
        let changed=pos.mount(Mode::Left).unwrap();
        assert!((changed.transform[0][3]-original.transform[0][3]-0.1).abs()<0.0001);
        pos.tick_grab(identity,Some(moved),Some(false));
        assert_eq!(pos.save_calibration(),Some((Mode::Left,changed)));
    }
    #[test]
    fn cancelled_grab_restores_mount() {
        let mut pos=Positioner::new([None,None]);
        pos.start_calibration(Mode::Right).unwrap();
        pos.adjust_width(0.005);
        assert_ne!(pos.mount(Mode::Right),Some(WatchMount::default()));
        pos.cancel_calibration();
        assert_eq!(pos.mount(Mode::Right),Some(WatchMount::default()));
    }
    #[test]
    fn first_use_is_uncalibrated_and_save_marks_only_selected_hand() {
        let saved = WatchMount::default();
        let mut pos=Positioner::new([None,Some(saved)]);
        assert_eq!(pos.calibrated,[false,true]);
        assert_eq!(pos.mount(Mode::Left),Some(saved));
        pos.start_calibration(Mode::Left).unwrap();
        assert_eq!(pos.save_calibration(),Some((Mode::Left,saved)));
        assert_eq!(pos.calibrated,[true,true]);
    }
    #[test]
    fn cancelled_calibration_does_not_mark_hand_saved() {
        let mut pos=Positioner::new([None,None]);
        pos.start_calibration(Mode::Left).unwrap();
        pos.cancel_calibration();
        assert_eq!(pos.calibrated,[false,false]);
    }
    #[test]
    fn watch_visibility_is_immediate_and_uncalibrated_hand_stays_visible() {
        // Uncalibrated/capturing a calibration, ignore the viewing angles.
        assert!(watch_visible(false,false,None,false));
        assert!(watch_visible(true,true,None,false));
        // Once calibrated, use the thresholds directly with no pending timer.
        assert!(!watch_visible(false,true,Some((0.7,0.9)),false));
        assert!(watch_visible(false,true,Some((0.9,0.9)),false));
        assert!(watch_visible(false,true,Some((0.7,0.9)),true));
        assert!(!watch_visible(false,true,Some((0.4,0.9)),true));
        assert!(!watch_visible(false,true,None,true));
    }
    #[test]
    fn visibility_has_hysteresis() {
        assert!(wants_visible(0.9, 0.9, false));
        assert!(!wants_visible(0.7, 0.9, false));
        assert!(wants_visible(0.7, 0.9, true));
        assert!(!wants_visible(0.4, 0.9, true));
    }
}
