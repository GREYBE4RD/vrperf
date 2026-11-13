# vrperf v1.35.0 (development temperature monitoring, 2026-11-12)

- Show CPU and GPU temperatures in HEAD and watch modes. Keep the watch at 550 x 160 pixels; condense only its bottom text and preserve evenly spaced fields.
- AMD CPU hwmon Tdie/Tctl, Intel CPU coretemp, AMD/Intel GPU hwmon, NVIDIA NVML, and optional Intel Level Zero Sysman GPU temperatures; show --° if absent.
- Temperature colour is orange at 80°C and red at 90°C, independent of CPU/GPU utilisation warnings.
- Cap displayed temperatures in both modes at 99° and watch battery/CPU/GPU percentages at 99% to prevent three-digit fields. Underlying warning values are not capped. HEAD utilisation remains uncapped.
- Document sensor limitations, GPU selection caveats and display caps in the README. Add tests for formatting, thresholds and worst-case watch row widths.
- Needs cargo tests, release build and SteamVR validation before publishing.

---

# vrperf v1.34.0 (2026-11-12)

- Shorten the dashboard position heading to "POSITION".
- Update the idle calibration instructions to: select a watch, close the SteamVR dashboard, grip the other controller and move the overlay, then reopen the dashboard to save or cancel.
- Remove the "SAVE PERSISTS AFTER RESTART" instruction from all dashboard calibration states. Saved calibration data remains persistent; only the on-screen text changes.
- Use narrower 16-pixel-high instruction lettering so the longer third instruction fits inside the existing 512 x 600 dashboard without clipping. Keep all other labels, controls and layout unchanged.
- Update the instruction layout tests and README. Verify the updated dashboard in SteamVR.

---

# vrperf v1.33.0 (2026-11-11)

- Redesign the SteamVR dashboard with separate ON and OFF buttons at the top, followed by the position selection. Keep the dashboard at 512 x 600 pixels and preserve calibration controls and resolution readouts.
- Update the title to "vrperf" / "PCVR Performance Overlay" and the position label to "POSITION HEAD / WATCH L / R". Do not add a fixed-opacity label.
- Make ON and OFF actions idempotent so selecting an already active state has no effect; keep visibility locked on during calibration.
- Restore the original v1.11 uncalibrated controller-relative watch offset (Y +0.12 m, Z -0.13 m) for first-use positioning. Saved left/right calibrations remain unchanged.
- Add on-screen confirmation that saved calibrations survive restarts, and document what happens when calibration is cancelled or interrupted.
- Clarify GPU monitoring fallback behaviour: AMD uses DRM sysfs; missing NVIDIA NVML or Intel Level Zero means those GPU counters are unavailable unless another supported device is detected.
- Improve installation and removal error messages with the affected file paths. Document and validate EGL texture dimension and RGBA buffer assumptions.
- Update dashboard tests and documentation. Runtime behaviour requires an in-headset check before release.

---

# vrperf v1.32.0

- Narrow the watch from 560 x 160 to 550 x 160 pixels, with the corresponding physical-width change. Keep the same glyph height, graph height and saved watch calibrations.
- Space the visible fields evenly across both top and bottom watch rows, using current text widths. Fields can shift when readings gain digits; even the widest formatted readings fit without overlap.
- Round watch FPS readings upward to the next whole frame per second. Keep the HEAD FPS precision unchanged.
- Change battery warnings in both HEAD and watch modes: orange at 11–20%, red at 0–10%.
- Remove the "FIXED OPACITY" dashboard label without changing the existing opacity setting.
- Update layout, FPS and battery threshold tests and the display documentation.

---

# vrperf v1.31.0 (development layout refinement)

- Narrow the watch from 584 x 160 to 560 x 160 pixels. Reduce its physical width proportionally, without changing saved calibrations.
- Keep the font at 16 pixels high; slightly reduce watch-only character width to fit the reserved fields.
- Distribute gaps evenly across the top and bottom rows. Centre shorter readings within their fixed-width slots to avoid large visual gaps while keeping other fields stationary when counts change.
- Order watch footer BAT, CPU, RAM, GPU, VRA; keep FPS, CPU ms, GPU ms, DROP, RES and clock on top.
- Show lowercase `ms` in numeric frame-time readings and the HEAD budget label.
- Keep graph aligned to both rows. Add tests for fixed slots and the shortened watch layout.
- SteamVR headset validation is required before publishing.

---

# vrperf v1.30.0 (development watch compaction)

