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
- Automatic startup with SteamVR after installation.
- Remembers the last ON/OFF state between SteamVR sessions.
- Single native Rust executable. Optional vendor monitoring libraries load only when needed.

## How it works

Once installed, vrperf starts automatically with SteamVR and displays a compact performance overlay near the bottom of your headset's view. It runs natively on Linux and uses SteamVR's OpenVR interface to collect frame timings and headset information.

### Understanding the performance graphs

The CPU and GPU graphs show application frame times reported by SteamVR, measured in milliseconds (ms).

Lower frame times indicate faster rendering. The horizontal reference line represents the time available to render each frame at the headset's current refresh rate.

For example:

| Refresh rate | Frame budget |
| --- | --- |
| 72 Hz | 13.89 ms |
| 90 Hz | 11.11 ms |
| 120 Hz | 8.33 ms |
| 144 Hz | 6.94 ms |

Frame times above the reference line indicate that the application is exceeding its frame budget.

Dropped-frame markers show frames identified through SteamVR's timing information.

These measurements describe **PC-side rendering performance**. They do not include video encoding, network transmission or headset decoding latency.

### Show or hide the overlay

Open the **SteamVR dashboard** and select the **vrperf** tab. Use the **ON/OFF** button to show or hide the performance overlay.

**The overlay remembers its last ON/OFF state.** If you turn it off, it will remain hidden the next time SteamVR starts. The same applies when turning it on.

Hiding the overlay does not stop vrperf from running. To stop it completely, disable automatic startup and restart SteamVR, or close the vrperf process.

![vrperf dashboard controls](screenshots/vrperf.on.off.jpg)

### Automatic startup

vrperf is registered to launch automatically with SteamVR during installation.

To enable or disable automatic startup:

1. Open the SteamVR window on your desktop.
2. Go to **Settings → Startup / Shutdown**.
3. Select **Choose Startup Overlay Apps**.
4. Enable or disable **vrperf**.

Disabling automatic startup does not uninstall vrperf. You can still launch it manually using `vrperf` in a terminal, provided `~/.local/bin` is in your `PATH`.

Automatic startup and overlay visibility are separate settings. You can leave vrperf running automatically while keeping the overlay hidden until you need it.

## Compatibility and support status

| Component | Status |
| --- | --- |
| Linux SteamVR + Steam Frame | Tested on Arch Linux |
| AMD Radeon RX 9070 XT / RADV | **Tested, stable** |
| Other AMD GPUs | Best effort, untested |
| NVIDIA proprietary driver | Experimental, untested |
| Intel `i915` / `xe` | Experimental, untested |
| Other headsets and GPU configurations | Untested |

The performance graphs and headset data use **OpenVR**, not an AMD-specific interface.

Hardware utilisation uses separate, optional monitoring backends:

- **AMD:** Linux `amdgpu` sysfs interfaces. No additional vendor library required.
- **NVIDIA:** NVML (`libnvidia-ml.so.1`) from the proprietary NVIDIA driver. No `nvidia-smi` subprocess is used. The device is matched by PCI address.
- **Intel:** Level Zero Sysman (`libze_loader.so.1`) with a compatible Intel driver. The first Intel Sysman device is used. On systems with multiple Intel GPUs, device selection may be incorrect. Only dedicated device-local memory is displayed as VRAM.

If a monitoring library or measurement is unavailable, the affected metric displays `--`.

**VR frame timings should still work on unsupported GPU monitoring configurations**, provided SteamVR and its OpenVR timing interface are functioning.

NVIDIA and Intel support has not yet been tested on physical hardware.

### Multi-GPU systems

If both AMD and NVIDIA GPUs are present, the initial detection prefers NVIDIA, followed by AMD and then Intel.

This may not match the GPU SteamVR uses on hybrid systems. Manual GPU selection is not yet supported.

## Installation

You can download a precompiled binary from GitHub Releases or build vrperf from source.

### Runtime requirements

The precompiled binary does **not** require Rust, Cargo or other build tools.

- **OS:** x86-64 Linux.
- **SteamVR:** Installed and working with a connected VR headset.
- **Graphics:** Working OpenGL/EGL drivers (`libEGL.so.1` and `libGL.so.1`).
- **System libraries:** Compatible glibc and other standard Linux libraries.
- **AMD:** AMDGPU kernel driver for GPU monitoring.
- **NVIDIA:** Proprietary NVIDIA driver with NVML for GPU monitoring (experimental).
- **Intel:** Level Zero loader and compatible Intel runtime for GPU monitoring (experimental).

