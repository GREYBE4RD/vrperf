//! Small, native OpenVR dashboard tab. No GUI toolkit or extra process.
//! The dashboard's raw images are uploaded only on creation and on a toggle;
//! the frequently updated performance overlay retains its OpenGL texture.

use font8x8::{BASIC_FONTS, UnicodeFonts};
use openvr::overlay::OverlayHandle;
use std::mem;

const W: usize = 512;
const H: usize = 240;
const THUMB: usize = 96;
const BUTTON: (f32, f32, f32, f32) = (132.0, 103.0, 380.0, 174.0);
const WHITE: [u8; 4] = [236, 245, 250, 255];
const BLUE: [u8; 4] = [96, 171, 255, 255];
const DIM: [u8; 4] = [170, 189, 205, 255];

pub struct Dashboard {
    api: *const openvr_sys::VR_IVROverlay_FnTable,
    main: OverlayHandle,
    thumbnail: OverlayHandle,
    image: Image,
}

impl Dashboard {
    pub fn new(overlays: &mut openvr::Overlay, enabled: bool) -> Result<Self, String> {
        // SAFETY: OpenVR has already been initialized. The table remains valid
        // until the Context is dropped, after this Dashboard is dropped.
        unsafe {
            let mut init_error = 0;
            let api = openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVROverlay_028".as_ptr(), &mut init_error,
            ) as *const openvr_sys::VR_IVROverlay_FnTable;
            if api.is_null() {
                return Err(format!("OpenVR dashboard interface unavailable: {init_error}"));
            }
            let create = (*api).CreateDashboardOverlay
                .ok_or("CreateDashboardOverlay is unavailable")?;
            let set_scale = (*api).SetOverlayMouseScale
                .ok_or("SetOverlayMouseScale is unavailable")?;
            if (*api).PollNextOverlayEvent.is_none() {
                return Err("PollNextOverlayEvent is unavailable".into());
            }

            let mut main = 0;
            let mut thumbnail = 0;
            let result = create(
                c"vrperf.linux.dashboard".as_ptr() as *mut _,
                c"vrperf".as_ptr() as *mut _,
                &mut main,
                &mut thumbnail,
            );
            if result != 0 { return Err(format!("CreateDashboardOverlay: {result}")); }
            let main = OverlayHandle(main);
            let thumbnail = OverlayHandle(thumbnail);
            let mut scale = openvr_sys::HmdVector2_t { v: [W as f32, H as f32] };
            let result = set_scale(main.0, &mut scale);
            if result != 0 { return Err(format!("SetOverlayMouseScale: {result}")); }
            overlays.set_width(main, 0.85)
                .map_err(|e| format!("Dashboard width: {e:?}"))?;

            let mut dashboard = Self { api, main, thumbnail, image: Image::new(W, H) };
            dashboard.redraw(overlays, enabled)?;
            let mut icon = Image::new(THUMB, THUMB);
            icon.clear([16, 23, 31, 255]);
            icon.rect(5, 5, 86, 86, [27, 47, 65, 255]);
            icon.text(24, 23, "VR", 3, BLUE);
            icon.text(16, 62, "PERF", 2, WHITE);
            overlays.set_raw_data(dashboard.thumbnail, &icon.pixels, THUMB, THUMB, 4)
                .map_err(|e| format!("Dashboard icon upload: {e:?}"))?;
            Ok(dashboard)
        }
    }

    fn redraw(&mut self, overlays: &mut openvr::Overlay, enabled: bool) -> Result<(), String> {
        self.image.clear([16, 23, 31, 255]);
        self.image.text(30, 18, "vrperf", 3, WHITE);
        self.image.text(30, 65, "Performance overlay", 2, DIM);
        let accent = if enabled { [39, 103, 79, 255] } else { [78, 61, 65, 255] };
        self.image.rect(132, 103, 248, 71, accent);
        self.image.text(if enabled { 232 } else { 220 }, 127, if enabled { "ON" } else { "OFF" }, 3, WHITE);
        self.image.text(30, 205, "Select to show or hide", 2, DIM);
        overlays.set_raw_data(self.main, &self.image.pixels, W, H, 4)
            .map_err(|e| format!("Dashboard image upload: {e:?}"))
    }

    /// Returns true only if the user pressed the toggle this polling cycle.
    /// OpenVR mouse coordinates are measured from the lower-left corner.
    pub fn poll(&mut self, overlays: &mut openvr::Overlay, enabled: bool) -> Result<bool, String> {
        // SAFETY: We own the dashboard handle and pass the SDK-sized event
        // structure. We only inspect the mouse union on mouse-button events.
        unsafe {
            let poll = (*self.api).PollNextOverlayEvent.expect("checked at creation");
            let mut toggled = false;
            for _ in 0..128 {
                let mut event: openvr_sys::VREvent_t = mem::zeroed();
                if !poll(self.main.0, &mut event, mem::size_of::<openvr_sys::VREvent_t>() as u32) {
                    break;
                }
                if event.eventType == 301 {
                    let mouse = event.data.mouse;
                    if mouse.button & 1 != 0 && hit_button(mouse.x, H as f32 - mouse.y) {
                        toggled = !toggled;
                    }
                }
            }
            if toggled { self.redraw(overlays, !enabled)?; }
            Ok(toggled)
        }
    }
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        // SAFETY: The OpenVR context outlives this struct.
        unsafe {
            if let Some(destroy) = (*self.api).DestroyOverlay {
                let _ = destroy(self.main.0);
                let _ = destroy(self.thumbnail.0);
            }
        }
    }
}

fn hit_button(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite() &&
        x >= BUTTON.0 && x <= BUTTON.2 && y >= BUTTON.1 && y <= BUTTON.3
}

struct Image {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}
impl Image {
    fn new(width: usize, height: usize) -> Self {
        Self { width, height, pixels: vec![0; width * height * 4] }
    }
    fn clear(&mut self, color: [u8; 4]) {
        for px in self.pixels.chunks_exact_mut(4) { px.copy_from_slice(&color); }
    }
    fn pixel(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return; }
        let i = (y as usize * self.width + x as usize) * 4;
        self.pixels[i..i + 4].copy_from_slice(&color);
    }
    fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 4]) {
        for py in y..y + height { for px in x..x + width { self.pixel(px, py, color); } }
    }
    fn text(&mut self, x: i32, y: i32, text: &str, scale: i32, color: [u8; 4]) {
        for (idx, ch) in text.chars().enumerate() {
            let Some(bitmap) = BASIC_FONTS.get(ch) else { continue; };
            let ox = x + idx as i32 * scale * 8;
            for (row, bits) in bitmap.iter().enumerate() {
                for col in 0..8 {
                    if (*bits & (1_u8 << col)) != 0 {
                        self.rect(ox + col * scale, y + row as i32 * scale,
                            scale, scale, color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dashboard_button_hit_test() {
        assert!(hit_button(256.0, 140.0));
        assert!(!hit_button(256.0, 90.0));
        assert!(!hit_button(f32::NAN, 120.0));
    }
    #[test]
    fn image_size() {
        let i = Image::new(W, H);
        assert_eq!(i.pixels.len(), W * H * 4);
    }
}
