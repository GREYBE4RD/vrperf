# Hardware support and sensor limitations

[Back to README](../README.md#what-the-display-means)

vrperf gets frame timing and headset/controller details from SteamVR/OpenVR. It reads host CPU, memory, GPU use and temperature through Linux and optional vendor interfaces. Some readings are not available on every device or driver.

## Tested hardware

| Hardware | Status |
| --- | --- |
| Arch Linux, SteamVR, Steam Frame, AMD Radeon RX 9070 XT | Tested development setup |
| Other SteamVR headsets and AMD GPUs | Best effort; not verified in this project |
| NVIDIA proprietary drivers | Experimental GPU monitoring, not verified in this project |
| Intel GPUs | Experimental GPU monitoring, not verified in this project |

Controller binding compatibility is documented in the [calibration guide](calibration.md#controller-support-and-bindings).

## CPU temperature

- **AMD CPUs:** vrperf reads Linux `k10temp` or `zenpower` hardware-monitor entries. Where present, it prefers `Tdie`, then `Tctl`.
- **Intel CPUs:** it looks for the `coretemp` package temperature.
- `Tctl` is a thermal-control reading and can differ from the die temperature.

Readings depend on kernel modules, platform firmware, sensor names and access permissions. A missing or unreadable sensor appears as `--°`.

## GPU utilisation and temperature

- **AMD GPUs:** utilisation and VRAM monitoring use the selected DRM device under `/sys/class/drm` and its `amdgpu` sysfs data. Temperature is read from the corresponding `hwmon` sensor when available.
- **NVIDIA GPUs:** best-effort monitoring uses optional NVIDIA NVML (`libnvidia-ml.so.1`), including temperature when exposed.
- **Intel GPUs:** best-effort monitoring uses optional Level Zero Sysman (`libze_loader.so.1`). GPU temperature can also come from a corresponding Linux `hwmon` sensor when exposed.

**There is no general fallback to DRM readings for missing NVIDIA NVML or Intel Level Zero GPU counters.** If the required vendor interface cannot be loaded and no other supported device is selected, those metrics are unavailable. Vendor libraries are not required for vrperf to run.

Intel Sysman currently selects the first GPU exposed by its runtime, so monitoring may not identify the intended device when several Intel GPUs are present.

## Temperature and compact display limits

- Temperature appears as whole degrees Celsius, capped visually at **99°** in HEAD and watch modes.
- The compact watch shows at most **99%** for BAT, CPU and GPU. A true 100% therefore reads as 99% on the watch.
- Warning colours use the uncapped values. Temperature is orange at 80°C and red at 90°C.
- Unsupported or invalid temperature readings show `--°`. These limitations are indicators, not hardware safety checks.

## References

- [Linux hardware-monitoring sysfs interface](https://docs.kernel.org/hwmon/sysfs-interface.html)
- [AMD amdgpu driver details](https://www.kernel.org/doc/html/latest/gpu/amdgpu/driver-misc.html)
- [NVIDIA NVML](https://docs.nvidia.com/deploy/nvml-api/latest/index.html)
- [Intel Level Zero Sysman](https://oneapi-src.github.io/level-zero-spec/)

For missing values, see [troubleshooting](troubleshooting.md#missing-hardware-readings).