NVIDIA and Intel monitoring libraries are optional. If unavailable, vrperf should still display SteamVR frame timings, but some hardware statistics may show `--`.

Precompiled binaries may not work on every Linux distribution due to differences in system libraries. If the binary fails to run, try building from source.

### Install a precompiled binary

Download the Linux x86-64 archive from the [GitHub Releases](https://github.com/GREYBE4RD/vrperf/releases) page.

Extract the archive:

```bash
tar -xzf vrperf-v1.11.0-linux-x86_64.tar.gz
```

With SteamVR running, install vrperf:

```bash
./vrperf --install
```

The installer:

- Copies the executable to `~/.local/bin/vrperf`.
- Creates a SteamVR application manifest under `~/.local/share/vrperf/`.
- Registers vrperf as a SteamVR dashboard overlay.
- Enables automatic startup with SteamVR.

Restart SteamVR to activate automatic startup.

**Note:** The precompiled binary may depend on shared libraries from the distribution used to build it. Building from source is recommended if you encounter compatibility issues.

## Building from source

### Build prerequisites

Building vrperf requires:

- Rust and Cargo, using a stable toolchain supporting Rust 2024 edition.
- A C/C++ compiler and standard development tools.
- CMake.
- Clang and libclang for the OpenVR bindings.
- Internet access for the initial Cargo dependency download.

These dependencies are required only for compilation. They are not needed to run the precompiled binary.

### Arch Linux

Install the required packages:

```bash
sudo pacman -S --needed rust base-devel cmake clang libglvnd
```

Arch Linux is the development distribution used for the tested AMD build.

### Ubuntu / Debian (untested)

```bash
sudo apt install cargo rustc build-essential cmake clang libclang-dev libegl-dev libgl-dev
```

Use a current stable Rust toolchain that supports edition 2024. Distribution packages may be too old. If necessary, install Rust using [rustup](https://rustup.rs/).

### Fedora (untested)

```bash
sudo dnf install rust cargo gcc-c++ cmake clang-devel libglvnd-devel
```

NVIDIA GPU utilisation additionally requires NVIDIA's NVML runtime library, normally installed with the proprietary driver.

Intel GPU monitoring requires the Level Zero loader and a compatible Intel compute runtime.

Neither library is required simply to launch vrperf.

### Compile vrperf

Clone the repository:

```bash
git clone https://github.com/GREYBE4RD/vrperf.git
```

Enter the project directory:

```bash
cd vrperf
```

Build the release executable:

```bash
cargo build --release --locked
```

Run the automated tests:

```bash
cargo test --locked
```

The compiled executable will be available at:

```text
target/release/vrperf
```

With SteamVR running, test vrperf manually:

```bash
./target/release/vrperf
```

Stop the manually launched instance with `Ctrl+C` before installing it.

### Install and enable SteamVR autostart

With SteamVR running and no manually running vrperf instance, execute:

```bash
./target/release/vrperf --install
```

This installs the executable to `~/.local/bin/vrperf`, creates the SteamVR application manifest and enables automatic startup.

Restart SteamVR to test automatic startup.

The installer uses SteamVR's native application registration. It does not create a systemd service or require a separate launcher.

### PATH and the vrperf terminal command

The installer places the executable in:

```text
~/.local/bin/vrperf
```

Some Linux distributions do not add `~/.local/bin` to the user's `PATH` automatically.

Check your current `PATH`:

```bash
printf '%s\n' "$PATH" | tr ':' '\n'
```

If `$HOME/.local/bin` is missing, add the following line to your login shell configuration. For Bash, this is commonly `~/.profile`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Log out and back in, or source the updated file to apply the change.

Verify that vrperf is available:

```bash
command -v vrperf
```

The expected location is:

```text
/home/YOUR_USER/.local/bin/vrperf
```

**SteamVR automatic startup does not require `~/.local/bin` in your `PATH`.** It uses the executable's absolute path.

### Updating vrperf

To update an existing installation:

1. Download or build the newer version.
2. Start SteamVR.
3. Stop any manually running vrperf instance.
4. Run the newer executable with `--install`.
5. Restart SteamVR.

For a source build:

```bash
./target/release/vrperf --install
```

For a downloaded binary:

```bash
./vrperf --install
```

The installer replaces the existing executable and updates the SteamVR registration.

Your saved ON/OFF preference is preserved.

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

vrperf accepts an empty command-line argument because SteamVR can pass one when launching external overlays.

## Uninstall

With SteamVR running and vrperf stopped, run:

```bash
vrperf --uninstall
```

If `~/.local/bin` is not in your `PATH`, use:

```bash
~/.local/bin/vrperf --uninstall
```

This:

- Disables automatic startup.
- Unregisters the SteamVR application manifest.
- Removes the installed executable and manifest.

Your saved overlay visibility setting is preserved under `~/.config/vrperf/`, or the equivalent location specified by `XDG_CONFIG_HOME`.

## Known limits

- **Resolution:** `RES` displays the manual global SteamVR resolution percentage. It does not include per-application resolution settings or show the combined effective render resolution. When SteamVR uses automatic resolution, the value displays `--`.
- **Battery levels:** Headset and controller battery percentages depend on information provided by the SteamVR/Steam Link runtime.
- **CPU timing:** CPU frame time is derived from SteamVR compositor timing fields.
- **GPU timing:** GPU scene time does not include video encoding, network transmission or headset decoding latency.
- **Dropped frames:** Markers use SteamVR compositor timing information. They do not independently measure network packet loss.
- **Graphics compatibility:** The OpenGL/EGL rendering path has been tested on the AMD/RADV configuration listed above. On other drivers, vrperf can fall back to `--upload raw`, although this mode may flicker.
- **NVIDIA monitoring:** NVML integration is experimental and untested on physical NVIDIA hardware.
- **Intel monitoring:** Level Zero integration is experimental. GPU utilisation uses combined 3D/render/compute engine activity when available. Dedicated VRAM is reported only for device-local memory.
- **Multi-GPU systems:** Manual selection between integrated and discrete GPUs is not currently supported.
- **SteamVR dependency:** SteamVR must be running when using `--install` or `--uninstall`. Starting vrperf before SteamVR is running can start Steam components as a side effect of OpenVR initialisation.

## Reporting issues

Please use [GitHub Issues](https://github.com/GREYBE4RD/vrperf/issues) to report problems.

Include:

- Linux distribution and version.
- SteamVR version.
- Headset model and connection method.
- GPU model and driver.
- Whether the game is running through Proton.
- Relevant terminal output or error messages.
- Whether `--upload raw` changes the behaviour.

Please indicate whether the problem affects frame timings, hardware statistics, dashboard controls, rendering or automatic startup.

Reports from NVIDIA, Intel and other AMD GPU users are particularly useful, as these configurations have not yet been tested.

## Source layout

```text
src/main.rs                 SteamVR connection and event loop
src/timing.rs               App FPS, CPU/GPU timing history and dropped frames
src/render.rs               Compact panel renderer and graphs
src/gpu_texture.rs          EGL/OpenGL upload, with raw fallback
src/hardware.rs             Linux CPU/RAM and GPU backend selection
src/hardware/nvml.rs        Optional NVIDIA NVML monitoring
src/hardware/level_zero.rs  Optional Intel Level Zero Sysman monitoring
src/hardware/dynlib.rs      Optional shared-library loader
src/registration.rs         Install/uninstall and SteamVR application manifest
src/dashboard.rs            In-headset dashboard toggle
src/devices.rs              HMD/controller metrics and local time
src/scene.rs                Game detection
src/config.rs               Command-line settings
src/overlay.rs              Panel visibility and tracking transforms
```

## Acknowledgements

vrperf was developed with the assistance of OpenAI's ChatGPT, which was used to help write, refine and troubleshoot the code.

Development, testing and validation were carried out on Arch Linux with SteamVR and Steam Frame.

## Licence and API references

vrperf is licensed under GNU GPL version 3 only (`GPL-3.0-only`). See [LICENSE](LICENSE).

- [OpenVR SDK](https://github.com/ValveSoftware/openvr)
- [Linux AMDGPU metrics](https://www.kernel.org/doc/html/latest/gpu/amdgpu/driver-misc.html)
- [NVIDIA NVML API](https://docs.nvidia.com/deploy/nvml-api/latest/index.html)
- [Intel Level Zero Sysman API](https://oneapi-src.github.io/level-zero-spec/)
