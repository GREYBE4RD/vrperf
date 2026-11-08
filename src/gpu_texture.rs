//! Optional persistent OpenGL texture upload for SteamVR overlays.
//!
//! Uses a tiny surfaceless EGL context; no window, render loop, shader or GUI
//! library is needed. The `openvr` safe wrapper only exposes SetOverlayRaw,
//! so this module calls SetOverlayTexture through the same SDK's C fn table.
//! Errors are returned to main; a raw-upload fallback remains available.

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use openvr::overlay::OverlayHandle;

const EGL_NONE: i32 = 0x3038;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_BIT: i32 = 0x0008;
const EGL_OPENGL_API: u32 = 0x30A2;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_PLATFORM_SURFACELESS_MESA: u32 = 0x31DD;
const EGL_PLATFORM_DEVICE_EXT: u32 = 0x313F;

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_RGBA8: i32 = 0x8058;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_LINEAR: i32 = 0x2601;
const GL_UNPACK_ALIGNMENT: u32 = 0x0CF5;
const GL_NO_ERROR: u32 = 0;

#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetPlatformDisplay(platform: u32, native_display: *mut c_void, attribs: *const isize) -> *mut c_void;
    fn eglGetDisplay(display: *mut c_void) -> *mut c_void;
    fn eglGetProcAddress(name: *const libc::c_char) -> *const c_void;
    fn eglInitialize(display: *mut c_void, major: *mut i32, minor: *mut i32) -> u32;
    fn eglChooseConfig(display: *mut c_void, attrs: *const i32, configs: *mut *mut c_void,
        config_size: i32, num_configs: *mut i32) -> u32;
    fn eglBindAPI(api: u32) -> u32;
    fn eglCreateContext(display: *mut c_void, config: *mut c_void, shared: *mut c_void,
        attrs: *const i32) -> *mut c_void;
    fn eglCreatePbufferSurface(display: *mut c_void, config: *mut c_void, attrs: *const i32) -> *mut c_void;
    fn eglMakeCurrent(display: *mut c_void, draw: *mut c_void, read: *mut c_void, context: *mut c_void) -> u32;
    fn eglDestroySurface(display: *mut c_void, surface: *mut c_void) -> u32;
    fn eglDestroyContext(display: *mut c_void, context: *mut c_void) -> u32;
    fn eglTerminate(display: *mut c_void) -> u32;
}

