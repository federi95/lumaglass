# Native rendering and assets

Rendering design and measurement examples are inherited from Luma. TV observations below describe that upstream baseline, not hardware verification of Lumaglass.

## Splash images

`pnpm assets` renders one 1920×1080 PNG per background into `.build/assets/splash/` from the shaded
wallpapers. `tools/release/package.mjs` ships them under `splash/` plus `splash.png` (the default background),
which `appinfo.json` names as `splashBackground`, with `bgColor` set to the UI base colour so the
frame between the splash and the first draw is not black. On a normal TV launch and after successful
preference saves, the native platform worker passes the loaded background to the embedded
`tv/sync-splash.cjs` helper through Homebrew Channel's elevated `exec` service. It atomically updates
the installed `splash.png` and the matching enabled Home stage, skipping identical images. Desktop
and isolated `--preferences` / `--test-extra-apps` runs do not update launch splashes. The first launch
after installation or a package update can still show the packaged default until this sync completes;
later launches use the saved selection. With Home takeover enabled, `tv/home.cjs`
copies the selected background's splash into the staged Home app on every apply, reading the saved
preferences from the TV storage candidates in `storage.rs`. Production installations prefer internal
storage; developer installations prefer developer storage, with existing files taking precedence over
the legacy installation-local preferences. Reapplying an active Home mount also refreshes the splash.
`pnpm test:home` checks splash selection, runtime updates and mounted refreshes with a mocked filesystem
and no TV calls. Verify the system-rendered splash on a real TV by changing Background, quitting Lumaglass,
and relaunching its normal tile; desktop checks cannot establish SAM/compositor image caching behavior.

## Vector rendering

`assets/ui-icons/` holds standalone SVGs exported from the original Lucide/control artwork,
including the supplied PlayStation/Apple TV marks. `assets/logos/` holds original bundled logos.
`tools/vector-assets/` parses SVGs with usvg, resolves stroke outlines and transforms, and uses
the shared lyon tessellator to emit `.lvec` geometry. The format contains positions, edge offsets,
vertex colors and coverage; holes, round strokes and the bundled gradients are preserved.
Gradient colors are adaptively sampled at build time. The TV uploads each mesh lazily once and
draws with a texture-free GLES2 shader. No SVG parser runs on the TV. The shared tessellator also
handles font outlines. Installed app PNG/JPEG/WebP artwork, wallpaper photos, tile-background
images and the webOS installation icon remain raster assets. See [asset provenance](assets.md) for licenses.

TV and desktop windows default to 4× MSAA with 2×/single-sample fallbacks and no text
emboldening, restoring the text treatment used in v2.8.0. **Settings → System → Text rendering**
offers **MSAA (default)** and **Analytic**. The `text_rendering` preference (`msaa` or `analytic`)
is saved immediately and applies when Lumaglass next starts because SDL selects MSAA when it creates
the window. Missing or invalid preferences use MSAA. The setting shows a restart reminder.

Analytic mode uses a single-sample window and analytic text/logo edge filters. Text up to 16
physical pixels is emboldened by 0.25 px per edge, fading out by 28 px. The choice is visual:
analytic rendering was preferred on a contributor's webOS 5 TV, while MSAA was preferred in
manual Mali-G52 TV comparisons. `LUMAGLASS_MSAA=4` or `0` overrides the saved mode and its default
emboldening. `LUMAGLASS_EMBOLDEN` overrides the strength independently. Diagnostics report the
startup `text_rendering` configuration, and `last_presented.multisample_samples` reports the
actual window's samples. Offscreen captures can still be single-sample.

Shape edges use a one-drawable-pixel box filter centred on the edge, and rectangles (and logo
placement) snap to whole drawable pixels. A straight edge is then a single step, and a one-pixel
border is fully opaque. On TVs that scale a 1080p graphics plane up to the panel, half-covered
edge pixels would otherwise become soft two-pixel bands. `LUMAGLASS_SNAP=0` keeps logical positions
for comparison.

### Installed app artwork

TV inventory retains each app's `folderPath`, `largeIcon` and `icon`. Visible cards resolve artwork
from that metadata and the installed `appinfo.json`, preferring large icons and falling back to
the regular icon or `icon.png`. Developer, store and system application directories are supported,
including absolute/`file://` paths and relative resource paths. Curated vector brand logos retain
their existing display treatment; other apps use their own installed artwork and an automatically
sampled card colour. Missing, corrupt or unsupported artwork uses the existing generic treatment.

