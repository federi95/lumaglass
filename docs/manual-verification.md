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
Compare Off, Light and Full on the supported optical/fallback paths. Confirm
legacy booleans preserve their saved choice and each mode persists after restart.
Check Light retains readable transparency without colored edge separation, while
Full retains the existing lens effect. Compare presentation p95, frame misses and
input response on the same transitions, independently at 1080p and experimental
4K; do not interpret CPU submission time as GPU execution time. Verify stationary
backdrops report zero `glass_background_refreshes` while foreground clock/labels
change outside their coverage; wallpaper or relevant lower-content changes must
refresh the affected cache. Check no stale lens pixels with partial redraw.
At experimental 4K, confirm target texel memory stays at or below 7.03 MiB in
Light and 15.82 MiB in Full, shrinks when switching Full → Light and reaches zero
after rendering Off. Accept the deliberately 720p/1080p optical backdrop while
foreground text remains at the main drawable resolution. Check no stale lenses
when switching quality with partial redraw enabled.
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

## App catalog and reversible local visibility

Open System → App management on the TV and record SDK, root observation and actual
inventory access separately. Check system/invisible entries and unknown flags
against read-only system observations; never infer mutation rights from a readable
list. Check page/filter navigation, long names, protected entries and both pointer
and remote focus. Hide one non-essential launcher entry locally, restart, show it
again, and verify favorites/order and LG Home are unaffected. Exercise Show all
and session Undo, including a conflicting later visibility choice. Catalog read
failure must keep Show all available and block actions relying on stale metadata.

LG Home hiding must stay unavailable; this revision must not mount app manifests,
restart SAM, disable services or change other launchers. If testing the established
uninstall path later, use one explicitly disposable user app, verify identity and
Cancel as the initial choice, then check confirmed absence and catalog/focus update.
Do not treat a request acknowledgment as successful removal. These checks and
the new regression suites have not been executed.

## Global visibility preflight and qualification

Before any qualification attempt, open the selected-app inspection and compare
its model/firmware/SDK and manifest identity with read-only TV observations.
Protected/user apps and foreign app-directory/manifest mounts must refuse global
changes. Queued Glasshouse hidden-app entries or overrides must show a conflict.
Normal builds must keep Hide in LG Home disabled on every unqualified firmware.

The transaction engine is covered by isolated CI fixtures, not hardware approval.
A reviewed candidate profile still requires a separate controlled hardware session
for manifest/SAM readback, failure rollback, interrupted release recovery and
standalone restore, including picture, audio and HDMI after standby/wake. No SAM
restart or automatic boot hook may be used as a fallback. See
[the adapter design](system-app-visibility.md). These checks have not been run.
