//! SteamVR dashboard with Head / Left Watch / Right Watch and watch calibration.
use crate::config::Mode;
use crate::devices::DeviceStats;
use font8x8::{BASIC_FONTS, UnicodeFonts};
use openvr::overlay::OverlayHandle;
use std::mem;

const W: usize = 512;
const H: usize = 600;
const THUMB: usize = 96;
const WHITE: [u8;4] = [236,245,250,255];
const BLUE: [u8;4] = [96,171,255,255];
const DIM: [u8;4] = [170,189,205,255];
const ACTIVE: [u8;4] = [39,103,79,255];
const INACTIVE: [u8;4] = [49,62,79,255];

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct State {
    pub enabled: bool,
    pub mode: Mode,
    pub calibrating: Option<Mode>,
    pub watch_width_m: f32,
    pub global_resolution: Option<u16>,
    pub app_resolution: Option<u16>,
    pub combined_resolution: Option<u16>,
}
impl State {
    pub fn new(enabled: bool, mode: Mode, calibrating: Option<Mode>, width_m: f32,
        devices: &DeviceStats) -> Self {
        Self {enabled,mode,calibrating,watch_width_m:width_m,
            global_resolution:devices.global_resolution_pct,
            app_resolution:devices.app_resolution_pct,
            combined_resolution:devices.combined_resolution_pct}
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Action { Toggle, SetMode(Mode), StartCalibration(Mode), SaveCalibration,
    CancelCalibration, SizeDelta(f32) }

fn hit(x:f32,y:f32,b:(f32,f32,f32,f32))->bool {
    x.is_finite() && y.is_finite() && x>=b.0 && x<=b.2 && y>=b.1 && y<=b.3
}
fn action_at(x:f32,y:f32)->Option<Action> {
    if hit(x,y,(16.,101.,165.,148.)) {return Some(Action::SetMode(Mode::Head));}
    if hit(x,y,(181.,101.,330.,148.)) {return Some(Action::SetMode(Mode::Left));}
    if hit(x,y,(346.,101.,496.,148.)) {return Some(Action::SetMode(Mode::Right));}
    if hit(x,y,(16.,181.,224.,242.)) {return Some(Action::Toggle);}
    if hit(x,y,(16.,273.,244.,317.)) {return Some(Action::StartCalibration(Mode::Left));}
    if hit(x,y,(268.,273.,496.,317.)) {return Some(Action::StartCalibration(Mode::Right));}
    if hit(x,y,(16.,339.,100.,379.)) {return Some(Action::SizeDelta(-0.005));}
    if hit(x,y,(412.,339.,496.,379.)) {return Some(Action::SizeDelta(0.005));}
    if hit(x,y,(16.,390.,244.,438.)) {return Some(Action::SaveCalibration);}
    if hit(x,y,(268.,390.,496.,438.)) {return Some(Action::CancelCalibration);}
    None
}

pub struct Dashboard {
    api: *const openvr_sys::VR_IVROverlay_FnTable,
    main: OverlayHandle,
    thumbnail: OverlayHandle,
    image: Image,
    last_state: State,
}
impl Dashboard {
    pub fn new(overlays:&mut openvr::Overlay,state:State)->Result<Self,String> {
        unsafe {
            let mut err=0;
            let api=openvr_sys::VR_GetGenericInterface(c"FnTable:IVROverlay_028".as_ptr(),&mut err)
                as *const openvr_sys::VR_IVROverlay_FnTable;
            if api.is_null() {return Err(format!("Dashboard interface missing: {err}"));}
            let create=(*api).CreateDashboardOverlay.ok_or("CreateDashboardOverlay missing")?;
            let set_scale=(*api).SetOverlayMouseScale.ok_or("SetOverlayMouseScale missing")?;
            if (*api).PollNextOverlayEvent.is_none() {return Err("Overlay event input missing".into());}
            let mut main=0;let mut thumb=0;
            let code=create(c"vrperf.linux.dashboard".as_ptr() as *mut _,
                c"vrperf".as_ptr() as *mut _, &mut main,&mut thumb);
            if code!=0 {return Err(format!("CreateDashboardOverlay: {code}"));}
            let main=OverlayHandle(main);let thumb=OverlayHandle(thumb);
            let mut scale=openvr_sys::HmdVector2_t {v:[W as f32,H as f32]};
            let code=set_scale(main.0,&mut scale);
            if code!=0 {return Err(format!("SetOverlayMouseScale: {code}"));}
            overlays.set_width(main,0.85).map_err(|e|format!("Dashboard width: {e:?}"))?;
            let mut panel=Self{api,main,thumbnail:thumb,image:Image::new(W,H),last_state:state};
            panel.redraw(overlays,state)?;
            let mut icon=Image::new(THUMB,THUMB);
            icon.clear([16,23,31,255]);icon.rect(5,5,86,86,[27,47,65,255]);
            icon.text(24,23,"VR",3,BLUE);icon.text(16,62,"PERF",2,WHITE);
            overlays.set_raw_data(thumb,&icon.pixels,THUMB,THUMB,4)
                .map_err(|e|format!("Dashboard icon: {e:?}"))?;
            Ok(panel)
        }
    }
    pub fn refresh(&mut self,overlays:&mut openvr::Overlay,state:State)->Result<(),String> {
        if state!=self.last_state {self.redraw(overlays,state)?;self.last_state=state;}
        Ok(())
    }
    fn redraw(&mut self,overlays:&mut openvr::Overlay,state:State)->Result<(),String> {
        self.image.clear([16,23,31,255]);
        self.image.text(24,15,"vrperf",3,WHITE);
        self.image.text(24,62,"Performance overlay",2,DIM);
        for (mode,x,label) in [(Mode::Head,16,"HEAD"),(Mode::Left,181,"LEFT"),(Mode::Right,346,"RIGHT")] {
            self.image.rect(x,101,149,47,if state.mode==mode {ACTIVE} else {INACTIVE});
            self.image.text(x+22,116,label,2,WHITE);
        }
        self.image.text(16,160,"VISIBILITY",2,DIM);
        self.image.rect(16,181,208,61,if state.enabled {ACTIVE} else {[78,61,65,255]});
        self.image.text(if state.enabled {93} else {83},199,
            if state.enabled {"ON"} else {"OFF"},3,WHITE);
        self.image.text(272,184,"HEAD / WATCH L/R",1,DIM);
        self.image.text(272,210,"FIXED OPACITY",2,DIM);
        self.image.text(16,251,"CALIBRATE WATCH",2,DIM);
        for (mode,x,label) in [(Mode::Left,16,"CAL LEFT"),(Mode::Right,268,"CAL RIGHT")] {
            self.image.rect(x,273,228,44,if state.calibrating==Some(mode) {ACTIVE} else {INACTIVE});
            self.image.text(x+36,284,label,2,WHITE);
        }
        self.image.text(16,320,"WATCH SIZE (DURING CALIBRATION)",2,DIM);
        self.image.rect(16,339,84,40,INACTIVE);
        self.image.text(44,348,"-",3,WHITE);
        self.image.rect(412,339,84,40,INACTIVE);
        self.image.text(442,348,"+",3,WHITE);
        let size_text=if state.mode==Mode::Head { "--".to_string() }
            else { format!("{:.1} CM",state.watch_width_m*100.0) };
        self.image.text(188,349,&size_text,2,WHITE);
        for (x,label,active) in [(16,"SAVE",state.calibrating.is_some()),
            (268,"CANCEL",state.calibrating.is_some())] {
            self.image.rect(x,390,228,48,if active {ACTIVE} else {INACTIVE});
            self.image.text(x+66,403,label,2,WHITE);
        }
        // Explicit instruction: the SteamVR dashboard must be CLOSED while
        // moving the watch. Calibration can continue inside a running game.
        let instructions = match state.calibrating {
            Some(Mode::Left) => [
                "CLOSE STEAMVR DASHBOARD",
                "IN GAME: RELEASE RIGHT GRIP",
                "HOLD RIGHT GRIP; MOVE WATCH",
                "REOPEN DASHBOARD; SAVE/CANCEL",
            ],
            Some(Mode::Right) => [
                "CLOSE STEAMVR DASHBOARD",
                "IN GAME: RELEASE LEFT GRIP",
                "HOLD LEFT GRIP; MOVE WATCH",
                "REOPEN DASHBOARD; SAVE/CANCEL",
            ],
            _ => [
                "1. SELECT CAL LEFT OR RIGHT",
                "2. CLOSE STEAMVR DASHBOARD",
                "3. GRIP OTHER HAND; MOVE",
                "4. REOPEN DASHBOARD; SAVE",
            ],
        };
        for (line,y) in instructions.into_iter().zip([445,469,493,517]) {
            self.image.text(16,y,line,2,if state.calibrating.is_some() {WHITE} else {DIM});
        }
        self.image.rect(16,548,480,1,[45,59,75,255]);
        let res=|x:Option<u16>|x.map_or_else(||"--".to_string(),|v|format!("{v}%"));
        self.image.text(16,556,&format!("GLOBAL {}",res(state.global_resolution)),1,DIM);
        self.image.text(256,556,&format!("APP {}",res(state.app_resolution)),1,DIM);
        self.image.text(16,576,&format!("COMBINED RES {}",res(state.combined_resolution)),1,WHITE);
        overlays.set_raw_data(self.main,&self.image.pixels,W,H,4)
            .map_err(|e|format!("Dashboard image upload: {e:?}"))
    }
    pub fn poll(&mut self)->Result<Option<Action>,String> {
        unsafe {
            let poll=(*self.api).PollNextOverlayEvent.expect("checked at creation");
            let mut action=None;
            for _ in 0..128 {
                let mut event:openvr_sys::VREvent_t=mem::zeroed();
                if !poll(self.main.0,&mut event,mem::size_of::<openvr_sys::VREvent_t>() as u32) {break;}
                if event.eventType==301 && event.data.mouse.button & 1!=0 {
                    let mouse=event.data.mouse;
                    action=action_at(mouse.x,H as f32-mouse.y).or(action);
                }
            }
            Ok(action)
        }
    }
}
impl Drop for Dashboard {
    fn drop(&mut self) {
        // SAFETY: OpenVR context must outlive this struct.
        unsafe {
            if let Some(destroy) = (*self.api).DestroyOverlay {
                let _ = destroy(self.main.0);
                let _ = destroy(self.thumbnail.0);
            }
        }
    }
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
                        self.rect(ox + col * scale, y + row as i32 * scale, scale, scale, color);
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
    fn dashboard_has_only_head_and_watch_controls() {
        assert_eq!(action_at(80.,120.),Some(Action::SetMode(Mode::Head)));
        assert_eq!(action_at(250.,120.),Some(Action::SetMode(Mode::Left)));
        assert_eq!(action_at(400.,120.),Some(Action::SetMode(Mode::Right)));
        assert_eq!(action_at(80.,200.),Some(Action::Toggle));
        assert_eq!(action_at(80.,295.),Some(Action::StartCalibration(Mode::Left)));
        assert_eq!(action_at(400.,295.),Some(Action::StartCalibration(Mode::Right)));
        assert_eq!(action_at(80.,420.),Some(Action::SaveCalibration));
        assert_eq!(action_at(400.,420.),Some(Action::CancelCalibration));
        assert_eq!(action_at(50.,350.),Some(Action::SizeDelta(-0.005)));
        assert_eq!(action_at(450.,350.),Some(Action::SizeDelta(0.005)));
        assert_eq!(action_at(f32::NAN,120.),None);
    }
    #[test] fn dashboard_dimensions() {assert_eq!(Image::new(W,H).pixels.len(),W*H*4);}
}
