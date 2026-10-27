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

On QNED756RA/webOS 25, verify the dedicated control-center button, Up/OK, optional color binding and Back focus
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

Verify that the avatar opens the local profile on the current page. Edit/clear its name and avatar, finish text input by Done/Back/system dismissal, and confirm return focus. A static Home greeting must remain unchanged. Compare glass On/Off, text contrast, slider padding, endpoint labels and pointer snapping. Confirm that Back exits the timer detail before the full panel. Measure input latency and frame times on the TV; this revision has not been visually exercised or benchmarked.

## Optical glass

Check refraction on a detailed wallpaper, the moving selected-tab lens, sliders
and control-center tiles. Labels must remain sharp without reflected duplicates.
Compare glass Off and the supported optical/fallback paths. Verify stationary
backdrops report zero `glass_background_refreshes` while foreground clock/labels
change outside their coverage; wallpaper or relevant lower-content changes must
refresh the affected cache. Check no stale lens pixels with partial redraw.
At experimental 4K, confirm target texel memory stays at or below 15.82 MiB and
accept the deliberately HD optical backdrop while foreground text remains 4K.
Verify orientation, letterboxing, resized windows, modal fades, Recents, allocation
failure fallback and input latency on TV. These checks have not been executed.

## Favorites dock and interaction states

Enable the dock in Home personalization and check a short list is centered, a long
list scrolls inside its tray, and all densities preserve padding. Confirm captions
are hidden, their switches are locked, and the saved caption choice returns when
the dock is Off. Use
remote reveal, wheel, pointer, app options and reordering at both ends. Enable two
rows and confirm the dock switches off and is locked. Attempt activation by both
pointer and remote; it must remain Off. Disable two rows before re-enabling the
dock and confirm zero horizontal scroll. Restart to check persistence, compare glass On/Off,
and check inputs above/below the lower dock remain reachable. Verify hover, active
tab, remote focus and press are distinguishable without moving hit areas; compare
reduced motion. These checks remain pending.

Check Control Center over Settings, Home and detailed personal wallpapers: parent
labels must be subdued, while title, disabled statuses, icons and focus remain
readable. Check profile and app-options panels for consistent rows, text sizing,
opaque fallback and restoration of the current page. Verify the Xbox Series X
preset on a discovered input, its icon, local persistence and Clear name; confirm
that no port or connection state is fabricated. These checks remain pending.

Compare the clearer Control Center and floating profile over both detailed
wallpapers and Settings. Parent labels must remain subdued by the existing scrim,
while the profile has no outer container and each capsule refracts only the parent
backdrop. Check focus, pressed/opaque fallback, keyboard return and modal hit
areas. No new runtime visual check has been performed.
