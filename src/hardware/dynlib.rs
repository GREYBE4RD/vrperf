//! Minimal optional shared-library loader. GPU vendor libraries are never mandatory.
use std::ffi::{CStr, c_void};

pub(super) struct DynLib(*mut c_void);

impl DynLib {
    pub(super) fn open(name: &CStr) -> Option<Self> {
        // SAFETY: C string is NUL-terminated and dlopen retains its own name.
        let ptr = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        (!ptr.is_null()).then_some(Self(ptr))
    }

    pub(super) unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Option<T> {
        // SAFETY: Callers must supply the exact C ABI signature for the symbol.
        // Function pointers are the same size as dlsym's void* on supported Linux targets.
        if std::mem::size_of::<T>() != std::mem::size_of::<*mut c_void>() { return None; }
        let ptr = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        if ptr.is_null() { None } else { Some(unsafe { std::mem::transmute_copy(&ptr) }) }
    }
}

impl Drop for DynLib {
    fn drop(&mut self) {
        // SAFETY: The handle was returned by dlopen and is owned by this struct.
        unsafe { libc::dlclose(self.0); }
    }
}
