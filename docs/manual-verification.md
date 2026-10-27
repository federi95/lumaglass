# Pending TV verification

Target supplied by the maintainer: LG QNED756RA, webOS 25, firmware 33.31.75,
rooted installation through Homebrew Channel. No hardware result is recorded yet.
CI covers host checks/rendering and ARM packaging, not firmware compatibility.

For the planned weekend session, record the exact commit and package checksum:

1. Install/start the native app and check remote/pointer navigation, long greeting,
   two-row favorites, density, accent, dimming/blur and language selection.
2. Check saved preferences after restart and standby; verify the four color-button
   reception indicators and shortcut scope. Retain independent Home recovery.
3. Check original privacy observations first. Change one supported key, refresh,
   verify it in LG settings, and restore it. Unknown/denied keys must remain explicit.
4. Inspect blocker status for foreign mounts before enable. Check normal services,
   disable/recovery and cold boot. Record which app jails observe the hosts change.
5. Import local/HTTPS images; exercise invalid/oversized input, offline cache,
   fitting, rotation, startup selection and splash updates. Preserve original files.
6. Check local saver timeout and waking input; also verify LG standby and panel
   protection while Lumaglass, another app and HDMI are foreground separately.
7. Inspect requested/drawable/display sizes at 1080p. If trying 4K, first allow an
   unconfirmed trial to revert, then confirm a later trial. Compare actual panel
   text quality, memory, startup time and frame pacing. Keep recovery accessible.

The final release audit, release readiness decision and 3.0.0 publication remain
milestone 8. No item above is marked passed by compilation or CI alone.

## Control center and header

On QNED756RA/webOS 25, verify avatar, Up/OK, optional color binding and Back focus
restoration. Check separate/combined clock layouts on both sides, 12/24-hour time,
seconds and empty/long greeting names. Confirm that local sliders work with remote
steps and Magic Remote drag without activating another control on release.

Check network observations without connecting/disconnecting anything. Verify mute
and unmute with readback, including external audio output. With a safe viewing
session, confirm one standby request and inspect actual power state after wake.
For the native sleep timer verify an available short slot, cancel through Off, and
confirm LG's own timer UI agrees. Missing permissions must produce unavailable
state, not a fabricated success. Start the local saver with automatic timeout Off, including while the panel’s
read-only status refresh is pending, and ensure its wake gesture is consumed. Confirm that these entry points do not
intercept Home while another application is active.
