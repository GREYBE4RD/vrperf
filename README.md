# vrperf

A lightweight, native Linux performance overlay for SteamVR, written in Rust. It displays **PC-side** frame timings and system statistics inside the headset while streaming SteamVR games, including to the Steam Frame.

No MangoHud, fpsVR, WayVR, WiVRn, Python, background daemon, or desktop window is required.

![vrperf performance overlay](screenshots/vrperf.jpg)

## Features

- CPU and GPU application frame-time graphs, with a reference line for the current refresh-rate frame budget.
- FPS and dropped-frame markers in the graph history.
- Linux CPU and GPU utilisation, system RAM and graphics memory usage (where supported).
- Headset and controller battery percentages, SteamVR global resolution setting and local time.
- Compact, head-fixed overlay near the lower centre of the headset view.
- A **vrperf** tab in the SteamVR dashboard to show or hide the overlay.
- Starts automatically with SteamVR after installation. The visibility setting is saved.
- Single native Rust executable. Optional vendor monitoring libraries load only if needed.

## Compatibility and support status

| Component | Status |
| --- | --- |
| Linux SteamVR + Steam Frame | Tested on Arch Linux |
| AMD Radeon RX 9070 XT / RADV | **Tested, stable** |
| Other AMD GPUs | Best effort, untested |
| NVIDIA proprietary driver | Experimental, untested |
| Intel `i915` / `xe` | Experimental, untested |
| Other headsets and GPU configurations | Untested |

The compositor graphs and headset data use **OpenVR**, not an AMD-specific interface. Hardware utilisation has separate, optional backends:

- **AMD:** Linux `amdgpu` sysfs interfaces. No vendor library required.
- **NVIDIA:** NVML (`libnvidia-ml.so.1`) from the proprietary NVIDIA driver. No `nvidia-smi` subprocess. The device is matched by PCI address.
- **Intel:** Level Zero Sysman (`libze_loader.so.1` with a compatible Intel driver). The first Intel Sysman device is used; on systems with multiple Intel GPUs, device selection may be incorrect. Only dedicated, device-local memory is displayed as VRAM.

On unsupported hardware or if a library or reading is unavailable, the affected metric displays `--`. **VR frame timings should still work** if the underlying Linux SteamVR setup supports the game and OpenVR timing interface. NVIDIA and Intel overlays have not yet been tested on real hardware.

If both AMD and NVIDIA GPUs are present, the initial vendor selection prefers NVIDIA, then AMD, then Intel. This may not match the GPU SteamVR uses on hybrid systems; multi-GPU selection needs further work.

## Prerequisites

- x86-64 Linux and a working SteamVR installation.
- A headset connected through SteamVR, such as the Steam Frame via Steam Link.
- Working Linux OpenGL/EGL driver support (`libEGL.so.1` and `libGL.so.1`).
- **To build:** Rust stable (edition 2024), Cargo, a C/C++ compiler, CMake and Clang/libclang for the OpenVR bindings.
- Internet access for Cargo's first dependency download.

### Arch Linux

```bash
sudo pacman -S --needed rust base-devel cmake clang libglvnd
```

This is the development distribution used for the tested AMD build.

### Ubuntu / Debian (best effort, untested)

```bash
sudo apt install cargo rustc build-essential cmake clang libclang-dev libegl-dev libgl-dev
```

