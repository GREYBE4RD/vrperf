//! Experimental Intel monitoring via optional oneAPI Level Zero Sysman.
//! Only the combined 3D/render/compute engine is used for GPU load. Dedicated
//! device memory is reported; shared system RAM is never labelled as VRAM.
use super::dynlib::DynLib;
use std::ffi::c_void;
use std::ptr::null_mut;

type Handle = *mut c_void;
type ResultCode = u32;
type Init = unsafe extern "C" fn(u32) -> ResultCode;
type DriverGet = unsafe extern "C" fn(*mut u32, *mut Handle) -> ResultCode;
type DeviceGet = unsafe extern "C" fn(Handle, *mut u32, *mut Handle) -> ResultCode;
type EnumComponents = unsafe extern "C" fn(Handle, *mut u32, *mut Handle) -> ResultCode;
type EnginePropsFn = unsafe extern "C" fn(Handle, *mut EngineProps) -> ResultCode;
type EngineActivity = unsafe extern "C" fn(Handle, *mut EngineStats) -> ResultCode;
type MemoryPropsFn = unsafe extern "C" fn(Handle, *mut MemoryProps) -> ResultCode;
type MemoryStateFn = unsafe extern "C" fn(Handle, *mut MemoryState) -> ResultCode;
type TempPropsFn = unsafe extern "C" fn(Handle, *mut TempProps) -> ResultCode;
type TempStateFn = unsafe extern "C" fn(Handle, *mut f64) -> ResultCode;

