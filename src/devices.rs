//! Low-frequency SteamVR device properties and *global manual* resolution.
//! The effective per-game render scale is not exposed through this settings query.

use openvr::{System, Settings, TrackedControllerRole, TrackedDeviceIndex};

#[derive(Default, Clone, Copy)]
pub struct DeviceStats {
    pub hmd: Option<u8>,
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub global_resolution_pct: Option<u16>,
}

impl DeviceStats {
    pub fn update_batteries(&mut self, system: &System) {
        self.hmd = battery(system, Some(openvr::tracked_device_index::HMD));
        self.left = battery(system, system.tracked_device_index_for_controller_role(
            TrackedControllerRole::LeftHand,
        ));
        self.right = battery(system, system.tracked_device_index_for_controller_role(
            TrackedControllerRole::RightHand,
        ));
    }

    /// The SteamVR global manual scale, not global × per-application scale.
    /// Show unknown when SteamVR is on automatic resolution.
    pub fn update_resolution(&mut self, settings: &mut Settings) {
        let section = c"steamvr";
        let manual = settings.get_bool(section, c"supersampleManualOverride")
            .unwrap_or(false);
        self.global_resolution_pct = if manual {
            settings.get_float(section, c"supersampleScale").ok()
                .filter(|v| v.is_finite() && (0.1..=10.0).contains(v))
                .map(|v| (v * 100.0).round() as u16)
        } else {
            None
        };
    }
}

fn battery(system: &System, device: Option<TrackedDeviceIndex>) -> Option<u8> {
    let index = device?;
    if !system.is_tracked_device_connected(index) { return None; }
    system.float_tracked_device_property(index, openvr::property::DeviceBatteryPercentage_Float)
        .ok()
        .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
        .map(|v| (v * 100.0).round() as u8)
}

/// Uses the local timezone set on the Linux PC. No child process or disk I/O.
pub fn local_clock() -> String {
    // SAFETY: localtime_r writes a complete tm struct on success. This call
    // is thread-safe and does not borrow any global mutable storage.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut fields: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut fields).is_null() {
            return "--:--".to_string();
        }
        format!("{:02}:{:02}", fields.tm_hour, fields.tm_min)
    }
}
