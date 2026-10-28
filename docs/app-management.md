# App management, visibility and uninstallation

**Hide in Lumaglass** changes only the launcher's local preferences. The Hidden
filter can show those apps again. It does not edit LG manifests, mount files or
hide tiles in LG Home.

## Native app catalog

**Settings → System → App management** reads the unfiltered app inventory returned
by LG's app manager, including system and invisible entries omitted from the
normal launcher grid. It does not scan arbitrary filesystem locations or invent
missing TV apps. Access, root observation and SDK version are kept separate;
permission denied, unavailable and unknown values remain explicit. A readable
inventory is not proof of permission to mutate a TV service.

Use All apps, System apps and Hidden filters, then open an app to inspect its ID,
classification, protected state, reported system visibility and available actions.
The catalog reads on entry or Refresh, on the bounded observation worker, separate
from app launches and removal verification. Replies are capped at 1,000 entries,
require unique valid IDs and the LG Home anchor, and preserve unknown boolean
metadata. The view shows 20 entries per page and skips drawing offscreen rows.
A failed refresh retains the old report for inspection but does not authorize a
new hide or uninstall from stale metadata.

## Reversible local visibility

**Hide in Lumaglass** and **Show in Lumaglass** use the existing atomic preference
writer. They do not stop an app, disable services, free its background memory or
change its LG Home tile. Favorites and app order are preserved. The catalog only
creates new hides for non-protected apps present in the launcher inventory;
previously hidden entries can be shown again, including protected or missing IDs.

**Show all in Lumaglass** clears only Lumaglass's own Hidden preferences. It works
even when the catalog cannot be read, so local recovery does not require a working
TV service. **Undo local visibility change** restores the last catalog action
within this launcher session. It changes only entries whose values still match
the action's result and never re-hides protected identities. Changes made later
through other controls are preserved when they conflict. Undo history is not
persisted; the saved visibility choices and Show all remain available after restart.
It does not undo uninstallation.

## LG Home and system debloating

**Hide in LG Home** is shown as unavailable. No firmware adapter for this operation
is verified or enabled in Lumaglass, including for the QNED756RA/webOS 25 target.
The SDK number or root access alone cannot unlock it. This milestone adds the
catalog and capability boundaries; it does not implement system-app deletion,
service disabling, manifest overlays or a system-wide debloating preset.

The current [Glasshouse app adapter](https://github.com/rorygallagher2024/glasshouse/blob/d6905c4d84d5282e3a95ebe611c0493f88ac164c/server/lib/apps.js)
warns about picture, audio and HDMI failures around tile hiding on recent webOS,
and gates the operation behind an explicit setting. Its manifest bind mounts and
SAM restarts are not ported. Lumaglass performs neither these mutations nor repairs
to another launcher's existing overrides. Hardware investigation and a separately
verified reversible adapter are required before changing LG Home visibility.

## Confirmed uninstallation

**Uninstall app** appears in an app's options only when the inventory explicitly
reports `removable: true` and `systemApp: false`, with a recognised developer or
Content Store installation path. Missing, ambiguous or differently typed metadata
does not enable removal. Lumaglass, Luma/Home identities, Homebrew Channel,
Input Hook, known Glasshouse Home/dashboard identities and selected essential
LG apps are protected by exact IDs. A prefix alone does not establish removability.

The confirmation shows the app name and ID, with **Cancel** initially selected.
Before sending anything, a fixed elevated helper reads the full installed
inventory again and checks the flags, installation path and version against
the confirmed target. It selects one endpoint from the installation path:
developer `dev/remove` or standard `remove`. It never retries a mutation or
switches to another removal endpoint after an error. It does not delete files
directly, close unrelated apps or unmount another integration.

Known package identity is pinned as well. Protected package IDs and packages
declaring multiple app IDs are refused, since removal can affect the package.
The fixed helper also checks an eight-second intent expiry before querying and
again before mutating, so a request delayed by the elevated-service queue is
refused rather than applied long after its caller times out.

Request acceptance is not completion. A separate worker verifies the inventory
for up to 30 seconds, keeping input/rendering and launch requests responsive.
Only a successful complete inventory containing the LG Home anchor can prove
the selected app absent. Empty, malformed or denied replies stay unconfirmed.
An uncertain operation offers **Check status**, which only reads; closing its
window does not cancel an already submitted request. Reopen app options and
**Uninstall status** to inspect a pending operation. One unresolved removal is
tracked at a time within the launcher session; it is not automatically resumed
or resent after the launcher exits.

Favorites, local Hidden entries, recent entries and artwork cache results are
cleaned only after confirmed absence. Inventory generations discard older
responses; stale recent-history entries remain filtered until a fresh inventory
observes the app installed again. A rejected or unconfirmed removal leaves saved
preferences intact.

## Desktop preview and current limits

Desktop catalog data is explicitly marked as example inventory and reads no TV
services. Local visibility changes still use the selected desktop preferences.
Kodi is the one explicit desktop uninstall simulation target. Its removal changes only
the preview's in-memory inventory; every dialog labels this as a simulation.
Restarting the preview restores its sample inventory. No desktop management
action calls TV services. Extra scroll-preview cards cannot be uninstalled.

The service methods, caller permissions and metadata coverage require manual
verification on each target firmware. These private TV endpoints are referenced
from [Glasshouse's app adapter](https://github.com/rorygallagher2024/glasshouse/blob/8e2521644f646ae6e6966f7ee06148a4daea0e41/server/lib/apps.js);
they are not a cross-firmware support guarantee. System-app deletion and forced filesystem cleanup remain outside this
implementation. LG Home hiding now has a candidate transaction backend, but
remains disabled until exact firmware qualification.

New regression cases cover complete catalog validation, unknown/denied values,
stale generations, local visibility restoration, protected identities and removal
focus restoration. They run in CI after push; local test execution remains disabled by the owner.
Desktop/ARM builds, lint and package inspection do not establish TV compatibility
or deletion behavior. Hardware checks and release publication remain pending.

## Experimental global visibility preflight

The selected-app page offers **Inspect system visibility**, a separate read-only
check of the current manifest, firmware and conflicting mounts or queued
Glasshouse changes. **Restore LG visibility** is enabled only for an observed
Lumaglass-owned transaction that can be recovered. Neither action changes the
local hidden-app preference or invokes uninstall.

The candidate transaction backend is implemented, but **Hide in LG Home stays
unavailable** because no exact model/firmware profile is qualified. There is no
force switch or automatic boot reapplication. See [system app visibility](system-app-visibility.md)
for journal, rollback, standalone recovery and the remaining hardware checks.
