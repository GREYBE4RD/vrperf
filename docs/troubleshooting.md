# Troubleshooting

[Back to README](../README.md#help-and-technical-documentation)

## Installation and startup

**The vrperf dashboard tab is missing:** Start SteamVR before running the installer. Use the new executable with `--install`, then **fully restart SteamVR**. The tab icon may appear as `?`; hover over it to find vrperf. Check **SteamVR Settings > Startup / Shutdown > Choose Startup Overlay Apps** to see whether it is enabled to start automatically.

**Installation or removal fails:** Run the command with SteamVR active. Check the file paths included in the terminal error; the installer writes under `~/.local/bin` and `~/.local/share/vrperf/`. For build dependencies and steps, see [building and installing](building.md).

**The overlay does not appear even though the tab exists:** Confirm vrperf is **ON** and the expected HEAD/LEFT/RIGHT mode is selected. A saved watch also depends on its orientation and controller tracking.

## Watch calibration

**The watch does not move during calibration:** Select CAL LEFT or CAL RIGHT, **close the dashboard**, then release and press the **opposite** controller's grip again. Move that controller while gripping. Calibration can continue inside a game. Grip must be released and pressed after calibration starts.

**Grip/trigger does nothing:** Open **SteamVR Settings > Controllers > Manage Controller Bindings**, select vrperf and check the action bindings. Some non-Steam Frame controllers have untested bindings. Start vrperf manually from a terminal if you need input diagnostics, but avoid running two copies at once.

**A saved watch does not appear:** Turn its calibrated face towards your eyes and look at it. Calibrated watches hide when you turn away or lose tracking. If the position is unusable, enter calibration and press **RESET** to preview the headset-facing default. Use SAVE to keep it or CANCEL to restore the old saved calibration.

**A new watch points away from you:** An uncalibrated or reset watch should keep its face directed towards the headset until manual gripping. Verify that the watch has no saved calibration. If it does, use RESET in the calibration dashboard.

### Resetting a saved watch

Normally use **CAL LEFT/RIGHT > RESET > SAVE**. If the dashboard cannot be used, you can also stop vrperf and **rename** (not delete) the relevant preference file, such as `~/.config/vrperf/watch_left` or `~/.config/vrperf/watch_right`, then restart vrperf. This makes only that watch uncalibrated again. If `XDG_CONFIG_HOME` is set, these files are in its `vrperf` directory instead.

See [watch calibration](calibration.md) for the normal RESET, SAVE and CANCEL behaviour.

## Updates and old versions

**The old layout or behaviour remains after upgrading:** Check that you used the newer executable with `--install`. Stop existing manual or autostarted instances, then fully restart SteamVR. An earlier instance may continue using the previous binary until it exits.

**A new source build fails:** Confirm you have a Rust toolchain supporting edition 2024 and the distribution dependencies from the [build guide](building.md#requirements). Run `cargo test --locked` before `cargo build --release --locked` to separate test errors from build errors.

## Missing hardware readings

Unavailable readings appear as `--` or `--°`. Some values require active SteamVR information, kernel sensor interfaces or optional NVIDIA/Intel libraries. The AMD, NVIDIA and Intel interfaces are not interchangeable; absence of a vendor interface does not imply that vrperf can read the same counter through DRM.

See [hardware and sensor limitations](hardware.md).

## Display or upload problems

The default upload method is OpenGL/EGL. If OpenGL texture upload fails, vrperf may try the raw fallback; raw mode can flicker. To test it directly, run a single foreground instance with:

```bash
./target/release/vrperf --upload raw
```

The command above applies to a source checkout after building; a downloaded release can use `./vrperf --upload raw` before installation.

vrperf's CPU/GPU frame times do not include headset encoding, Wi-Fi transmission or decoding. See [display measurements](display.md#frame-times-and-graph-colours) before using them to diagnose streaming delays.

## Reporting an issue

Report problems on [GitHub Issues](https://github.com/GREYBE4RD/vrperf/issues). Include your distribution, SteamVR version, headset and controllers, GPU/driver, vrperf version, steps to reproduce, and relevant terminal messages. The [advanced options](configuration.md) include a PNG capture option for display diagnosis.
