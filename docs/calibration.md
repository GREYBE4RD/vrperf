# Watch placement, calibration and RESET

[Back to README](../README.md#calibrate-a-watch)

## First-use headset-facing mode

A LEFT or RIGHT watch with **no saved calibration** starts about **12 cm above its tracked controller in standing space**. It stays visible and automatically turns to face the headset as you move your head or controller. This makes the panel easier to find than a fixed, controller-relative default that may point towards the floor.

The same headset-facing preview appears when you select **RESET** during calibration.

Once you **grip the overlay for manual placement**, automatic facing stops. The watch then stays at its placed position relative to the controller. After **SAVE**, normal watch visibility applies; it appears when you face and look at the watch, and hides when you turn away.

A saved calibration overrides the first-use default. Existing saved watch placements are not changed by upgrading vrperf.

## Calibrate either hand

Calibration works in SteamVR Home or in a running game.

1. Open the SteamVR dashboard and its **vrperf** tab.
2. Select **CAL LEFT** or **CAL RIGHT**.
3. **Close the SteamVR dashboard** using your controller's dashboard button. The watch stays visible while you calibrate.
4. Release the grip on the **opposite controller**, then press and hold it: right controller for the left watch, left controller for the right watch. Supported bindings may also allow the trigger.
5. Move and rotate the opposite controller while holding the grip. The watch follows it. Release the grip to leave the watch in place. Grab again to make further adjustments.
6. Reopen the dashboard. Use **-** and **+** to change watch width in 0.5 cm steps if needed.
7. Select **SAVE** to store the new position, angle and size.

You must close the dashboard to move the watch. Releasing the grip **does not save** the result.

## RESET, SAVE and CANCEL

| Control | Effect |
| --- | --- |
| **RESET** | Preview the headset-facing default for the selected watch. Keep its current width. Do not overwrite the saved calibration yet. |
| **SAVE** | Store the current position, angle and size for this watch. A RESET followed by SAVE saves the default orientation as a calibration. |
| **CANCEL** | Restore the position and size from before calibration began. |

RESET never changes the other watch. If you switch display modes while calibrating, the unfinished adjustment is cancelled. If SteamVR exits before SAVE, the previous saved calibration returns at the next start. A watch that has never been saved resumes first-use headset-facing mode after cancellation.

There is no keyboard shortcut for CANCEL. Use the dashboard control.

## When a calibrated watch appears

A saved watch uses separate face and gaze thresholds, so it does not flicker around one angle. The face-angle thresholds are about **40° to show** and **60° to hide**; gaze-angle thresholds are about **55° to show** and **70° to hide**. Visibility changes without a fade or delay. Tracking loss can also hide the watch.

An uncalibrated watch or a watch being calibrated remains visible while its controller is tracked.

## Controller support and bindings

The Steam Frame grip input has been tested. vrperf includes best-effort SteamVR action bindings for Valve Index, Meta/Oculus Touch, HTC Vive, Vive Cosmos, Windows Mixed Reality and HP Reverb G2 controllers. Those alternatives have not been tested in this project.

Index grip uses a pressure-sensitive grab input. SteamVR may automatically remap other controller types, but this is not guaranteed. A tracked controller can still lack a working grip or trigger binding.

If grabbing does nothing, open **SteamVR Settings > Controllers > Manage Controller Bindings**, select vrperf, and check the assigned actions. See [controller troubleshooting](troubleshooting.md#watch-calibration).

## Saved calibration files

The default user settings directory is `~/.config/vrperf/`. The files `watch_left` and `watch_right` hold independent calibration transforms. The `visible` and `mode` files hold the ON/OFF state and selected display mode. An `XDG_CONFIG_HOME` setting changes the base configuration directory.

The installer does not delete these files when upgrading or uninstalling. For an alternative way to clear one watch's saved calibration, see [manual recovery](troubleshooting.md#resetting-a-saved-watch).