Discovery, bounded file reads, decoding and colour sampling run on a dedicated worker. If the
jailed app cannot read an asset directly, the native process calls Homebrew Channel's elevated
`exec` service with the embedded Node 8-compatible `tv/read-app-icon.cjs` reader. It returns bounded
image bytes without writing collected files or changing filesystem permissions. PNG, JPEG and WebP
are supported; runtime SVG parsing is not required. Source files are limited to 512 KiB and
2048×2048 pixels; decoded icons are at most 256×256 with a 16 MiB/256-entry CPU cache. GPU uploads
stay on the SDL thread and share the existing 48 MiB texture cache. Pending/unavailable icons do
not block input, and failures are cached until inventory is refreshed.

Inventory also refreshes at startup and whenever the window becomes visible. Decoded icons are
keyed by app ID, version and icon paths, so those refreshes keep them and retry only previous
failures; an updated app has a new version and is read again. **Refresh apps** (or desktop F5)
invalidates all artwork, including previous failures; request tokens discard stale replies. Check newly installed/updated apps and both normal tile and Home-takeover
launches on a real TV to verify sandbox access. These paths are covered with mocked filesystem
and worker tests, not live-TV calls in automated checks.

Dumped TV artwork, collected icon/colour maps, and their collection tools have been removed.
Desktop previews use curated vector logos or generic/category artwork for other demo apps.
Asset generation clears retired dumped-artwork outputs from older build caches, and packaging
continues rejecting those legacy files. Adding support for an installed TV app's icon does not
require collecting artwork or changing JSON.

### Curated brand cards and the desktop demo

`src/features/apps/branding.rs` contains the exact app-ID allowlist for 38 curated brand and utility cards. Titles and
partial ID matches do not select vector overrides. In particular, Amazon Alexa retains its own
installed icon rather than receiving Prime Video branding. NOW / Now TV recognizes `now.tv`
and `nowtv`; WOW, Sky Q and the separate Hong Kong Now TV service are not treated as aliases.
Unknown regional IDs continue using their installed artwork. Verify additional aliases against
that TV's inventory before adding them to the allowlist.