- Reduce watch raster width from 640 to 584 pixels, preserving the 160-pixel height and 16-pixel text height.
- Reduce the SteamVR watch physical width by the same approximately 9% ratio. Keep saved calibration data unchanged, and leave HEAD size and controls unchanged.
- Reduce reserved field spacing from 6 to 2 pixels. Keep fixed field starts so DROP 9 becoming DROP 10 never moves other labels.
- On the watch only, show whole-millisecond frame times at 100 ms and above, and abbreviate dropped-frame counts from 1,000 upward.
- Keep two aligned information rows, the full-width graph, warning colours and calibration behaviour.
- Add tests for condensed values and effective physical watch width. Verify in SteamVR before publishing.

---

# vrperf v1.29.0 (development compile correction)

- Correct the watch helper function return types: the top row has six fields (FPS, CPU ms, GPU ms, DROP, RES, time) and the bottom row has five fields (BAT, CPU%, GPU%, VRA, RAM).
- Fix the Rust E0308 compile errors in v1.28 without changing row spacing, graph dimensions, colours, or watch calibration.
- Run Cargo tests and verify the updated watch in SteamVR before publishing.

---

# vrperf v1.28.0 (development watch layout correction)

- Reduce watch raster width from 800 to 640 pixels while increasing its height from 144 to 160. The physical watch width and saved calibration are unchanged; the panel has a less stretched aspect ratio.
- Reserve 16 pixels above the first row and 14 pixels below the second row to avoid the clipping reported from v1.27 headset testing.
- Use a horizontally condensed watch-only bitmap font with its original 16-pixel glyph height. Keep head mode untouched.
- Make both text rows and the graph span the same left/right boundaries. Keep fixed slots for changing DROP, FPS, RES and memory values.
- Move the clock to the end of the top row after measuring both maximum row widths. Remove the layout helper that caused a dead-code warning in release builds.
- Add margin and layout tests. Keep GPU texture height the same in both modes to avoid vertical texture-region cropping.
- Requires cargo tests, release compilation and an in-headset check before publication. GitHub is not changed.

---

# vrperf v1.27.0 (development test fix)

- Correct the OpenGL texture-region unit test to use the wrist renderer's actual 144-pixel height, not the 160-pixel backing texture height. This matches the existing runtime call to `GlUploader::set_dimensions`.
- No changes to overlay layout, rendering, tracking, calibration, resource warnings or GPU upload behaviour.
- Requires the Rust test suite and SteamVR headset check before release.

---

# vrperf v1.26.0 (development fix)

- Fix watch row layout overflow by allowing enough pixel width for the longest reserved values, while keeping the watch's calibrated physical width and two-row design.
- Fix OpenGL texture region tests to use the actual maximum renderer dimensions. The head and wrist UV regions are checked against their respective dimensions.
- Keep the v1.25 resource warnings, frame-time colours, and calibration behaviour unchanged.
- Requires Rust tests and in-headset checks before publishing a stable release.

---

# vrperf v1.25.0 (development)

- Redesign the LEFT and RIGHT watch display with one information row above the graph and one below. The first row has integer FPS, CPU/GPU frame time, dropped frames and resolution; the second has headset battery (BAT), CPU/GPU utilisation, video memory (VRA), RAM and the clock.
- Position the clock on the bottom row to balance maximum text widths. Use fixed field widths so DROP 9 to 10 and other changing readings never shift adjacent labels. Align the graph with the text-row boundaries.
- Shorten the watch memory readouts to used/total GiB without G suffixes, with TiB notation for very large capacities. Remove unneeded controller battery, refresh-rate and numeric frame-budget fields from the watch. Keep the graph's budget line.
- Colour numeric CPU/GPU frame-time readings blue/green within budget and amber/orange above budget, matching their traces in both HEAD and watch modes.
- Apply orange utilisation and memory warnings from 85% and red warnings from 95%. Show battery warnings in orange at 6–15% and red at 0–5% remaining. Apply warnings to applicable HEAD and watch readings.
- Keep the watch's calibrated physical width and existing grip calibration, visibility and persistence behaviour. Reduce the watch panel's height and retain the full graph height within it.
- Add tests for layout spacing, changing field lengths, threshold boundaries and warning colours. Update the README while retaining both screenshots.
- The update needs an Arch Linux build and Steam Frame headset check before stable release.

---

# vrperf v1.24.0

