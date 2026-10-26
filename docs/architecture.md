# Lumaglass architecture

Lumaglass is a native Rust/SDL2/OpenGL ES 2 application for rooted LG webOS TVs, derived from Luma. It combines a Home replacement with planned TV-management features inspired by Glasshouse. There is no web dashboard, WebView or MQTT bridge. See [NOTICE.md](../NOTICE.md) for provenance.

## Source boundaries

```text
src/main.rs                 executable entry point and internal module aliases
src/app/                    application model, state and SDL runtime
src/ui/                     native scene composition and translations
src/render/                 GLES renderer, shaders, vector/font geometry and damage
src/features/apps/          artwork, branding, recent apps and preview inventory
                            and typed, verified app-removal state
src/features/privacy/       fixed privacy observations and transient request state
src/features/home/          local Home customization and validated preferences
src/features/appearance/    shared card geometry and local visual preferences
src/features/weather/       forecast scheduling, HTTP worker and cache
src/features/system/        typed read-only TV observations and request state
src/platform/               inventory/actions, SDL input and installation lookup
src/platform/webos/         LS2, native Home lifecycle and libc compatibility
src/storage/                local preferences
crates/vector-geometry/     shared vector geometry crate
tools/vector-assets/        host SVG compiler crate
tools/{dev,assets,webos,release}/  host-only development and packaging tools
tv/                         helpers executed on the TV
assets/                     original artwork, fonts, translations and notices
```

The root module aliases preserve existing internal names during the mechanical
move. Rendering algorithms remain inherited from Luma.
New feature modules are added when they have an implementation; empty privacy,
blocker or system-service scaffolding is not shipped. A separate service crate
is deferred until a second executable actually needs shared code.

Input/rendering and GPU uploads stay on SDL. TV calls, HTTP and icon decoding
stay on their workers. Host SDL and the soft-float ARM SDL-webOS runtime remain
isolated. The `.lvec` magic `LUMAVEC1` is intentionally retained as a format
identifier, independently of project branding.

The installed application is `org.lumaglass.app`, executable `lumaglass`.
Preferences, Home markers, boot hooks and diagnostics use Lumaglass-specific
paths. Upstream Luma IDs remain excluded from inventory and recent apps.

## Read-only TV information and clock

Settings → System → TV information requests a snapshot only when opened or
explicitly refreshed. A separate bounded platform queue keeps these reads out
of the launch queue. The embedded Node 8-compatible helper uses fixed, bounded
reads of `getSystemInfo` and `/proc/self/mountinfo` through the existing native
Homebrew Channel connection. It does not change settings, hooks or mounts.
Missing fields and unknown/denied access stay explicit; root access does not
promise permission to perform future mutations. Late generations are ignored.
Desktop preview uses no TV services and labels the values as not applicable.
See LG's [device information API](https://webostv.developer.lge.com/develop/references/tv-device-information).

The clock refreshes its OS timezone offset once a minute and on foreground
events. A dedicated worker starts a bounded, single-threaded invocation of the
same executable with `--local-offset`, handled before SDL or any workers start.
This preserves `time`'s Unix thread-safety safeguard. Failed lookups keep the
previous offset; fixed `--time` captures do not start this worker.

