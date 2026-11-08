use crate::config::{Config, Mode};
use openvr::TrackedDeviceIndex;

pub struct Positioner {
    current: Option<TrackedDeviceIndex>,
    visible: bool,
}

impl Positioner {
    pub fn new() -> Self {
        Self { current: None, visible: false }
    }

    pub fn update(
        &mut self,
        system: &openvr::System,
        overlay: &mut openvr::Overlay,
        handle: openvr::overlay::OverlayHandle,
        config: &Config,
        enabled: bool,
    ) -> Result<(), String> {
        if !enabled {
            if self.visible {
                overlay.set_visibility(handle, false)
                    .map_err(|e| format!("Cannot hide overlay: {e:?}"))?;
                self.visible = false;
            }
            return Ok(());
        }
        let target = match config.mode {
            Mode::Head => Some(openvr::tracked_device_index::HMD),
            Mode::Left => system.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::LeftHand),
            Mode::Right => system.tracked_device_index_for_controller_role(openvr::TrackedControllerRole::RightHand),
        };
        match target {
            Some(device) => {
                if self.current != Some(device) {
                    let [x, y, z] = config.position;
                    // Aim the HMD-fixed panel upward toward the eyes when it sits
                    // well below the centre of view. Controller mode stays unrotated.
                    let pitch = if config.mode == Mode::Head { y.atan2(-z) } else { 0.0 };
                    let (sin, cos) = pitch.sin_cos();
                    let transform = openvr::pose::Matrix3x4([
                        [1.0, 0.0, 0.0, x],
                        [0.0, cos, -sin, y],
                        [0.0, sin, cos, z],
                    ]);
                    overlay.set_transform_tracked_device_relative(handle, device, &transform)
                        .map_err(|e| format!("Overlay positioning failed: {e:?}"))?;
                    self.current = Some(device);
                }
                if !self.visible {
                    overlay.set_visibility(handle, true)
                        .map_err(|e| format!("Cannot show overlay: {e:?}"))?;
                    self.visible = true;
                }
            }
            None => {
                // Hide rather than leave a controller overlay floating when a controller disconnects.
                if self.visible {
                    overlay.set_visibility(handle, false)
                        .map_err(|e| format!("Cannot hide overlay: {e:?}"))?;
                    self.visible = false;
                }
                self.current = None;
            }
        }
        Ok(())
    }
}
