# System app visibility adapter

The native **System → App management → selected app** page now offers **Inspect
system visibility** and **Restore LG visibility**. Inspection is read-only and
independent of Hide/Show in Lumaglass. It checks the selected app against a fresh
SAM inventory, exact system paths, manifest identity, file ownership, symlinks,
mount conflicts and known queued Glasshouse overrides, and records the observed
model, firmware and SDK. A readable inventory or root access is not a mutation
capability.

**Global hiding remains disabled in the distributed launcher.** There are no
qualified model/firmware profiles, including QNED756RA / 33.31.75 / webOS 25.
The experimental backend and its rollback are implemented for qualification;
fixture-based CI tests do not qualify real hardware. For a qualified response, the native confirmation selects Cancel first and
sends the current plan only after explicit confirmation. The UI has no force-enable
switch. Requests cannot supply a trusted profile through JSON or environment
variables. Missing or conflicting metadata refuses the operation.

## Transaction and recovery design

`tv/system-apps.cjs` is an original Node 8-compatible adapter. A future reviewed
profile must match the exact observed model, firmware, SDK and individually
approved app IDs. Essential app IDs are always protected, even if a profile
lists them. A hide also needs a fresh intent and a plan fingerprint matching the
current manifest, identity and firmware; changing the manifest invalidates it.
Only `/usr/palm/applications/ID/appinfo.json` and
`/media/system/apps/usr/palm/applications/ID/appinfo.json` are candidate targets.

For a qualified candidate, the adapter stages the original manifest, a
`visible:false` derivative and an atomic/fsynced journal in the root-owned
`/var/lib/lumaglass-system-apps/ID` directory. Original app files are never
rewritten or deleted. It verifies ownership, inode/device identity, content hashes
and an unstacked exact mount before treating a bind mount as its own.

The candidate refresh uses only `blockSystemApp` / `releaseBlockSystemApp` for
that approved app, with `forceBlockSystemApp:false` when the local schema declares
it, then requires an explicit visibility readback. The transient block intent is
journalled before the call. Release is attempted even when the block reply is
lost; an unresolved release remains pending for recovery. There is no SAM kill,
service restart, configd regional-list rewrite or fallback to a stronger method.
Failed/ambiguous application attempts restore the owned mount and retain the
journal rather than reporting a successful hide.

Restore verifies ownership again before unmounting only that manifest. It never
removes another launcher's mount or writes a saved manifest over a changed stock
file. Firmware changes do not authorize a new refresh sequence. Incomplete
readback stays **Needs verification**; original visibility is not claimed merely
because unmount returned success. Cleanly restored records allow a later
qualified transaction. Unknown locks remain untouched; only a proven dead,
journalled lock owner can be recovered automatically. Liveness checks use signal
0 and do not terminate processes.

Before the first mount, a standalone copy of the adapter is staged in the state
directory. If the launcher cannot start, root-shell recovery for one owned entry
is independent of its UI or installed package:

```sh
node /var/lib/lumaglass-system-apps/system-apps.cjs restore APP_ID
```

The command refuses protected IDs, foreign/tampered state and conflicting mounts.
A pending result requires inspection; it is not a verified restoration. No boot
hook or automatic reapplication is added in this revision. Persistence across
cold boot remains a later, separately qualified integration. Hiding a tile also
does not promise lower RAM usage or termination of an already-running app.

## Validation status

CI fixtures redirect every file to disposable storage and replace LS2, mount and
unmount with an allowlisted fake. They cover the closed production gate,
protected IDs, root denial, foreign/queued mounts, stale plans, lost replies,
failed readback, pending release recovery, tampered journals and lock recovery.
Rust regressions cover reply identity, strict capability flags, stale responses
and the separation from local visibility and uninstall.

No local test or TV mutation was used to develop this adapter. The candidate
protocol and visible result still require hardware qualification before global
hiding can be enabled. Screensaver replacement and cloud photo connectors remain
separate future work.

The protocol reference is Glasshouse's pinned
[`apps.js`](https://github.com/rorygallagher2024/glasshouse/blob/d6905c4d84d5282e3a95ebe611c0493f88ac164c/server/lib/apps.js)
and [`samrescan.js`](https://github.com/rorygallagher2024/glasshouse/blob/d6905c4d84d5282e3a95ebe611c0493f88ac164c/server/lib/samrescan.js).
The firmware warning about picture/audio/HDMI failures remains relevant; upstream
observations do not establish compatibility of Lumaglass's new adapter. Attribution
and the complete original MIT notices remain in [NOTICE](../NOTICE.md).
