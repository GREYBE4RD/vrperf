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