- Change CPU graph traces from blue to amber/yellow only where their frame time exceeds the current budget.
- Change GPU graph traces from green to orange only where their frame time exceeds the budget. Lines switch colours at budget crossings, including between sampled frames.
- Keep the muted amber dashed frame-budget reference and red dropped-frame markers unchanged in both head and watch layouts.
- Add tests for CPU and GPU warning colours, exact-budget handling, crossing segments and dropped-frame marker priority.
- Reorganise the README around basic use, calibration, installation and updating first. Move hardware compatibility, troubleshooting and detailed technical notes below those instructions.
- Preserve the v1.23 calibration, controller bindings, visibility and hardware monitoring code. No saved settings or watch placements are changed.
- This development source requires Rust compilation and an in-headset display check before release.

---

# vrperf v1.23.0

- Start uncalibrated watches as visible flat panels 7 cm above their controllers (the earlier Above wrist placement). Previously saved per-hand calibrations remain unchanged.
- Remove the 200 ms angle-change debounce and keep immediate show/hide with angle hysteresis; no fade effect.
- Expand dashboard instructions: close the SteamVR dashboard to move a watch; grip the opposite controller, in-game or in SteamVR Home, then reopen to SAVE or CANCEL.
- Add best-effort SteamVR input bindings for Index, Quest/Oculus Touch, Vive, Vive Cosmos, Windows Mixed Reality and HP Reverb G2 controllers. Preserve the tested Steam Frame bindings.
- Add tests for initial uncalibrated watch placement and saving/cancelling per-hand calibration.
- Review legacy code and remove redundant visibility delay state and stale startup comments. No changes to performance rendering, graphs or hardware monitoring.
- Rewrite README with calibration steps near the top and explicit update instructions from v1.11 onward.
- Only Steam Frame input was tested in a headset in v1.22. The new controller bindings and changes in v1.23 require runtime checks.

---

# v1.22.0 (development source)

- Replaced legacy OpenVR `GetControllerState` grip/trigger polling with `IVRInput_010` actions.
- Included explicit Steam Frame `frame_controller` grip and trigger bindings for both hands.
- Saved action files under `~/.local/share/vrperf` and declared `action_manifest_path` in SteamVR app registration.
- Added diagnostics for binding activity, pressed state, and grab transitions.
- Preserved tracked-controller watch positioning, independent saved mount transforms, SAVE/CANCEL, and watch sizing.
- Added input and calibration state-transition tests.
- SteamVR headset test remains necessary; no SteamVR runtime was available in the build environment.

# vrperf v1.21.0

Development build for Steam Frame testing. Based on v1.20.0.

## Fix

- Correct four Rust test assertions in `src/overlay.rs` that used invalid `.9`, `.7`, and `.4` float literals. All values now have leading zeros. This fixes the `cargo test` compilation error reported for v1.20.
- No changes to overlay appearance, watch tracking, calibration mechanics, or hardware monitoring.
- Keep distinct SAVE and CANCEL dashboard controls: SAVE writes the selected watch calibration to disk, and CANCEL restores the previous position, orientation, and width.

The v1.20 functionality and testing limitations continue to apply. The Rust tests and Steam Frame controller calibration must still be checked on the user's Arch PC.

---

# vrperf v1.20.0

Development build for Steam Frame testing. Based on v1.19.

## Display modes

- Keep the existing fixed Head panel and its combined CPU/GPU graph.
- Replace Flat/Watch wrist style selection, Above/Below placement, and per-panel opacity controls with only Head, Left Watch, and Right Watch.
- Keep the compact graph and information fields for both watches.

## Interactive watch calibration

- Add independent left and right controller-relative watch transforms, saved in `watch_left` and `watch_right` under the vrperf configuration directory.
- Begin calibration with CAL LEFT or CAL RIGHT in the SteamVR dashboard. The selected watch stays visible during calibration.
- With the SteamVR menu closed, hold the **opposite** controller's grip or trigger and move/rotate it to adjust the watch. Release to keep the temporary pose; repeat for fine adjustments.
- SAVE writes the calibrated position, rotation, and size. CANCEL restores the previous values. Size controls adjust physical watch width by 0.5 cm while calibrating.
- Use the calibrated face direction for the 40°/60° automatic visibility test, rather than the fixed guessed controller angle.

## Restore fixed v1.11 opacity

- Restore straight RGBA backgrounds with fixed alpha 245/255, and set OpenVR overlay opacity to 0.93.
- Remove partial-opacity settings and the premultiplied-alpha overlay flag, which were associated with green text/border fringing.
- Remove watch fades to keep overlay alpha fixed.
- Return to linear OpenGL texture filtering as in v1.11.

## Testing

Static checks and archive integrity checks run at packaging time. This environment cannot compile the Rust source or test the Steam Frame controller bindings. On Arch Linux run `cargo build --release && cargo test`, then test both watch calibrations and save/restart behaviour before replacing the installed version. No GitHub publication in this build.