// Layout and constant values follow the Level Zero Sysman C headers.
#[repr(C)]
struct EngineProps {
    stype: u32,
    p_next: *mut c_void,
    group: u32,
    on_subdevice: u32,
    subdevice_id: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct EngineStats { active_time: u64, timestamp: u64 }
#[repr(C)]
struct MemoryProps {
    stype: u32,
    p_next: *mut c_void,
    mem_type: u32,
    on_subdevice: u32,
    subdevice_id: u32,
    location: u32,
    physical_size: u64,
    bus_width: i32,
    channels: i32,
}
#[repr(C)]
struct MemoryState {
    stype: u32,
    p_next: *mut c_void,
    health: u32,
    free: u64,
    size: u64,
}
#[repr(C)]
struct TempProps {
    stype: u32,
    p_next: *mut c_void,
    sensor_type: u32,
    on_subdevice: u32,
    subdevice_id: u32,
    max_temperature: f64,
    is_critical_supported: u32,
    is_threshold1_supported: u32,
    is_threshold2_supported: u32,
}

const TEMP_PROPERTIES: u32 = 0x14;
const TEMP_GPU: u32 = 1;
const TEMP_GLOBAL: u32 = 0;
const ENGINE_PROPERTIES: u32 = 0x5;
const MEMORY_PROPERTIES: u32 = 0xb;
const MEMORY_STATE: u32 = 0x1e;
const ENGINE_3D_RENDER_COMPUTE_ALL: u32 = 11;
const ENGINE_RENDER_ALL: u32 = 12;
const MEMORY_LOCATION_DEVICE: u32 = 1;

pub(super) struct IntelSysman {
    _library: DynLib,
    engine: Option<Handle>,
    last_engine: Option<EngineStats>,
    engine_activity: EngineActivity,
    memories: Vec<Handle>,
    memory_state: MemoryStateFn,
    temperature: Option<Handle>,
    temperature_state: Option<TempStateFn>,
}

impl IntelSysman {
    pub(super) fn new() -> Option<Self> {
        let library = DynLib::open(c"libze_loader.so.1")?;
        // SAFETY: Symbol signatures/structs follow the public C Sysman API.
        unsafe {
            let init: Init = library.symbol(c"zesInit")?;
            let drivers: DriverGet = library.symbol(c"zesDriverGet")?;
            let devices: DeviceGet = library.symbol(c"zesDeviceGet")?;
            let enum_engines: EnumComponents = library.symbol(c"zesDeviceEnumEngineGroups")?;
            let engine_props: EnginePropsFn = library.symbol(c"zesEngineGetProperties")?;
            let engine_activity: EngineActivity = library.symbol(c"zesEngineGetActivity")?;
            let enum_memories: EnumComponents = library.symbol(c"zesDeviceEnumMemoryModules")?;
            let memory_props: MemoryPropsFn = library.symbol(c"zesMemoryGetProperties")?;
            let memory_state: MemoryStateFn = library.symbol(c"zesMemoryGetState")?;
            let enum_temperatures: Option<EnumComponents> = library.symbol(c"zesDeviceEnumTemperatureSensors");
            let temperature_props: Option<TempPropsFn> = library.symbol(c"zesTemperatureGetProperties");
            let temperature_state: Option<TempStateFn> = library.symbol(c"zesTemperatureGetState");

            if init(0) != 0 { return None; }
            let mut driver_count = 0;
            if drivers(&mut driver_count, null_mut()) != 0 || driver_count == 0 || driver_count > 32 {
                return None;
            }
            let mut driver_handles = vec![null_mut(); driver_count as usize];
            if drivers(&mut driver_count, driver_handles.as_mut_ptr()) != 0 { return None; }
            // Until PCI matching is implemented, use the first Sysman Intel device.
            let mut selected = null_mut();
            for driver in driver_handles {
                let mut count = 0;
                if devices(driver, &mut count, null_mut()) != 0 || count == 0 || count > 32 {
                    continue;
                }
                let mut handles = vec![null_mut(); count as usize];
                if devices(driver, &mut count, handles.as_mut_ptr()) == 0 {
                    if let Some(&device) = handles.iter().find(|device| !device.is_null()) {
                        selected = device;
                        break;
                    }
                }
            }
            if selected.is_null() { return None; }

            let mut engine = None;
            let mut count = 0;
            if enum_engines(selected, &mut count, null_mut()) == 0 && (1..=128).contains(&count) {
                let mut handles = vec![null_mut(); count as usize];
                if enum_engines(selected, &mut count, handles.as_mut_ptr()) == 0 {
                    let mut best = 0;
                    for handle in handles.into_iter().filter(|h| !h.is_null()) {
                        let mut props = EngineProps {
                            stype: ENGINE_PROPERTIES, p_next: null_mut(), group: 0,
                            on_subdevice: 0, subdevice_id: 0,
                        };
                        if engine_props(handle, &mut props) != 0 { continue; }
                        let priority = match props.group {
                            ENGINE_3D_RENDER_COMPUTE_ALL => 3,
                            ENGINE_RENDER_ALL => 2,
                            5 | 10 => 1, // render or 3D single when aggregates unavailable
                            _ => 0,
                        };
                        if priority > best {
                            best = priority;
                            engine = Some(handle);
                        }
                    }
                }
            }

            let mut memories = Vec::new();
            let mut count = 0;
            if enum_memories(selected, &mut count, null_mut()) == 0 && (1..=128).contains(&count) {
                let mut handles = vec![null_mut(); count as usize];
                if enum_memories(selected, &mut count, handles.as_mut_ptr()) == 0 {
                    for handle in handles.into_iter().filter(|h| !h.is_null()) {
                        let mut props = MemoryProps {
                            stype: MEMORY_PROPERTIES, p_next: null_mut(), mem_type: 0,
                            on_subdevice: 0, subdevice_id: 0, location: 0,
                            physical_size: 0, bus_width: 0, channels: 0,
                        };
                        if memory_props(handle, &mut props) == 0 && props.location == MEMORY_LOCATION_DEVICE {
                            memories.push(handle);
                        }
                    }
                }
            }
            // Prefer the GPU sensor, otherwise the global device sensor.
            // Sysman temperature APIs are optional on some Intel drivers.
            let mut temperature = None;
            if let (Some(enumerate), Some(properties), Some(_)) =
                (enum_temperatures, temperature_props, temperature_state) {
                let mut count = 0;
                if enumerate(selected, &mut count, null_mut()) == 0 && (1..=128).contains(&count) {
                    let mut handles = vec![null_mut(); count as usize];
                    if enumerate(selected, &mut count, handles.as_mut_ptr()) == 0 {
                        let mut best = 0;
                        for handle in handles.into_iter().filter(|h| !h.is_null()) {
                            let mut props = TempProps {
                                stype: TEMP_PROPERTIES, p_next: null_mut(), sensor_type: 0,
                                on_subdevice: 0, subdevice_id: 0, max_temperature: 0.0,
                                is_critical_supported: 0, is_threshold1_supported: 0,
                                is_threshold2_supported: 0,
                            };
                            if properties(handle, &mut props) != 0 { continue; }
                            let priority = match props.sensor_type {
                                TEMP_GPU => 2, TEMP_GLOBAL => 1, _ => 0,
                            };
                            if priority > best { best = priority; temperature = Some(handle); }
                        }
                    }
                }
            }
            Some(Self { _library: library, engine, last_engine: None,
                engine_activity, memories, memory_state, temperature, temperature_state })
        }
    }

