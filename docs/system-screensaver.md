# System screensaver

**Settings → General → System screensaver** is separate from Lumaglass’s local
idle timeout and local saver style. Opening the page performs read-only inspection.
It reports the observed model, firmware and SDK, verifies the stock saver and
installed native executable, checks ownership/symlinks and detects mount conflicts
and known active or queued Glasshouse integrations.

**System replacement is disabled in the distributed launcher.** No firmware profile
is qualified, including QNED756RA / 33.31.75 / webOS 25. Root access and a successful
fixture or desktop rendering check do not establish hardware compatibility. There
is no force switch, trusted profile supplied through JSON, or automatic installation.
The native player and reversible adapter are candidates for hardware qualification.

## Native player and settings

The candidate uses the existing Rust executable in a separate process, entered as
`lumaglass-screensaver` or with LG’s screensaver launch metadata before the launcher
initializes. Its surface ID is `com.webos.app.screensaver`. The staged native manifest
requests lifecycle interface 1 for SDL-webOS; permissions, registration, foreground
ordering, close/relaunch and wake behavior must all be verified before qualification.

Available styles are a moving dim clock and a dim local photo slideshow with that
clock. System style is saved separately from the app-local saver style. Application
captures language, clock format and the imported photo IDs at that moment; changing
these settings requires restore and a fresh apply. Changing a style does not silently
rewrite a running system integration. Photo mode requires imported images first.

Snapshots contain at most 12 images, at most 8 MiB per image and 32 MiB in total.
Only exact known Lumaglass cache roots and `personal-` IDs are accepted. No original
photo paths, titles, HTTPS URLs, cloud credentials or tokens are staged. Images are
copied into the staged app and readable there by the native app; the journal and
recovery entry remain root-owned. Snapshots and backups remain on disk after restore
for recovery; they are not automatically deleted. Review/remove them manually only
after the stock saver has been confirmed and no owned mount remains.

The player reads a bounded settings file, decodes one selected photo on a worker,
and falls back to the black clock view when a photo is missing or invalid. It uses
1080p, GLES2 on TV, the existing bounded texture cache and normally one redraw per
second, plus exposure/photo-load redraws. The clock moves every 15 seconds; the photo
changes every 30 seconds. It does not start launcher inventory/weather/HTTP workers,
load a launcher preference writer or use Liquid Glass, Qt, QML or a WebView. No TV
CPU/GPU or memory measurements are claimed from desktop CI results.

## LG policy and wake

**Request LG screensaver** asks `tvpower/power/turnOnScreenSaver` only while LG
reports `Active` (or reports the already-active saver). A rejected or unconfirmed
request never falls back to directly launching an app. A reported `Screen Saver`
state is an observation, not proof that a particular frame is visible.

The native candidate refuses TV startup unless LG reports exactly `Screen Saver`.
Power checks run away from SDL. Physical input requests `turnOnScreen` only from
that state, consumes the event and exits when LG leaves saver state. Unknown,
standby and screen-off states never authorize waking the panel. Cursor visibility
notifications do not wake it. Losing focus, hiding/minimizing or OS close exits
without a power request; no Home app or other app is launched or killed. LS2 call
errors retain the dim presentation and allow later input/retry; they are not treated
as a verified wake. The native LS2 role and exact power labels are qualification
requirements, not inferred from a root preflight.

LG retains its idle eligibility and panel protections. There is no global idle
timer, ASBL disable, hold override, `preferredType` modification or gallery/video
policy override. A custom clock/photo view is not a burn-in prevention guarantee.
See LG’s [screen saver guide](https://webostv.developer.lge.com/develop/guides/screensaver)
for the distinctions between application content and panel types.

## Transaction and recovery

A future reviewed profile must match exact model, firmware, SDK, stock manifest
`type`/`main` and the verified native lifecycle. Apply needs a short-lived explicit
intent and the current plan binding firmware, original manifest, installed ARM ELF,
configuration and photo hashes. Apply also requires LG to report `Active` immediately
before mounting; an already-running saver or unknown power state is refused. Confirmation selects **Cancel** first. Inspection
and unqualified apply create no state directories or locks.

The Node 8 adapter stages `/var/lib/lumaglass-screensaver/app`, an original manifest
backup, settings/photo snapshot, recovery script and atomic/fsynced journal. It binds
only that directory over `/usr/palm/applications/com.webos.app.screensaver`; stock
files are never edited or deleted. It proves its own mount by exact unstacked
mountpoint, source/target inode and device, staged manifest/config/photo hashes and
fixed executable/assets/library links. Ancestor or descendant mounts are conflicts.

Only the saver’s per-app `blockSystemApp` / `releaseBlockSystemApp` refresh is used;
`forceBlockSystemApp` is false when declared by the local schema. Block intent is
journalled before the request and release is attempted even after a timeout. SAM
metadata must explicitly match the expected saver type/main. A failed or uncertain
apply attempts ownership-checked unmount and retains a pending recovery result if
release or readback cannot be confirmed. There is no SAM restart, regional-list
rewrite, boot hook or automatic reapply after reboot/update.

**Restore LG screensaver** unmounts only a proven Lumaglass overlay. It does not write
a backup over an updated stock manifest. A changed firmware never authorizes a new
block sequence; an outstanding block owned by this journal can still be released.
A stale/malformed/live lock is refused; only a proved-dead owner PID permits lock
recovery, and no process is terminated. The standalone recovery entry is staged
before the mount, so the original helper remains available if the launcher is closed:

```sh
/usr/bin/node /var/lib/lumaglass-screensaver/screensaver.cjs restore
```

This command requires root and is for an adapter-owned transaction; it never removes
Glasshouse or another launcher’s integration. It is documentation, not a command
that development tooling runs on a TV.

## Qualification before enabling a profile

CI exercises only redirected filesystem/LS2/mount fixtures and desktop SDL rendering.
Hardware review must inspect stock manifest/permissions and power labels, verify
SDL lifecycle and native bus access, exercise clock/photo startup and every wake
path, check playback/HDMI exclusions and panel policy, restore after an interrupted
refresh and test reboot/update behavior. Keep root recovery access available. Record
results for the exact firmware before editing the source allowlist. Final hardware
audit, release and the major version bump remain separate work.

Protocol reference: [Glasshouse screensavers.js](https://github.com/rorygallagher2024/glasshouse/blob/d6905c4d84d5282e3a95ebe611c0493f88ac164c/server/lib/screensavers.js),
with its MIT attribution retained in [NOTICE](../NOTICE.md). The native implementation
has no dependency on Glasshouse’s dashboard, MQTT, Qt assets or panel overrides.
