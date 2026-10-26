# TV compatibility

> These reports describe upstream Luma, not Lumaglass hardware verification. They are preserved from the fork baseline. Lumaglass requires separate TV verification before Home takeover.

## Compatibility

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

## Compatibility

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
