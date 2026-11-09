//! Low-frequency SteamVR device properties and configured resolution multipliers.
//! These settings are not the actual application render target dimensions.

use openvr::{System, Settings, TrackedControllerRole, TrackedDeviceIndex};
use std::ffi::CString;

#[derive(Default, Clone, Copy)]
pub struct DeviceStats {
    pub hmd: Option<u8>,
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub global_resolution_pct: Option<u16>,
    pub app_resolution_pct: Option<u16>,
    pub combined_resolution_pct: Option<u16>,
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

    /// SteamVR's explicit global multiplier and app-specific multiplier.
    /// No global estimate is fabricated in automatic mode.
    /// An existing app key without an override uses SteamVR's 100% default.
    pub fn update_resolution(&mut self, settings: &mut Settings, app_key: Option<&str>) {
        let section = c"steamvr";
        let manual = settings.get_bool(section, c"supersampleManualOverride")
            .unwrap_or(false);
        self.global_resolution_pct = if manual {
            settings.get_float(section, c"supersampleScale").ok()
                .filter(|v| v.is_finite() && (0.1..=10.0).contains(v))
                .map(|v| (v * 100.0).round() as u16)
        } else { None };

        self.app_resolution_pct = app_key.and_then(|key| {
            let section = CString::new(key).ok()?;
            // Older SteamVR releases store per-app resolutionScale as an
            // integer percentage. Some configurations use fractional
            // supersampleScale instead.
            settings.get_int32(&section, c"resolutionScale").ok()
                .filter(|value| (10..=1000).contains(value))
                .map(|value| value as u16)
                .or_else(|| settings.get_float(&section, c"supersampleScale").ok()
                    .filter(|value| value.is_finite() && (0.1..=10.0).contains(value))
                    .map(|value| (value * 100.0).round() as u16))
                .or(Some(100))
        });
        self.combined_resolution_pct = combined_resolution(
            self.global_resolution_pct, self.app_resolution_pct,
        );
    }

    pub fn display_resolution(&self, scene_active: bool) -> Option<u16> {
        if scene_active { self.combined_resolution_pct }
        else { self.global_resolution_pct }
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

fn combined_resolution(global: Option<u16>, app: Option<u16>) -> Option<u16> {
    let result = u32::from(global?) * u32::from(app?);
    u16::try_from((result + 50) / 100).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_percentages() {
        assert_eq!(combined_resolution(Some(150), Some(120)), Some(180));
        assert_eq!(combined_resolution(Some(100), Some(100)), Some(100));
        assert_eq!(combined_resolution(Some(120), None), None);
        assert_eq!(combined_resolution(None, Some(150)), None);
    }
}