The aliases include developer fixtures and community webOS inventories, including
[FHEM's LGTV app IDs](https://github.com/fhem/LGTV_WebOS/blob/master/lib/FHEM/Devices/LGTV/LGTVWebOS.pm),
[a UK webOS inventory](https://github.com/chrispab/AutoHome/blob/master/items/CT_LGWebOS_TV.items),
and the [UK streaming catalogue](https://github.com/tjeffree/webOS-Streaming-Aggregator/blob/main/frontend/src/services/appCatalog.js).
These sources do not establish that every alias is available on every region/firmware combination.

Every curated logo has an app in `demo_inventory()`, including all 27 expanded streaming/music
brands. Open them with `pnpm dev --script apps`, then scroll or search the Apps grid. A unit check
keeps the SVG directory, curated catalogue and demo inventory aligned. `pnpm test:brands` renders
every card through the actual desktop renderer and writes `.build/brands-gallery.png` for visual
review. Like the other smoke scripts, it uses isolated preferences and no TV or real weather API.
The gallery also runs as part of `pnpm test:smoke`. Asset source/licence details are in [asset provenance](assets.md).

Live TV (`com.webos.app.livetv`) and Web Browser (`com.webos.app.browser`) also have dedicated
SVG-backed demo cards: a blue/cyan TV display with a live indicator, and a matching globe with a
navigation pointer. Both are original Luma vector artwork. The existing card caption remains available when
names are visible. `Brand::artwork_size()` sets optical sizing per brand, keeping heavy wordmarks
such as Hulu, Joyn and Plex closer to the visual weight of Apple TV, YouTube, Das Erste and ARTE.

Text reads the bundled TTF's advances, kerning and line metrics lazily. Only needed glyph outlines
are tessellated in font-em coordinates. Complete text runs are cached in static GPU buffers,
independent of position/color/drawable density, avoiding a draw call per letter. CJK fonts open
only when needed; there are no glyph bitmap atlases or full-font mesh exports.

MSAA mode requests 4× MSAA, with 2×/single-sample fallbacks if EGL rejects a configuration.
Analytic mode requests a single-sample window. Multisampled text draws
intact glyph fills without inward offsets/fringe triangles, which can cross on very thin
small-text strokes. Single-sample targets use a narrower half-pixel boundary fringe. Glyph
origins/baselines are pixel-aligned per drawable axis. There is no LCD subpixel rendering, font
hinting or complex-script shaping. Inspect `multisample_samples` and `font_antialiasing`:
offscreen captures can be single-sample while the actual window uses MSAA.

Glyph geometry has an 8 MiB/4096-entry CPU cache target and text runs a 32 MiB/512-entry GPU
target. Eviction preserves the current frame, which can temporarily exceed those targets.
Raster artwork has a 48 MiB cache budget. Diagnostics include geometry bytes, triangles, draw
calls and missing/failed artwork/glyphs. Idle screens draw on demand; VSync paces animation.

## Resolution and profiling

### Rendering requests and captures

The shipping default requests 1920×1080 and retains a 1920×1080 logical layout. Drawable-aware
vector coverage follows the actual SDL framebuffer. `--resolution 4k` is an **experimental**
native surface request, not a guaranteed panel-output mode. The TV compositor can constrain or
rescale it. A 4K screenshot or drawable alone does not prove 1:1 panel presentation.

```sh
pnpm dev --resolution 4k --profile-frames 180 --diagnostics .build/profile.json
pnpm dev --script 'home,down' --frames 90 --capture-scale 2 --screenshot .build/home-4k.png
```

`--resolution` also accepts `1080p` or `WIDTHxHEIGHT`. Explicit TV requests use a borderless
window, avoiding SDL desktop-fullscreen overriding the dimensions. `--capture-scale 2` uses a
separate 3840×2160 offscreen target and leaves the actual window unchanged. Font/vector geometry
uses that target's physical coverage rather than enlarging a 1080p image. Optional `@2x` wallpaper
images need both `pnpm assets --4k` and `node tools/release/package.mjs --4k`; default packages exclude
them even if they remain in an old cache. Source photographic detail still limits sharpness.

### 4K TV investigation

Manual testing on 2026-10-23 used an **LG OLED55C57LA (C5), webOS 26, chip `O22A3`, driver
`lg1kdrm`**. A contributor reported similar corruption on a C3 in
[#25](https://github.com/ecklf/lg-luma/issues/25). Usable native 4K UI presentation has not been
established on these tested TVs.

The C5's default EGLFS JSON reported `1920x1080`, and the active DRM state independently confirmed
a 1080p compositor framebuffer and output mode. The connector advertised `3840x2160`, and
`/proc/lg/fbdev/drm-helper/check_mode` reported `3840x2160: ok` (`7680x4320: err`). FB0 reported
`cfg.max_xres = 1920`, `cfg.max_yres = 1080` and virtual size `1920,2160`, consistent with two
stacked 1080p buffers rather than a 4K framebuffer.

The startup script uses Weston-style arguments and a `weston.ini` with `backend=drm-backend.so`
and `override-output-from-env=true`. Its device environment also generates the EGLFS JSON.
Inspect the actual startup path and DRM state; the JSON alone does not establish which output
configuration is active.

A temporary compositor-level test bind-mounted patched copies of `surface-manager.sh` and
`eglfs_starfish.env`, requesting matching 3840×2160 compositor and primary output geometry, then
restarted `surface-manager-daemon.service`. Systemd scheduled restoration after 90 seconds.

| Observation                  | Default            | Forced 4K test                  |
| ---------------------------- | ------------------ | ------------------------------- |
| Compositor framebuffer       | 1920×1080          | 3840×2160                       |
| Format                       | ABGR8888 with AFBC | ABGR8888 with AFBC              |
| Pitch                        | 7680 bytes         | 15360 bytes                     |
| Plane source and destination | 1920×1080          | 3840×2160                       |
| Active DRM mode              | 1920×1080 at 60 Hz | 3840×2160 at 60 Hz              |
| Visible picture              | Normal             | Flickering lines and corruption |

Restoring the original configuration returned the picture to normal. This was a system compositor
experiment, not a supported Lumaglass setting or installation procedure.

**Mode acceptance, a 4K drawable or screenshot, and even matching 4K framebuffer/plane/CRTC
dimensions do not prove correct panel presentation.** Check the visible output and use a
pixel-resolution test pattern before claiming 1:1 presentation. The corruption's exact cause is
unresolved: FB0's configured maximum and compatibility metadata alone do not establish a physical
hardware limit or explain the failure. These results do not prove that 4K graphics are impossible
on every LG TV; they establish that the tested override does not produce a usable picture on this C5.
An app-side resolution request alone does not resolve that display-path failure.

For read-only inspection in the TV's root SSH shell (paths depend on firmware):

```sh
cat /sys/kernel/debug/dri/0/state
cat /proc/lg/fbdev/status
cat /proc/lg/fbdev/drm-helper/status
cat /proc/lg/fbdev/drm-helper/check_mode
```

**4K video playback is a separate capability.** LG's
[app resolution specifications](https://webostv.developer.lge.com/develop/specifications/app-resolution)
distinguish 1080p graphics from 4K video on UHD TVs. Native media players can route decoded video
through a separate video plane while drawing their UI at 1080p. For example,
[PlxNative documents its default 1080p UI drawable](https://github.com/GLinnik21/plx-native/blob/13bed25004df4c89f8bcba9558b571b8ed962bd0/rust-modules/base/src/surface.rs#L10-L25)
alongside 4K video support. That media path does not demonstrate a usable 4K OpenGL graphics path
for Lumaglass; bypassing the browser does not by itself reconfigure LG's graphics output pipeline.

### Frame profiling

`--profile-frames N` forces redraws and measures swap-to-swap intervals after ten warmup
presentations. Hidden windows pause profiling. Capture stalls are excluded. CPU submission is
not a GPU timestamp, and `--frames` counts event-loop iterations, not presentation rate.
Inspect `window`, `drawable`, `drawable_is_4k`, `last_presented`, `capture` and `profile` in JSON.
Do not infer 60 fps from a successful VSync request; measure actual TV pacing and transitions.
`LUMAGLASS_PROFILE_SKIP=wallpaper,image,app,shape,gradient,shadow,text` (or `all`) leaves those draws
out, to find which part of a scene costs GPU time. On an LG webOS 5 TV (Mali-TDVX) the cost
tracks covered area: a full-screen layer takes about 7 ms at 1080p, and MSAA costs nothing
measurable.

### Partial redraw

Each frame redraws only what changed since the frame its back buffer holds. `src/render/damage.rs`
compares consecutive scenes command by command, realigning after inserted or removed commands
such as a focus ring; the union of changes since the buffer's frame (EGL buffer age) is the only
area cleared and drawn, with a scissor intersected with each clip, and commands entirely outside
it are not laid out. `EGL_KHR_partial_update` also tells the driver which tiles to keep (webOS
Mali); `EGL_EXT_buffer_age` alone still allows the smaller redraw. The EGL entry points come from
the libEGL SDL loaded, and only with a current EGL context, so GLX/CGL/WGL contexts always draw in
full. Unknown buffer ages, changed clip or fade state, a new drawable and screenshot captures draw
the whole frame, as does `LUMAGLASS_PARTIAL=0`. The startup log names the mode
(`Lumaglass partial redraw: …`).

`LUMAGLASS_DAMAGE_VERIFY=1` compares every frame with the same scene drawn in full offscreen at the
window's sample count and reports differing pixels in the diagnostics (`damage_verify`, with the
full frames as baseline). `--profile-frames` diagnostics include `redrawn_fraction_mean` and a
per-frame `timeline` of interval, compose, render and swap time and redrawn share.

Full-screen translucent fills drawn straight over the wallpaper (the Recents backdrop and
gradient) each map a pixel p to p * (1 - a) + c * a, so together they are p * m + k. The renderer
draws the wallpaper once with m and k per vertex (16 strips keep gradients' colour-alpha products
exact) instead of one full-screen pass per fill. During the Recents crossfade these fills sit
beneath the outgoing interface rather than over it, so they fold in while both fade.

### Idle preparation

The first visit to a page pays for laying out its text, outlining new glyphs, loading logo meshes
and uploading icons. After each inventory, once the screen has been still for 1.5 s, Lumaglass builds
the other pages' scenes without drawing them (`Renderer::prepare`), one page per idle wake; the TV
log reports each (`Lumaglass prewarm: …`). The script action `prewarm` triggers it at once, and the
profiling timeline records first-use work per frame (`glyph_ms`, `vector_ms`, `image_ms`).
