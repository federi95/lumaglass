# Privacy controls and LG hosts blocker

## Privacy

Settings → System → Privacy retains the nine fixed observations. OK on Live Plus
or Limit ad tracking opens a confirmation when a boolean value was read from
the TV. Other consent records remain read-only. The QNED/webOS 25 adapter reads
those seven booleans from `eulaStatus`; legacy root-level records are retained.
Malformed or absent fields remain unknown rather than being treated as off. The adapter does not activate
ACR/ad services, accept agreements, edit consent files or kill processes.

Cancel is selected first. Apply rereads the exact category/key, refuses a changed
or unknown value, saves its original representation, writes only that key once
and verifies the result. No automatic retry follows a timeout or refusal.
Refresh after an unconfirmed result. Restore original value is available when
an owned backup exists; an unrelated externally changed value is not overwritten.
The root-owned journal lives in `/var/lib/lumaglass-controls/privacy.json`.
Desktop controls are unavailable rather than simulating successful TV changes.

## Blocker

Settings → System provides one LG ads and telemetry blocker row. It displays
the observed state and requests status when unknown. When known, selecting the
row offers the corresponding enable or disable/restore action with confirmation.
Entering the System category requests a read-only status snapshot.
The optional blocker is off until explicitly enabled. Its bundled minimal list
is revision `2026-10-lg-minimal-1`; the list and generation logic are inspectable
in [blocker.cjs](../tv/blocker.cjs). There are no downloaded lists or wildcards.
Exact domain references originate from Glasshouse (MIT; see NOTICE.md).

The adapter preserves the LG hosts contents beneath one owned bind mount,
including localhost and existing third-party lines. Foreign/stacked hosts mounts,
recognizable existing blockers, symlinks, unsafe ownership and conflicting boot
hooks are refused. A configured Glasshouse blocker startup flag is also refused.
Entries cover IPv4 and IPv6. It does not block general Store/update/ThinQ or time services.
Blocking these exact hosts cannot remove ads delivered from video origins.

Persistent files live in `/var/lib/lumaglass-blocker`, with the single startup
hook `/var/lib/webosbrew/init.d/65-lumaglass-blocker`. Boot checks the generated
file hash before mounting it. No renderer or LAN server needs to stay running.
An independent root-shell recovery command is:

```sh
node /var/lib/lumaglass-blocker/blocker.cjs disable
```

It removes only a verified owned mount/hook; the underlying hosts file is not
replaced. Before first enable, status is read-only. A failed enable may leave
its own journal/hook for explicit recovery; no mutation is silently retried.
If a helper is forcibly killed, its exclusive lock may remain. With Lumaglass
closed and no helper running, inspect the `lock/owner` PID before removing only
that app-owned `lock/owner` file and empty lock directory; no automatic lock
stealing is performed. Then rerun recovery. Existing application jails may retain earlier hosts bindings: effectiveness in
those jails, cold boot and firmware behavior need TV verification this weekend.
No TV changes were applied during development. System-wide app hiding/debloating
and consent writes remain separate from these controls.
