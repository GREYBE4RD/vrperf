# Display readings, graphs and warning colours

[Back to README](../README.md#what-the-display-means)

## HEAD and watch

**HEAD** is a larger display fixed to your view. **LEFT** and **RIGHT** use a compact **550 × 160 pixel** panel. The watch has one information row above the graph and another below it.

The top watch row shows FPS, CPU frame time, GPU frame time, dropped frames, resolution and local time. The lower row shows headset battery (**BAT**), CPU utilisation and temperature, system memory (**RAM**), GPU utilisation and temperature, and video memory (**VRA**).

The watch omits controller batteries, refresh rate and the numeric frame-budget field. The frame-budget reference remains visible in its graph.

## Frame times and graph colours

The display measures PC-side SteamVR compositor timing. It does **not** measure video encoding, network or Wi-Fi transfer, or headset decoding delay. A PC frame-time warning alone does not identify the cause of wireless VR stutter.

The budget is `1000 / headset refresh rate` milliseconds. At **90 Hz**, a frame has a budget of about **11.11 ms**.

| Graph item | Colour or behaviour |
| --- | --- |
| CPU trace within budget | Blue |
| CPU trace above budget | Amber/yellow |
| GPU trace within budget | Green |
| GPU trace above budget | Orange |
| Dashed horizontal line | Muted amber frame-time budget |
| Vertical dropped-frame markers | Red |

Only portions of traces above the budget change colour; they return to their normal colours below it. Numeric CPU/GPU frame times use matching warning colours.

The graph uses about 65% of its height for times below budget and 35% for times from one to twice the budget. Larger spikes clip at the top of the graph, while the numeric readout still shows the measured time. Each graph includes individual sampled frames rather than an averaged trace.

## Values and compact formatting

- **FPS:** The watch rounds upward to a whole number. HEAD retains its existing precision.
- **CPU/GPU frame time:** Units are shown as lowercase `ms`. Watch readings below 100 ms use one decimal; larger readings are shortened to fit.
- **DROP:** SteamVR compositor dropped-frame count. Large watch values use compact K/M/B/T/P suffixes. This is not a wireless packet-loss counter.
- **RES:** Combined SteamVR global and per-application resolution percentages, where available. It does not show a game's own upscaling or internal render resolution.
- **RAM/VRA:** Used/total memory in GiB. The watch omits the `G` suffix and uses `T` for very large TiB quantities.
- **Temperature:** Rounded to whole degrees Celsius. If unavailable, shows `--°`.

The watch distributes gaps across both text rows using the widths of currently displayed fields. A field may move slightly when another field gains digits. The graph and text rows share their outer boundaries.

To fit the compact panel, the watch's bottom-row characters are slightly narrower but remain 16 pixels high. **Watch BAT, CPU and GPU percentages are capped at 99%**: a real 100% reads as 99%. **Temperature readouts in both modes are capped at 99°**, even if the actual value is higher. HEAD utilisation percentages are not capped. These are display limits only: warning colours still use the real measurements.

## Warning thresholds

| Reading | Normal | Orange | Red |
| --- | --- | --- | --- |
| CPU/GPU utilisation | Below 85% | 85% to below 95% | 95% and above |
| RAM/video memory use | Below 85% | 85% to below 95% | 95% and above |
| Headset battery; controller batteries on HEAD | Above 20% | 11% to 20% | 10% and below |
| CPU/GPU temperature | Below 80°C | 80°C to below 90°C | 90°C and above |

Memory warning colours use the actual used/total ratio before rounding. Battery warnings indicate low remaining charge. Temperature thresholds are generic visual alerts, **not device-specific safe temperature limits**. CPU/GPU utilisation and temperature colours are independent.

Missing readings show `--` or `--°` and do not cause warning colours. For sensor availability, see [CPU/GPU compatibility](hardware.md).