Use a current **stable Rust toolchain** that supports edition 2024. Distribution packages may be too old; in that case install Rust using [rustup](https://rustup.rs/).

### Fedora (best effort, untested)

```bash
sudo dnf install rust cargo gcc-c++ cmake clang-devel libglvnd-devel
```

NVIDIA GPU utilisation additionally requires NVIDIA's NVML runtime library, normally installed with the proprietary driver. Intel hardware monitoring requires the Level Zero loader and a compatible Intel compute runtime. **Neither library is required just to launch vrperf.**

## Build from source

From the repository directory:

```bash
cargo build --release
```

Run the tests:

```bash
cargo test
```

The executable is built at `target/release/vrperf`. Start SteamVR and connect the headset before testing it:

```bash
./target/release/vrperf
```

Stop a manually launched instance with `Ctrl+C` before installing or testing autostart.

Cargo generates `Cargo.lock` during the first build. Commit it to the GitHub repository after building locally so future release builds can use the same dependencies. The current source archive does not include a lockfile.

## Install and enable SteamVR autostart

With **SteamVR already running** and no manually running vrperf instance, run:

```bash
./target/release/vrperf --install
```

This installs the executable at **`~/.local/bin/vrperf`**, writes the SteamVR manifest under `~/.local/share/vrperf/`, and registers the `vrperf.linux.dashboard` overlay for native SteamVR startup. It does **not** install a systemd service.

Restart SteamVR to test automatic launch. Open the SteamVR dashboard and select **vrperf** to turn the performance panel on or off.

### `PATH` and the `vrperf` terminal command

`~/.local/bin` is commonly used for per-user executables, but some Linux distributions **do not add it to `PATH` automatically**. Check with:

```bash
printf '%s\n' "$PATH" | tr ':' '\n'
```

If `/home/YOUR_USER/.local/bin` is absent, add this line to your login shell configuration (for Bash, commonly `~/.profile`):

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Log out and back in, or source the updated file to apply it. Verify using:

```bash
command -v vrperf
```

The expected result is `$HOME/.local/bin/vrperf`. SteamVR autostart uses the absolute installed path and **does not require `~/.local/bin` in your shell's `PATH`**.

To update an existing installation, build the new release and run `./target/release/vrperf --install` again with SteamVR running. Restart SteamVR to load the new executable.

## Prebuilt binary (GitHub Releases)

For a tagged release, the included GitHub Actions workflow is set up to build an x86-64 Linux binary archive called `vrperf-v1.11.0-linux-x86_64.tar.gz`. Download it from the repository's **Releases** page after that workflow succeeds. This binary is built on the GitHub-hosted Ubuntu runner and is **not the same as the locally tested Arch Linux binary**. Some systems may require a source build due to differences in system libraries.

Extract the binary archive, then while SteamVR is running, install with:

```bash
./vrperf --install
```

This source archive has no prebuilt executable. The binary release asset is created when the GitHub tag workflow succeeds.

## Command-line options

```text
--install                  Install to ~/.local/bin and register SteamVR autostart
--uninstall                Unregister SteamVR and remove installed files
--mode head|left|right     Head-fixed (default) or controller-relative
--upload gl|raw            OpenGL upload (default); raw fallback for diagnostics
--hz auto|NUMBER           Target frame-rate budget (default auto)
--position X,Y,Z           Relative offset, in metres
--width METRES             Physical panel width (default 0.415)
--opacity 0.1..1.0         Panel opacity (default 0.93)
--interval-ms 100..1000    Panel update interval (default 200 ms)
--version                  Show version
-h, --help                 Show help
```

The program accepts an empty command-line argument because SteamVR can pass one when launching external overlays.

## Uninstall

With SteamVR running and vrperf stopped, run the installed command (or the built executable):

```bash
vrperf --uninstall
```

This unregisters autostart and removes the installed executable and manifest. It preserves your saved overlay visibility setting under `~/.config/vrperf/` (or `XDG_CONFIG_HOME`).

## Known limits

- `RES` is the manual **global** SteamVR resolution percentage, not the combined per-game render resolution.
- Battery percentages depend on data forwarded by the SteamVR/Steam Link runtime.
- CPU frame time is derived from SteamVR compositor fields. GPU scene time does **not** include encoder, network or headset decoder latency.
- A dropped-frame indicator represents SteamVR's compositor information, not an independent count of network packet loss.
- The overlay's OpenGL/EGL rendering path and hardware counters are tested only on the AMD/RADV setup listed above. On other drivers, vrperf can fall back to `--upload raw`, but that mode may flicker.
- NVIDIA NVML and Intel Level Zero monitoring are **experimental**. Intel GPU utilisation is based on combined 3D/render/compute engine activity, when available. Intel dedicated memory is reported only for modules marked as device-local.
- Integrated and discrete GPU selection is not yet configurable.
- SteamVR must be running for `--install` and `--uninstall`. Starting the program before SteamVR can start Steam components as a side effect of OpenVR initialisation.

## Reporting issues

Please include your Linux distribution, SteamVR version, headset, GPU model and driver, whether SteamVR is using Proton, your terminal output, and whether `--upload raw` changes the behaviour. Mention whether the fault affects the graph timings, hardware statistics, SteamVR dashboard or autostart.

## Source layout

```text
src/main.rs                 SteamVR connection and event loop
src/timing.rs               App FPS, CPU/GPU timing history and dropped frames
src/render.rs               Compact panel renderer and graphs
src/gpu_texture.rs          EGL/OpenGL upload, with raw fallback
src/hardware.rs             Linux CPU/RAM and GPU backend selection
src/hardware/nvml.rs        Optional NVIDIA NVML monitoring
src/hardware/level_zero.rs  Optional Intel Level Zero Sysman monitoring
src/hardware/dynlib.rs     Optional shared-library loader
src/registration.rs         Install/uninstall and SteamVR application manifest
src/dashboard.rs            In-headset dashboard toggle
src/devices.rs              HMD/controller metrics and local time
src/scene.rs                Game detection
src/config.rs               Command-line settings
src/overlay.rs              Panel visibility and tracking transforms
```

### Acknowledgement

vrperf was developed with the assistance of OpenAI's ChatGPT, which was used to help write, refine and troubleshoot the code. Development, testing and validation were carried out on Arch Linux with SteamVR and Steam Frame.

## Licence and API references

GNU GPL version 3 only (`GPL-3.0-only`). See [LICENSE](LICENSE).

- [OpenVR SDK](https://github.com/ValveSoftware/openvr)
- [Linux AMDGPU metrics](https://www.kernel.org/doc/html/latest/gpu/amdgpu/driver-misc.html)
- [NVIDIA NVML API](https://docs.nvidia.com/deploy/nvml-api/latest/index.html)
- [Intel Level Zero Sysman API](https://oneapi-src.github.io/level-zero-spec/)
