//! Read the scene renderer PID from the existing OpenVR compositor.
//! The safe Rust compositor wrapper does not currently expose this query.

use openvr_sys::{VR_IVRCompositor_FnTable, VR_IVRApplications_FnTable};
use std::ffi::CStr;
use std::time::{Duration, Instant};

pub struct SceneProbe {
    api: *const VR_IVRCompositor_FnTable,
    applications: Option<*const VR_IVRApplications_FnTable>,
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
        let mut application_error = 0;
        let applications = unsafe {
            openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVRApplications_008".as_ptr(), &mut application_error,
            ) as *const VR_IVRApplications_FnTable
        };
        let applications = (!applications.is_null()
            && unsafe { (*applications).GetApplicationKeyByProcessId.is_some() })
            .then_some(applications);
        Ok(Self {
            api,
            applications,
            current: None,
            candidate: None,
            candidate_since: Instant::now(),
        })
    }

    /// Looks up the SteamVR application key for the current scene renderer.
    /// A missing application mapping is not treated as a 100% per-app override.
    pub fn application_key(&self) -> Option<String> {
        let pid = self.current?;
        let api = self.applications?;
        let mut key = [0_i8; 256];
        // SAFETY: The OpenVR interface lives within the active context, the
        // function pointer was checked in new(), and our output is writable.
        let get_key = unsafe { (*api).GetApplicationKeyByProcessId }?;
        let status = unsafe { get_key(pid, key.as_mut_ptr(), key.len() as u32) };
        if status != 0 { return None; }
        // SAFETY: The fixed buffer is zero-initialised and only used for
        // the duration of this call. If no NUL exists, return unknown.
        if !key.contains(&0) { return None; }
        let result = unsafe { CStr::from_ptr(key.as_ptr()) };
        result.to_str().ok().filter(|text| !text.is_empty()).map(str::to_string)
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
