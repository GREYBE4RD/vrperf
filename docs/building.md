# Building, installing and updating vrperf

[Back to README](../README.md#install)

## Requirements

vrperf needs Linux x86-64, SteamVR, working graphics drivers with OpenGL/EGL support and system libraries compatible with the binary. The source build also requires a Rust toolchain that supports the **2024 edition**, a C/C++ compiler, CMake and Clang/libclang. Cargo downloads Rust dependencies on the first build.

### Arch Linux

```bash
sudo pacman -S --needed rust base-devel cmake clang libglvnd
```

### Ubuntu/Debian

```bash
sudo apt install cargo rustc build-essential cmake clang libclang-dev libegl-dev libgl-dev
```

If the distribution's Rust version does not support edition 2024, install a current toolchain from [rustup](https://rustup.rs/).

## Build from source

Clone the current published source:

```bash
git clone https://github.com/GREYBE4RD/vrperf.git && cd vrperf
```

Run the tests and build a release executable:

```bash
cargo test --locked && cargo build --release --locked
```

The executable is `target/release/vrperf`. With SteamVR running, you may launch it in the foreground to test without installing:

```bash
./target/release/vrperf
```

Stop this test process before installation. Running two instances at once can cause overlay-registration conflicts.

## Install

**Start SteamVR before installing.** For a source build, run:

```bash
./target/release/vrperf --install
```

For a downloaded release, extract the Linux archive, then run `./vrperf --install` from the extracted executable's directory.

The installer:

- Copies the executable to `~/.local/bin/vrperf`.
- Adds the SteamVR application manifest and controller action/binding files under `~/.local/share/vrperf/`.
- Enables SteamVR automatic startup for the overlay.

**Fully restart SteamVR** after installing. To control automatic startup separately from the overlay's ON/OFF button, open **SteamVR Settings > Startup / Shutdown > Choose Startup Overlay Apps**.

## Updating

You do not need to uninstall the old version or reset your calibrations.

1. Obtain and build/extract the new version. For a Git checkout, run `git pull --ff-only` before the build.
2. Start SteamVR and stop the existing vrperf process, including a copy launched at startup.
3. Run the **new executable** with `--install`.
4. Fully restart SteamVR and check that the new version appears in the dashboard.

A ZIP download is not a Git checkout, so `git pull` does not work inside an extracted ZIP folder. The installer updates the binary and controller binding files but leaves saved settings in place.

## Verify a downloaded archive

GitHub Releases provides a `.sha256` checksum file alongside the Linux archive. In the download directory, you can verify it before extracting:

```bash
sha256sum -c vrperf-v*-linux-x86_64.tar.gz.sha256
```

This assumes the directory contains only one version's matching archive and checksum.

## Uninstall

Stop any vrperf process, start SteamVR, then run:

```bash
~/.local/bin/vrperf --uninstall
```

This removes the executable and SteamVR registration, but not settings stored under your user configuration directory.

For common problems, see [troubleshooting](troubleshooting.md#installation-and-startup). For command-line settings, see [advanced options](configuration.md).
