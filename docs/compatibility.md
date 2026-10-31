# TV compatibility

> Lumaglass observations below are separate from the inherited upstream Luma reports. A successful first session does not qualify every feature or firmware.

## Lumaglass observations — 31 October 2026

Target observed over root SSH: **LG 65QNED756RA**, firmware **33.31.75**,
webOS 25 / SDK **10.3.2**, internal release **10.3.2-33**.
Lumaglass **2.9.0** was installed as a separate app. The owner confirmed the visible
interface and remote navigation, then confirmed that Home returns to Lumaglass
from an app or HDMI after enabling the manifest takeover.

The session exposed a mount-ownership diagnostic bug: this TV covers an overlay
at `/var` with an ext4 mount at the same path. The reader now uses parent mount
IDs to identify the covering filesystem, independently of mountinfo line order,
and refuses ambiguous candidates. Foreign/child/stacked Home mounts remain
conflicts. After the correction, the active mount reported `ownership: lumaglass`
and `conflict: false`. Disabling takeover restored `ownership: stock`, removed the
mount and disabled the boot hook before the corrected package was installed.

Corrected local ARM IPK SHA-256:
`a033222849193b4a35da6c2b467211a6c26393f2f6c9b4da29050bea8ecd1a01`.
The updated detector is included in both packaged scripts and the native TV-info
helper. Regression cases are assigned to CI; no local test suite was executed.

Cold-boot persistence, standby/wake, recovery after interruption, Magic Remote
pointer details, glass performance, privacy/blocker, cloud accounts, global app
visibility and system screensaver qualification remain pending. No new release
or broad model/firmware support claim follows from these observations.

## Inherited Luma reports

The following TV configurations have been reported working with Luma:

| TV        | Model identifier | webOS | Takeover Support |
| --------- | ---------------- | ----- | ---------------- |
| LG C8     | `OLED??C8*`      | 4     | ❌               |
| LG UM7450 | `UM74507LA`      | 4     | ❌               |
| LG UN7100 | `49UN71006LB`    | 5     | ✅               |
| LG C1     | `OLED48C1KNB`    | 6     | ✅               |
| LG C2     | `OLED42C2PSA`    | 25    | ✅               |
| LG C2     | `OLED65C2PSA`    | 26    | ✅               |
| LG C3     | `OLED65C31LA`    | 26    | ✅               |
| LG C3     | `OLED65C36LC`    | 26    | ✅               |
| LG C4     | `OLED42C4*`      | 25    | —                |
| LG C5     | `OLED55C57LA`    | 26    | ✅               |
| LG G3     | `OLED??G3*`      | 25    | —                |

> [!NOTE]
> Model identifiers containing wildcards are inferred: `??` is an unknown screen size and `*` is an
> unconfirmed regional suffix. `—` indicates information that has not been reported. `❌` means
> Luma runs on the TV, but Home takeover was reported not to work.

## Upstream implementation notes

Luma's native startup, rendering and app/input discovery have been verified on rooted webOS 26.
External testing confirms compatibility with webOS 6 and newer. On webOS 26 use elevated Homebrew Channel
0.8.0-rc1+ for `lunabus.query`. Root SSH execution does not establish the normal jailed caller
permissions; check TV-menu startup, launching/reopening and remote/pointer behavior manually.

[webOS Homebrew Can I Use](https://www.webosbrew.org/develop/caniuse) lists firmware library
versions. Its [library guide](https://www.webosbrew.org/develop/guides/library-version) maps
internal releases to marketing names: 7/22, 8/23, 9/24, 10/25, 11/26. SDL-webOS's newer
[`2.30.12-webos.6`](https://github.com/webosbrew/SDL-webOS/releases/tag/release-2.30.12-webos.6)
reports hotplug verification on webOS 3.4, 4 and 10. That is subsystem evidence for a newer
prerelease than the pinned `.5` library, not a full Luma compatibility guarantee.
