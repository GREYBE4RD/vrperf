//! Read the scene renderer PID from the existing OpenVR compositor.
//! The safe Rust compositor wrapper does not currently expose this query.

use openvr_sys::VR_IVRCompositor_FnTable;
use std::time::{Duration, Instant};

pub struct SceneProbe {
    api: *const VR_IVRCompositor_FnTable,
    current: Option<u32>,
    candidate: Option<u32>,
    candidate_since: Instant,
}

impl SceneProbe {
    pub fn new() -> Result<Self, String> {
        let mut error = 0;
        // SAFETY: An OpenVR context has already been created in main. We use
        // this pointer only on the same thread and before context shutdown.
        let api = unsafe {
            openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVRCompositor_029".as_ptr(), &mut error,
            ) as *const VR_IVRCompositor_FnTable
        };
        if api.is_null() || unsafe { (*api).GetLastFrameRenderer.is_none() } {
            return Err(format!("OpenVR compositor scene API unavailable ({error})"));
        }
        Ok(Self {
            api,
            current: None,
            candidate: None,
            candidate_since: Instant::now(),
        })
    }

    /// Returns the new scene process only after a short steady period. This
    /// prevents brief compositor-only frames from clearing valid game graphs.
    pub fn poll(&mut self) -> Option<Option<u32>> {
        // SAFETY: The table was checked in `new`, and OpenVR is still alive.
        let raw = unsafe { (*self.api).GetLastFrameRenderer.unwrap()() };
        let observed = (raw != 0).then_some(raw);
        if observed == self.current {
            self.candidate = self.current;
            return None;
        }
        if observed != self.candidate {
            self.candidate = observed;
            self.candidate_since = Instant::now();
            return None;
        }
        if self.candidate_since.elapsed() < Duration::from_millis(350) {
            return None;
        }
        self.current = observed;
        Some(observed)
    }
}
