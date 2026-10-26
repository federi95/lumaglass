# App visibility and uninstallation

**Hide in Lumaglass** changes only the launcher's local preferences. The Hidden
filter can show those apps again. It does not edit LG manifests, mount files or
hide tiles in LG Home.

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

Kodi is the one explicit desktop simulation target. Its removal changes only
the preview's in-memory inventory; every dialog labels this as a simulation.
Restarting the preview restores its sample inventory. No desktop management
action calls TV services. Extra scroll-preview cards cannot be uninstalled.

The service methods, caller permissions and metadata coverage require manual
verification on each target firmware. These private TV endpoints are referenced
from [Glasshouse's app adapter](https://github.com/rorygallagher2024/glasshouse/blob/8e2521644f646ae6e6966f7ee06148a4daea0e41/server/lib/apps.js);
they are not a cross-firmware support guarantee. System-app deletion, forced
filesystem cleanup and LG Home tile hiding are outside this implementation.

Regression cases are included but have not been executed. Desktop/ARM builds,
lint and package inspection do not establish deletion behavior on a TV.
