# Developer notes

[Back to README](../README.md#help-and-technical-documentation)

vrperf is a native Rust application using OpenVR, direct panel rendering and an optional OpenGL/EGL texture upload path. The user-facing setup and calibration instructions remain in the [main README](../README.md).

## Rendering and measurements

- The watch is **575 × 160 pixels**, with two text rows and one graph. Text remains 16 pixels high; its lower-row glyphs use up to 66% of HEAD character advance and can narrow further when necessary.
- The effective watch width in SteamVR is scaled with the image width. Existing saved placement and width values remain unchanged during layout updates.
- The normal upload path uses an EGL/OpenGL texture. The application can fall back to `SetOverlayRaw`, which may flicker on some systems.
- The OpenGL texture allocation is fixed during a run. Changing between HEAD and watch changes the visible region rather than replacing the texture.
- Rendered frames use contiguous RGBA byte data with four bytes per pixel. Texture dimensions must fit the OpenGL integer dimension types; buffer lengths are checked before upload.
- The panel uses straight RGBA and fixed OpenVR opacity of **0.93**.
- Frame timing comes from SteamVR/OpenVR. CPU/GPU timings are PC-side and do not cover encoding, network or headset decoding.
- GPU monitoring uses AMD DRM sysfs, NVIDIA NVML or Intel Level Zero Sysman as available. See [hardware support](hardware.md) for limitations and sensor selection.

## Source layout

```text
src/main.rs           SteamVR event loop and configuration
src/overlay.rs        Head/watch tracking, visibility and calibration movement
src/calibration.rs    Watch transforms, validation and persistence format
src/input.rs          SteamVR grip/trigger action polling
src/dashboard.rs      Dashboard and calibration controls
src/registration.rs   Install/uninstall and SteamVR action bindings
src/render.rs         Panel rendering and graphs
src/gpu_texture.rs    EGL/OpenGL texture upload
src/timing.rs         Frame times, FPS and dropped frames
src/hardware.rs       Host CPU, RAM and GPU monitoring
src/hardware/         Optional NVIDIA/Intel monitoring backends
src/devices.rs        Controller batteries and resolution settings
src/scene.rs          Game detection
src/config.rs         Command-line configuration
src/capture.rs        Diagnostic PNG capture
assets/steamvr/       SteamVR action manifest and controller bindings
```

## Tests and releases

Run tests and build the binary with:

```bash
cargo test --locked && cargo build --release --locked
```

[Latest release notes](../RELEASE_NOTES.md) describe the current version; [release history](../CHANGELOG.md) preserves all previous notes. The GitHub Actions workflow builds the Linux executable when a version tag is pushed or a GitHub release is published, then uploads `vrperf.tar.gz` and `vrperf.tar.gz.sha256`. Release titles and asset filenames do not include a version; Git tags and `vrperf --version` identify each version.

## References

- [OpenVR](https://github.com/ValveSoftware/openvr)
- [SteamVR action manifests](https://github.com/ValveSoftware/openvr/wiki/Action-manifest)
- [Linux hardware-monitoring sysfs](https://docs.kernel.org/hwmon/sysfs-interface.html)
- [AMD amdgpu driver details](https://www.kernel.org/doc/html/latest/gpu/amdgpu/driver-misc.html)
- [NVIDIA NVML](https://docs.nvidia.com/deploy/nvml-api/latest/index.html)
- [Intel Level Zero Sysman](https://oneapi-src.github.io/level-zero-spec/)
