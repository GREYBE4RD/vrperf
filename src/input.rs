//! SteamVR action-based controller input for calibration.
//! Per-controller default bindings map grip and trigger to the same actions.
use crate::config::Mode;
use std::ffi::CString;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::time::{Duration, Instant};

const INVALID_INPUT_HANDLE: u64 = 0; // OpenVR k_ulInvalidInputValueHandle

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Digital { active: bool, pressed: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HandInput { grip: Digital, trigger: Digital }
impl HandInput {
    fn grab(self) -> Option<bool> {
        (self.grip.active || self.trigger.active)
            .then_some(self.grip.pressed || self.trigger.pressed)
    }
}

pub struct Input {
    api: *const openvr_sys::VR_IVRInput_FnTable,
    action_set: u64,
    actions: [[u64; 2]; 2], // [left/right][grip/trigger]
    last: [Option<HandInput>; 2],
    last_inactive_warning: Instant,
    last_error_warning: Instant,
}

impl Input {
    pub fn new() -> Result<Self, String> {
        // Must run after OpenVR init, but before the first PollNextEvent.
        let path = crate::registration::ensure_input_assets()?;
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| "Invalid action manifest path".to_string())?;
        unsafe {
            let mut error = 0;
            let api = openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVRInput_010".as_ptr(), &mut error,
            ) as *const openvr_sys::VR_IVRInput_FnTable;
            if api.is_null() { return Err(format!("OpenVR IVRInput_010 not available ({error})")); }
            let set_path = (*api).SetActionManifestPath.ok_or("SetActionManifestPath unavailable")?;
            check("SetActionManifestPath", set_path(path.as_ptr() as *mut _))?;
            let get_set = (*api).GetActionSetHandle.ok_or("GetActionSetHandle unavailable")?;
            let get_action = (*api).GetActionHandle.ok_or("GetActionHandle unavailable")?;
            if (*api).UpdateActionState.is_none() || (*api).GetDigitalActionData.is_none() {
                return Err("OpenVR action polling unavailable".into());
            }
            let mut action_set = 0;
            check("GetActionSetHandle", get_set(
                c"/actions/calibration".as_ptr() as *mut _, &mut action_set))?;
            let mut actions = [[0; 2]; 2];
            for (i, hand) in ["left", "right"].iter().enumerate() {
                for (j, button) in ["grip", "trigger"].iter().enumerate() {
                    let name = CString::new(format!("/actions/calibration/in/{hand}_{button}"))
                        .expect("static action name");
                    check("GetActionHandle", get_action(name.as_ptr() as *mut _, &mut actions[i][j]))?;
                }
            }
            eprintln!("vrperf: SteamVR action input initialised (grip and trigger actions)");
            Ok(Self {api, action_set, actions, last:[None,None],
                last_inactive_warning:Instant::now()-Duration::from_secs(30),
                last_error_warning:Instant::now()-Duration::from_secs(30)})
        }
    }

    fn digital(&self, handle: u64) -> Result<Digital, String> {
        unsafe {
            let read = (*self.api).GetDigitalActionData.expect("checked at init");
            let mut data: openvr_sys::InputDigitalActionData_t = mem::zeroed();
            check("GetDigitalActionData", read(handle, &mut data,
                mem::size_of_val(&data) as u32, INVALID_INPUT_HANDLE))?;
            Ok(Digital {active:data.bActive, pressed:data.bActive && data.bState})
        }
    }

    // Missing bindings are reported as None, not as a released grip. This
    // stops calibration from arming silently when controller input is absent.
    pub fn poll(&mut self, calibrating: Option<Mode>) -> [Option<bool>; 2] {
        let Some(mode) = calibrating else {
            self.last = [None, None];
            return [None, None];
        };
        let result = (|| {
            unsafe {
                let update = (*self.api).UpdateActionState.expect("checked at init");
                let mut set = openvr_sys::VRActiveActionSet_t {
                    ulActionSet:self.action_set, ulRestrictedToDevice:INVALID_INPUT_HANDLE,
                    ulSecondaryActionSet:0, unPadding:0, nPriority:0,
                };
                check("UpdateActionState", update(&mut set, mem::size_of_val(&set) as u32, 1))?;
            }
            Ok::<_, String>([
                HandInput {grip:self.digital(self.actions[0][0])?,
                    trigger:self.digital(self.actions[0][1])?},
                HandInput {grip:self.digital(self.actions[1][0])?,
                    trigger:self.digital(self.actions[1][1])?},
            ])
        })();
        let hands = match result {
            Ok(hands) => hands,
            Err(error) => {
                if self.last_error_warning.elapsed() >= Duration::from_secs(5) {
                    eprintln!("vrperf: calibration input: {error}");
                    self.last_error_warning = Instant::now();
                }
                return [None, None];
            }
        };
        // CAL LEFT uses the RIGHT hand to position the left watch, and vice versa.
        let other = if mode == Mode::Left {1} else {0};
        if self.last[other] != Some(hands[other]) {
            let h = hands[other];
            eprintln!("vrperf: {:?} controller input: grip={} trigger={} (active: grip={} trigger={})",
                if other==0 {Mode::Left} else {Mode::Right},
                h.grip.pressed, h.trigger.pressed, h.grip.active, h.trigger.active);
        }
        if hands[other].grab().is_none() && self.last_inactive_warning.elapsed() >= Duration::from_secs(5) {
            eprintln!("vrperf: no active {:?} controller bindings; check SteamVR bindings for vrperf",
                if other==0 {Mode::Left} else {Mode::Right});
            self.last_inactive_warning = Instant::now();
        }
        self.last = [Some(hands[0]),Some(hands[1])];
        [hands[0].grab(),hands[1].grab()]
    }
}

fn check(method: &str, code: openvr_sys::EVRInputError) -> Result<(), String> {
    if code == 0 {Ok(())} else {Err(format!("{method} failed: OpenVR error {code}"))}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn input_requires_active_binding() {
        let empty = Digital::default();
        assert_eq!(HandInput {grip:empty,trigger:empty}.grab(),None);
        assert_eq!(HandInput {grip:Digital{active:true,pressed:false},trigger:empty}.grab(),Some(false));
        assert_eq!(HandInput {grip:Digital{active:true,pressed:true},trigger:empty}.grab(),Some(true));
        assert_eq!(HandInput {grip:empty,trigger:Digital{active:true,pressed:true}}.grab(),Some(true));
    }
}