`tv/home-mounts.cjs` is the shared, read-only mount observer used by both the
native information helper and takeover/recovery scripts. It reads at most
256 KiB, decodes mountinfo path escapes and checks all relevant layers, including
child mounts. A valid own takeover has exactly one mount and one method.
Mount roots are resolved relative to their containing filesystem and device,
including a separate or symlinked `/var`, as described by the
[kernel mountinfo format](https://www.kernel.org/doc/Documentation/filesystems/proc.txt).

Before enabling/reapplying takeover, the same reader checks the known Luma
Home hook and Glasshouse's `home.json` enable flag, including an unreadable-state
outcome. The shared Glasshouse boot hook alone does not imply a Home takeover.
Other integrations' files are never changed; Lumaglass recovery remains available
when a different integration is merely configured but has no conflicting mount.
This is detection of known integrations, not a scan of arbitrary boot scripts.
Foreign, stacked or mixed-method mounts prevent mutations. The observer is
embedded alongside the native information helper and packaged/copied with the
standalone recovery scripts; it does not depend on the renderer remaining open.

## Privacy observations

System → Privacy reads only on entry or manual Refresh. Its typed jobs share a
bounded observation worker with TV information, independent of app launches and
SDL. Two fixed `getSystemSettings` queries read `option/livePlus` and `general/lmt`
with 1.5-second process budgets and 64 KiB replies. A bounded 256 KiB read of
`/var/luna/preferences/eula` extracts only seven allowlisted JSON boolean fields.
Unrecognised shapes/types remain unknown. The general read API is documented by
[LG](https://webostv.developer.lge.com/develop/references/settings-service);
these particular keys and consent storage are firmware-specific observations
informed by Glasshouse, not a public cross-firmware API guarantee.

No advertising ID is requested. No ACR/admanager endpoints are queried, no
settings/consent writes occur and consent contents are not logged or stored in
preferences. Generation IDs discard late results and a failed refresh retains
the previous snapshot with a notice. Consent flags are not evidence of service
activity or effective data collection. The UI uses neutral labels, including
the opposite meaning of enabled Limit ad tracking versus granted consents.

## Home layout

`src/features/home/` owns typed local Home preferences with independent defaults
for malformed fields. Header composition follows greeting/date visibility and
order; favorites and inputs are laid out as separate measured sections. Remote
routes follow the displayed positions. Startup-focus targets carry their typed
identity through the selector and are checked against current visible Home cards;
the initial inventory can focus one without emitting a launch effect.

Home favorites share row/column rules in `src/features/apps/home_layout.rs` for
scene composition, scroll bounds, remote focus and reordering. The optional
two-row layout uses the saved order and divides visible favorites evenly across
at most two rows. Missing preference fields keep the
single-row layout. Layout changes affect local preferences, not TV app state.

## Font parsing and geometry

The renderer uses [Skrifa 0.49](https://docs.rs/skrifa/0.49.0/skrifa/)
and its `read-fonts` model instead of `owned_ttf_parser`/`ttf-parser`.
Each loaded font owns its bytes and one unhinted scaler. Outlines are requested
per glyph in font units, converted to em-space geometry with the existing Y-axis
flip, then tessellated with the existing nonzero fill rule and subpixel tolerance.
Glyph geometry, font weights and CJK/Hangul fallback remain demand-loaded and
cached by the renderer; no whole-font outline scan runs at startup.

The adapter validates units per em, glyph count, horizontal metrics and the
presence of supported outlines before accepting a generated font asset. It uses
the font's horizontal line metrics and advance widths, preserving the first
matching horizontal legacy `kern` subtable rule. Cross-stream/vertical kerning
and state-machine kerning are ignored. This migration does not introduce GPOS
shaping, hinting or different antialiasing; the existing MSAA/Analytic choice
continues to apply. Autohint shaping is disabled in Skrifa's Cargo features.

Desktop/ARM compilation and package verification cannot establish visual parity.
Text baselines, spacing, curves and CJK/Hangul fallback still need a visual
comparison in both rendering modes and on the target TV. Rendering smoke tests
have not been run for this migration.

## Persistent privileged operations

App removals use their own bounded worker, separate from inventory, observations
and interactive launches. The UI captures an immutable removal target in the
options/confirmation flow. The fixed elevated adapter revalidates metadata and
sends exactly one request; subsequent jobs only verify inventory. Completion
invalidates earlier inventory generations and filters stale recent entries,
then persists local preference cleanup and invalidates artwork results. Desktop
removal uses an isolated in-memory inventory with one explicit Kodi simulation.
See [app-management.md](app-management.md) for the policy and completion rules.

The current platform adapter uses the native LS2 caller and Homebrew Channel's
elevated fixed helpers. Future privacy, blocker and screensaver integrations
must expose typed, bounded operations with detected capabilities. Boot hooks
and persistent settings must not depend on the renderer remaining open. A
separate local service is introduced only for operations that need one; it
must not expose a general shell or a LAN administration server.

See [rendering.md](rendering.md) for GPU/text/asset behavior and
[tv-integration.md](tv-integration.md) for takeover and recovery.

## Storage, weather and languages

TV preferences live in an app-owned persistent directory outside the installed executable's
payload. Startup probes `/media/developer/<appid>/`, `/media/developer/temp/<appid>/` and
`/media/internal/.<appid>/`, selecting a directory the jailed app can actually create and write.
Production `/media/cryptofs` installations prefer internal storage. `/media/developer/temp` is
on the persistent developer partition, not the reboot-cleared `/tmp`. No settings are placed
in `/tmp` and no system/app-payload permissions are widened. Existing JSON preferences and a
valid weather cache from beside this installation's old executable are imported once, without
overwriting the chosen destination. Invalid existing preferences still report an error.
Startup prints `Lumaglass persistent storage:` with the chosen path. Desktop preview retains the legacy
`$XDG_CONFIG_HOME/lumaglass-native/preferences.json` path, or `~/.config/lumaglass-native/preferences.json`,
so choices from earlier Lumaglass previews survive updates. Luma preferences are
not imported. `--preferences PATH` isolates a manual run.
Invalid preference files report an error rather than being overwritten. A single
preference worker coalesces pending snapshots and flushes the last save on shutdown.
Preferences and weather cache writes use unique same-directory temporary files,
file sync, atomic rename and Unix parent-directory sync. Only the newest save
completion affects the UI and splash; errors remain visible. Use one instance
per preferences file. Browser local storage is not
migrated into native JSON preferences.

Weather Location searches cities/postcodes only on explicit Search. Forecasts run when Home is
visible, the widget enabled and a city configured. ureq/rustls uses embedded certificate roots,
an eight-second timeout, no redirects/proxy and 256 KiB reply limits. Search/forecast/cache writes
have separate workers and generation IDs, avoiding stale responses overwriting current state.
Fresh Celsius readings are reused for 15 minutes; valid saved data lasts up to six hours and
failure retries after five minutes. Unit changes convert unrounded cached Celsius immediately.
Removing a city clears its cache. The `.weather.json` sidecar uses Unix-seconds timestamps.
Mock servers accept only literal loopback HTTP origins and are refused in TV builds.

`assets/messages.json` supplies English, German, Spanish, Simplified Chinese,
Portuguese, Italian, Russian, Polish and French, including TV information labels.
App/device names and saved city names remain real data. No geolocation,
runtime font/translation services or telemetry are used. Weather attribution: Open-Meteo,
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
