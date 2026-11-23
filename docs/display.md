# Display readings, graphs and warning colours

[Back to README](../README.md#what-the-display-means)

## HEAD and watch

**HEAD** is a larger display fixed to your view. **LEFT** and **RIGHT** use a compact **575 × 160 pixel** panel. The watch has one information row above the graph and another below it.

The top watch row shows measured FPS and target refresh rate together (`FPS:72/90Hz`), CPU/GPU frame time, dropped frames, resolution and local time. The lower row shows headset, left and right controller batteries (`BAT:H99/L85/R72%`), CPU utilisation and temperature, system memory (`RAM`), GPU utilisation and temperature, and video memory (`VRA`).

In watch mode, headings use colons instead of spaces. CPU/GPU usage and temperature have no space between them (for example `CPU:21%50°`). The watch omits the numeric frame-budget field; its graph still shows the reference line. White text marks fixed labels, units, battery separators, memory totals and target Hz. Normal measurements are green; warning measurements turn orange or red. Missing values are grey. The clock stays white. Each battery is coloured separately, and CPU/GPU graph lines retain their distinct colours.

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

Only portions of traces above the budget change colour; they return to their normal colours below it. The watch's numeric CPU/GPU frame times are green within budget, orange above budget and red above twice the budget. HEAD keeps its existing CPU/GPU-specific frame-time colours.

The graph uses about 65% of its height for times below budget and 35% for times from one to twice the budget. Larger spikes clip at the top of the graph, while the numeric readout still shows the measured time. Each graph includes individual sampled frames rather than an averaged trace.

## Values and compact formatting

- **FPS:** The watch rounds measured FPS upward to a whole number and shows the target refresh beside it (for example `FPS:45/90Hz`); HEAD retains its existing precision and a separate Hz field. The target is the headset's SteamVR reported refresh rate (or `--hz` override); if this cannot be read, vrperf defaults to 90 Hz. Measured FPS is green at 95% or more of target, orange at 80% to below 95%, and red below 80%. The status uses the unrounded measurement.
- **CPU/GPU frame time:** Units are shown as lowercase `ms`. Watch readings below 100 ms use one decimal; larger readings are shortened to fit.
- **DROP:** SteamVR compositor cumulative dropped-frame count since the current scene or monitor reset. The number stays red while any dropped-frame marker is visible in the watch graph, and returns to green when the final marker scrolls off the left edge. The cumulative count remains unchanged. Large watch values use compact K/M/B/T/P suffixes. This is not a wireless packet-loss counter.
- **RES:** Combined SteamVR global and per-application resolution percentages, where available. It does not show a game's own upscaling or internal render resolution.
- **RAM/VRA:** Used/total memory in GiB. The watch omits the `G` suffix and uses `T` for very large TiB quantities.
- **Temperature:** Rounded to whole degrees Celsius. If unavailable, shows `--°`.

The watch distributes gaps across both text rows using the widths of currently displayed fields. When readings become longer, it narrows only the horizontal character spacing to keep at least 5 pixels between fields. Characters stay 16 pixels high. A field may move slightly when another field gains digits. The graph and text rows share their outer boundaries.

To keep the watch compact, the bottom row uses adjustable horizontal character spacing (up to 66% of the HEAD glyph advance) while remaining 16 pixels high. The wider 575-pixel raster provides more space for the same readings than the previous 550-pixel watch. **Watch BAT, CPU and GPU percentages are capped at 99%**: a real 100% reads as 99%. **Temperature readouts in both modes are capped at 99°**, even if the actual value is higher. HEAD utilisation percentages are not capped. These are display limits only: warning colours still use the real measurements.

## Warning thresholds

| Reading | Normal | Orange | Red |
| --- | --- | --- | --- |
| CPU/GPU utilisation | 90% and below | Above 90% to 95% | Above 95% |
| RAM/video memory use | Below 85% | 85% to below 95% | 95% and above |
| Headset and controller batteries (HEAD and watch) | Above 20% | 11% to 20% | 10% and below |
| CPU/GPU temperature | Below 80°C | 80°C to below 90°C | 90°C and above |

Memory warning colours use the actual used/total ratio before rounding. Battery warnings indicate low remaining charge. Temperature thresholds are generic visual alerts, **not device-specific safe temperature limits**. CPU/GPU utilisation and temperature colours are independent.

Missing readings show `--` or `--°` in grey and do not cause warning colours. For sensor availability, see [CPU/GPU compatibility](hardware.md).