#[link(name = "GL")]
unsafe extern "C" {
    fn glGenTextures(n: i32, textures: *mut u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glTexParameteri(target: u32, pname: u32, param: i32);
    fn glPixelStorei(pname: u32, param: i32);
    fn glTexImage2D(target: u32, level: i32, internal_format: i32, width: i32, height: i32,
        border: i32, format: u32, pixel_type: u32, pixels: *const c_void);
    fn glTexSubImage2D(target: u32, level: i32, xoffset: i32, yoffset: i32, width: i32,
        height: i32, format: u32, pixel_type: u32, pixels: *const c_void);
    fn glDeleteTextures(n: i32, textures: *const u32);
    fn glGetError() -> u32;
    fn glFlush();
}


/// Last-resort driver-neutral EGL platform-device search. No change to the
/// existing successful Mesa surfaceless path.
unsafe fn try_egl_devices(major: &mut i32, minor: &mut i32) -> *mut c_void {
    type QueryDevices = unsafe extern "C" fn(i32, *mut *mut c_void, *mut i32) -> u32;
    type GetPlatformDisplay = unsafe extern "C" fn(u32, *mut c_void, *const isize) -> *mut c_void;
    // SAFETY: Function signatures are those published by EGL_EXT_device_enumeration
    // and EGL_EXT_platform_base. Both are resolved at runtime if supported.
    unsafe {
        let query = eglGetProcAddress(c"eglQueryDevicesEXT".as_ptr());
        let platform = eglGetProcAddress(c"eglGetPlatformDisplayEXT".as_ptr());
        if query.is_null() || platform.is_null() { return null_mut(); }
        let query: QueryDevices = std::mem::transmute(query);
        let platform: GetPlatformDisplay = std::mem::transmute(platform);
        let mut count = 0;
        if query(0, null_mut(), &mut count) == 0 || count <= 0 || count > 32 {
            return null_mut();
        }
        let mut devices = vec![null_mut(); count as usize];
        if query(count, devices.as_mut_ptr(), &mut count) == 0 { return null_mut(); }
        for device in devices {
            if device.is_null() { continue; }
            let display = platform(EGL_PLATFORM_DEVICE_EXT, device, null());
            if !display.is_null() && eglInitialize(display, major, minor) != 0 {
                return display;
            }
        }
        null_mut()
    }
}

pub struct GlUploader {
    display: *mut c_void,
    surface: *mut c_void,
    context: *mut c_void,
    texture: u32,
    width: i32,
    height: i32,
    overlay_api: *const openvr_sys::VR_IVROverlay_FnTable,
}

impl GlUploader {
    pub fn new(width: usize, height: usize) -> Result<Self, String> {
        // SAFETY: The EGL context is owned by this struct and used on one thread only.
        unsafe {
            let mut display = eglGetPlatformDisplay(EGL_PLATFORM_SURFACELESS_MESA, null_mut(), null());
            let mut major = 0;
            let mut minor = 0;
            if display.is_null() || eglInitialize(display, &mut major, &mut minor) == 0 {
                display = eglGetDisplay(null_mut());
                if display.is_null() || eglInitialize(display, &mut major, &mut minor) == 0 {
                    // EGL_EXT_platform_device can provide a headless context on
                    // drivers without EGL_MESA_platform_surfaceless (notably
                    // some proprietary NVIDIA EGL configurations).
                    display = try_egl_devices(&mut major, &mut minor);
                    if display.is_null() {
                        return Err("EGL could not initialise an OpenGL display".into());
                    }
                }
            }
            let attrs = [EGL_SURFACE_TYPE, EGL_PBUFFER_BIT, EGL_RENDERABLE_TYPE, EGL_OPENGL_BIT, EGL_NONE];
            let mut config = null_mut();
            let mut configs = 0;
            if eglChooseConfig(display, attrs.as_ptr(), &mut config, 1, &mut configs) == 0 || configs == 0
                || eglBindAPI(EGL_OPENGL_API) == 0 {
                let _ = eglTerminate(display);
                return Err("EGL has no usable OpenGL pbuffer configuration".into());
            }
            let context = eglCreateContext(display, config, null_mut(), [EGL_NONE].as_ptr());
            let surface = eglCreatePbufferSurface(display, config, [EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE].as_ptr());
            if context.is_null() || surface.is_null() || eglMakeCurrent(display, surface, surface, context) == 0 {
                if !surface.is_null() { let _ = eglDestroySurface(display, surface); }
                if !context.is_null() { let _ = eglDestroyContext(display, context); }
                let _ = eglTerminate(display);
                return Err("EGL could not create an OpenGL context".into());
            }
            let mut texture = 0;
            glGenTextures(1, &mut texture);
            glBindTexture(GL_TEXTURE_2D, texture);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
            glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA8, width as i32, height as i32,
                0, GL_RGBA, GL_UNSIGNED_BYTE, null());
            if texture == 0 || glGetError() != GL_NO_ERROR {
                if texture != 0 { glDeleteTextures(1, &texture); }
                let _ = eglMakeCurrent(display, null_mut(), null_mut(), null_mut());
                let _ = eglDestroySurface(display, surface);
                let _ = eglDestroyContext(display, context);
                let _ = eglTerminate(display);
                return Err("OpenGL texture creation failed".into());
            }
            // The high-level crate has no SetOverlayTexture method; request the
            // matching function table from the already-initialised OpenVR runtime.
            let mut init_error = 0;
            let api = openvr_sys::VR_GetGenericInterface(
                c"FnTable:IVROverlay_028".as_ptr(), &mut init_error,
            ) as *const openvr_sys::VR_IVROverlay_FnTable;
            if api.is_null() || (*api).SetOverlayTexture.is_none()
                || (*api).SetOverlayTextureBounds.is_none() {
                glDeleteTextures(1, &texture);
                let _ = eglMakeCurrent(display, null_mut(), null_mut(), null_mut());
                let _ = eglDestroySurface(display, surface);
                let _ = eglDestroyContext(display, context);
                let _ = eglTerminate(display);
                return Err(format!("OpenVR texture API unavailable ({init_error})"));
            }
            Ok(Self { display, surface, context, texture, width: width as i32,
                height: height as i32, overlay_api: api })
        }
    }

    /// Flip only the OpenGL texture's vertical sampling direction. The UI pixel
    /// buffer uses a top-left origin, but OpenGL texture rows use a bottom-left
    /// origin. Reverse this setting before switching to SetOverlayRaw.
    pub fn set_vertical_flip(&self, handle: OverlayHandle, flipped: bool) -> Result<(), String> {
        let mut bounds = openvr_sys::VRTextureBounds_t {
            uMin: 0.0,
            vMin: if flipped { 1.0 } else { 0.0 },
            uMax: 1.0,
            vMax: if flipped { 0.0 } else { 1.0 },
        };
        // SAFETY: The OpenVR interface was checked during construction and
        // remains valid while the OpenVR context is alive on this thread.
        let result = unsafe {
            let set_bounds = (*self.overlay_api).SetOverlayTextureBounds.expect("validated API");
            set_bounds(handle.0, &mut bounds)
        };
        if result != 0 {
            return Err(format!("OpenVR SetOverlayTextureBounds failed: {result}"));
        }
        Ok(())
    }

    pub fn upload(&mut self, handle: OverlayHandle, pixels: &[u8]) -> Result<(), String> {
        if pixels.len() != self.width as usize * self.height as usize * 4 {
            return Err("Unexpected frame buffer size".into());
        }
        // SAFETY: The context remains current on this single thread, texture is
        // alive, `pixels` has the required size, and the OpenVR table outlives us.
        unsafe {
            glBindTexture(GL_TEXTURE_2D, self.texture);
            glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, self.width, self.height,
                GL_RGBA, GL_UNSIGNED_BYTE, pixels.as_ptr().cast());
            if glGetError() != GL_NO_ERROR { return Err("OpenGL texture upload failed".into()); }
            glFlush();
            let mut texture = openvr_sys::Texture_t {
                handle: self.texture as usize as *mut c_void,
                eType: openvr_sys::ETextureType_TextureType_OpenGL,
                eColorSpace: openvr_sys::EColorSpace_ColorSpace_Auto,
            };
            let upload = (*self.overlay_api).SetOverlayTexture.expect("validated API");
            let result = upload(handle.0, &mut texture);
            if result != 0 { return Err(format!("OpenVR SetOverlayTexture failed: {result}")); }
        }
        Ok(())
    }
}

impl Drop for GlUploader {
    fn drop(&mut self) {
        // SAFETY: Owned EGL/GL resources; created and freed on the same thread.
        unsafe {
            glDeleteTextures(1, &self.texture);
            let _ = eglMakeCurrent(self.display, null_mut(), null_mut(), null_mut());
            let _ = eglDestroySurface(self.display, self.surface);
            let _ = eglDestroyContext(self.display, self.context);
            let _ = eglTerminate(self.display);
        }
    }
}
