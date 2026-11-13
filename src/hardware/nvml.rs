//! Best-effort NVIDIA monitoring with NVML. No subprocesses or link-time dependency.
use super::dynlib::DynLib;
use std::ffi::{CString, c_void};
use std::ptr::null_mut;

type Device = *mut c_void;
type Return = i32;
type Init = unsafe extern "C" fn() -> Return;
type Shutdown = unsafe extern "C" fn() -> Return;
type ByPci = unsafe extern "C" fn(*const libc::c_char, *mut Device) -> Return;
type Util = unsafe extern "C" fn(Device, *mut Utilization) -> Return;
type Memory = unsafe extern "C" fn(Device, *mut MemoryInfo) -> Return;
type Temperature = unsafe extern "C" fn(Device, u32, *mut u32) -> Return;

#[repr(C)]
#[derive(Default)]
struct Utilization { gpu: u32, memory: u32 }
#[repr(C)]
#[derive(Default)]
struct MemoryInfo { total: u64, free: u64, used: u64 }

pub(super) struct Nvml {
    // Keep the library open for the lifetime of the function pointers.
    _library: DynLib,
    device: Device,
    shutdown: Shutdown,
    utilization: Util,
    memory: Memory,
    temperature: Option<Temperature>,
}

impl Nvml {
    pub(super) fn new(pci_address: &str) -> Option<Self> {
        let library = DynLib::open(c"libnvidia-ml.so.1")?;
        // SAFETY: Types are copied from the documented NVML C API.
        unsafe {
            let init: Init = library.symbol(c"nvmlInit_v2")?;
            let shutdown: Shutdown = library.symbol(c"nvmlShutdown")?;
            let by_pci: ByPci = library.symbol(c"nvmlDeviceGetHandleByPciBusId_v2")?;
            let utilization: Util = library.symbol(c"nvmlDeviceGetUtilizationRates")?;
            let memory: Memory = library.symbol(c"nvmlDeviceGetMemoryInfo")?;
            // Older NVML GPU temperature API. Optional: monitoring must still
            // work on devices/drivers which do not expose this entry point.
            let temperature: Option<Temperature> = library.symbol(c"nvmlDeviceGetTemperature");
            if init() != 0 { return None; }
            let Some(address) = CString::new(pci_address).ok() else {
                let _ = shutdown();
                return None;
            };
            let mut device = null_mut();
            if by_pci(address.as_ptr(), &mut device) != 0 || device.is_null() {
                let _ = shutdown();
                return None;
            }
            Some(Self { _library: library, device, shutdown, utilization, memory, temperature })
        }
    }

    pub(super) fn temperature(&self) -> Option<f32> {
        let get = self.temperature?;
        let mut degrees = 0_u32;
        // NVML_TEMPERATURE_GPU = 0, in whole degrees Celsius.
        let ok = unsafe { get(self.device, 0, &mut degrees) } == 0;
        (ok && degrees <= 200).then_some(degrees as f32)
    }

    pub(super) fn sample(&self) -> (Option<f32>, Option<f32>, Option<f32>) {
        let mut util = Utilization::default();
        let mut mem = MemoryInfo::default();
        // SAFETY: The device and symbols remain valid until shutdown in Drop.
        let busy = unsafe { (self.utilization)(self.device, &mut util) == 0 }
            .then(|| util.gpu.min(100) as f32);
        let ok = unsafe { (self.memory)(self.device, &mut mem) == 0 } && mem.total > 0
            && mem.used <= mem.total;
        if !ok { return (busy, None, None); }
        const GIB: f32 = 1024.0 * 1024.0 * 1024.0;
        (busy, Some(mem.used as f32 / GIB), Some(mem.total as f32 / GIB))
    }
}

impl Drop for Nvml {
    fn drop(&mut self) {
        // SAFETY: nvmlShutdown pairs with nvmlInit_v2 while the library is loaded.
        unsafe { (self.shutdown)(); }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn documented_nvml_struct_layout() {
        assert_eq!(std::mem::size_of::<super::Utilization>(), 8);
        assert_eq!(std::mem::size_of::<super::MemoryInfo>(), 24);
    }
}
