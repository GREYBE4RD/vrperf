# Advanced options and saved settings

[Back to README](../README.md#help-and-technical-documentation)

vrperf uses **command-line options and SteamVR dashboard controls**. It does not currently read a general user-editable `config.example` file. Head placement is configurable by command line; watch placement and size are configured in the dashboard.

## Command-line options

```text
--mode head|left|right      Select the Head or Left/Right watch
--upload gl|raw            OpenGL upload (default) or raw diagnostic mode
--hz auto|NUMBER           Override detected headset refresh rate
--width METRES             Head overlay width (default 0.415)
--position X,Y,Z           Head-relative position in metres
--interval-ms 100..1000    Display update interval (default 200 ms)
--capture-frame PATH       Save one source RGBA image to a PNG file
--install                  Install and register SteamVR autostart
--uninstall                Unregister and uninstall
--version                  Print version
--help                     Show help
```

The default HEAD position is `[0.0, -0.32, -0.85]` metres relative to the headset. The `--position` and `--width` options affect **HEAD only**. Watch position, angle and width must be set through [watch calibration](calibration.md).

SteamVR can pass an empty startup argument; vrperf ignores it. The older `--opacity`, `--wrist-style` and `--wrist-placement` options were removed in v1.20. Opacity is fixed at 0.93 and is not adjustable from the dashboard.

## Examples

Run a source build manually in HEAD mode:

```bash
./target/release/vrperf --mode head
```

Override the headset refresh-rate reading:

```bash
./target/release/vrperf --hz 90
```

Change HEAD positioning and width:

```bash
./target/release/vrperf --mode head --position 0,-0.3,-0.8 --width 0.4
```

Capture one rendered frame as PNG before SteamVR upload:

```bash
./target/release/vrperf --capture-frame /tmp/vrperf-frame.png
```

Test the raw upload mode if the OpenGL/EGL path fails:

```bash
./target/release/vrperf --upload raw
```

For foreground testing, start SteamVR first and **stop any existing vrperf instance**. These are launch-time options; the dashboard still controls ON/OFF, display mode and watch calibration.

## Saved settings

By default, vrperf stores its preferences in `~/.config/vrperf/`:

| File | Purpose |
| --- | --- |
| `visible` | Saved ON/OFF state |
| `mode` | Selected HEAD/LEFT/RIGHT display |
| `watch_left` | Left watch position, rotation and width |
| `watch_right` | Right watch position, rotation and width |

If `XDG_CONFIG_HOME` is set, vrperf uses its `vrperf` subdirectory. Watch placement is stored as a validated transform and width, not as three simple XYZ values. Do not manually edit it unless you understand the format.

Updating or uninstalling vrperf does not remove these files. To return a watch to its initial placement, use the dashboard's **RESET** button while calibrating. For manual recovery, see [troubleshooting](troubleshooting.md#resetting-a-saved-watch).
