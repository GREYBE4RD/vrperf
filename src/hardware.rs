//! Linux CPU/RAM plus optional AMD, NVIDIA and Intel GPU monitoring.
//! No vendor library or external command is required for the program to run.
mod dynlib;
mod nvml;
mod level_zero;

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default, Clone, Copy)]
pub struct HardwareStats {
    pub cpu_load: Option<f32>,
    pub gpu_load: Option<f32>,
    pub vram_used_gib: Option<f32>,
    pub vram_total_gib: Option<f32>,
    pub ram_used_gib: Option<f32>,
    pub ram_total_gib: Option<f32>,
}

enum GpuMonitor {
    Amd(PathBuf),
    Nvidia(nvml::Nvml),
    Intel(level_zero::IntelSysman),
    Unavailable,
}

pub struct HardwareMonitor {
    gpu: GpuMonitor,
    previous_cpu: Option<(u64, u64)>,
    pub stats: HardwareStats,
}

impl HardwareMonitor {
    pub fn new() -> Self {
        Self {
            gpu: detect_gpu(),
            previous_cpu: None,
            stats: HardwareStats::default(),
        }
    }

    pub fn update(&mut self) {
        let (used, total) = system_ram();
        self.stats.ram_used_gib = used;
        self.stats.ram_total_gib = total;
        if let Some((total, idle)) = cpu_counters() {
            if let Some((prev_total, prev_idle)) = self.previous_cpu {
                let elapsed = total.saturating_sub(prev_total);
                let idle_elapsed = idle.saturating_sub(prev_idle);
                if elapsed > 0 {
                    self.stats.cpu_load = Some(100.0 * elapsed.saturating_sub(idle_elapsed) as f32 / elapsed as f32);
                }
            }
            self.previous_cpu = Some((total, idle));
        }

        // Clear unavailable/stale samples instead of retaining an old reading.
        let (gpu, used, total) = match &mut self.gpu {
            GpuMonitor::Amd(device) => {
                const GIB: f32 = 1024.0 * 1024.0 * 1024.0;
                (read_number(device.join("gpu_busy_percent")).filter(|n| *n <= 100).map(|n| n as f32),
                 read_number(device.join("mem_info_vram_used")).map(|n| n as f32 / GIB),
                 read_number(device.join("mem_info_vram_total")).map(|n| n as f32 / GIB))
            }
            GpuMonitor::Nvidia(backend) => backend.sample(),
            GpuMonitor::Intel(backend) => backend.sample(),
            GpuMonitor::Unavailable => (None, None, None),
        };
        self.stats.gpu_load = gpu;
        self.stats.vram_used_gib = used;
        self.stats.vram_total_gib = total;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Vendor { Amd, Nvidia, Intel }

fn identify_vendor(value: &str) -> Option<Vendor> {
    match value.trim().to_ascii_lowercase().as_str() {
        "0x1002" => Some(Vendor::Amd),
        "0x10de" => Some(Vendor::Nvidia),
        "0x8086" => Some(Vendor::Intel),
        _ => None,
    }
}

fn drm_card(name: &str) -> bool {
    let Some(digits) = name.strip_prefix("card") else { return false; };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn detect_gpu() -> GpuMonitor {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return GpuMonitor::Unavailable; };
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !drm_card(&name.to_string_lossy()) { continue; }
        let device = entry.path().join("device");
        let Some(vendor) = fs::read_to_string(device.join("vendor"))
            .ok().and_then(|v| identify_vendor(&v)) else { continue; };
        let vram = read_number(device.join("mem_info_vram_total")).unwrap_or(0);
        // Prefer discrete NVIDIA and AMD cards over integrated Intel graphics.
        let priority = match vendor { Vendor::Nvidia => 3, Vendor::Amd => 2, Vendor::Intel => 1 };
        candidates.push((priority, vram, vendor, device));
    }
    candidates.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    for (_, _, vendor, device) in candidates {
        match vendor {
            Vendor::Amd if device.join("gpu_busy_percent").exists() => return GpuMonitor::Amd(device),
            Vendor::Nvidia => {
                let pci = fs::canonicalize(&device).ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
                if let Some(monitor) = pci.as_deref().and_then(nvml::Nvml::new) {
                    return GpuMonitor::Nvidia(monitor);
                }
            }
            Vendor::Intel => {
                if let Some(monitor) = level_zero::IntelSysman::new() {
                    return GpuMonitor::Intel(monitor);
                }
            }
            _ => (),
        }
    }
    GpuMonitor::Unavailable
}

fn system_ram() -> (Option<f32>, Option<f32>) {
    let Ok(text) = fs::read_to_string("/proc/meminfo") else { return (None, None); };
    parse_meminfo(&text)
}

fn parse_meminfo(text: &str) -> (Option<f32>, Option<f32>) {
    let mut total_kib = None;
    let mut available_kib = None;
    for line in text.lines() {
        let (key, rest) = match line.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        let amount = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok());
        match key {
            "MemTotal" => total_kib = amount,
            "MemAvailable" => available_kib = amount,
            _ => (),
        }
    }
    const KIB_PER_GIB: f32 = 1024.0 * 1024.0;
    match (total_kib, available_kib) {
        (Some(total), Some(available)) if available <= total => (
            Some((total - available) as f32 / KIB_PER_GIB),
            Some(total as f32 / KIB_PER_GIB),
        ),
        _ => (None, None),
    }
}

fn cpu_counters() -> Option<(u64, u64)> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let mut parts = content.lines().next()?.split_whitespace();
    if parts.next()? != "cpu" { return None; }
    let fields: Vec<u64> = parts.map(str::parse::<u64>).collect::<Result<_, _>>().ok()?;
    if fields.len() < 4 { return None; }
    let total = fields.iter().copied().sum();
    // idle + iowait; consistent with aggregate Linux CPU utilisation displays.
    let idle = fields[3] + fields.get(4).copied().unwrap_or(0);
    Some((total, idle))
}

fn read_number(path: impl AsRef<Path>) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ram_accounts_for_reclaimable_memory() {
        let (used, total) = parse_meminfo(
            "MemTotal: 33554432 kB\nMemFree: 1048576 kB\nMemAvailable: 16777216 kB\n"
        );
        assert_eq!(total, Some(32.0));
        assert_eq!(used, Some(16.0));
    }

    #[test]
    fn sysfs_filters_only_drm_cards() {
        assert!(drm_card("card0"));
        assert!(drm_card("card12"));
        assert!(!drm_card("card"));
        assert!(!drm_card("card0-DP-1"));
        assert!(!drm_card("renderD128"));
    }

    #[test]
    fn vendor_detection_is_explicit() {
        assert_eq!(identify_vendor("0x1002\n"), Some(Vendor::Amd));
        assert_eq!(identify_vendor("0x10de"), Some(Vendor::Nvidia));
        assert_eq!(identify_vendor("0x8086"), Some(Vendor::Intel));
        assert_eq!(identify_vendor("0x1234"), None);
    }
}