    pub(super) fn temperature(&self) -> Option<f32> {
        let (Some(handle), Some(get)) = (self.temperature, self.temperature_state) else { return None; };
        let mut value = 0.0_f64;
        let ok = unsafe { get(handle, &mut value) } == 0;
        (ok && value.is_finite() && (0.0..=200.0).contains(&value)).then_some(value as f32)
    }

    pub(super) fn sample(&mut self) -> (Option<f32>, Option<f32>, Option<f32>) {
        let mut busy = None;
        if let Some(handle) = self.engine {
            let mut current = EngineStats::default();
            // SAFETY: Valid Sysman handle while loader remains open.
            if unsafe { (self.engine_activity)(handle, &mut current) } == 0 {
                if let Some(previous) = self.last_engine {
                    busy = engine_percent(previous, current);
                }
                self.last_engine = Some(current);
            } else {
                self.last_engine = None;
            }
        }
        let mut used = 0_u64;
        let mut total = 0_u64;
        for &handle in &self.memories {
            let mut state = MemoryState { stype: MEMORY_STATE, p_next: null_mut(),
                health: 0, free: 0, size: 0 };
            // SAFETY: Memory module handle is owned by the selected Sysman device.
            if unsafe { (self.memory_state)(handle, &mut state) } == 0 && state.size > 0
                && state.free <= state.size {
                used = used.saturating_add(state.size - state.free);
                total = total.saturating_add(state.size);
            }
        }
        if total == 0 { return (busy, None, None); }
        const GIB: f32 = 1024.0 * 1024.0 * 1024.0;
        (busy, Some(used as f32 / GIB), Some(total as f32 / GIB))
    }
}

fn engine_percent(before: EngineStats, after: EngineStats) -> Option<f32> {
    let elapsed = after.timestamp.checked_sub(before.timestamp)?;
    let active = after.active_time.checked_sub(before.active_time)?;
    if elapsed == 0 || active > elapsed { return None; }
    Some(100.0 * (active as f32 / elapsed as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_activity_uses_counter_differences() {
        assert!(engine_percent(EngineStats { active_time: 50, timestamp: 100 },
            EngineStats { active_time: 80, timestamp: 200 }).is_some_and(|v| (v - 30.0).abs() < 0.01));
        assert_eq!(engine_percent(EngineStats { active_time: 0, timestamp: 100 },
            EngineStats { active_time: 110, timestamp: 200 }), None);
    }
    #[test]
    fn sysman_abi_struct_sizes() {
        assert_eq!(std::mem::size_of::<EngineStats>(), 16);
        assert_eq!(std::mem::size_of::<EngineProps>(), 32);
        assert_eq!(std::mem::size_of::<MemoryProps>(), 48);
        assert_eq!(std::mem::size_of::<MemoryState>(), 40);
        assert_eq!(std::mem::size_of::<TempProps>(), 56);
    }
}
