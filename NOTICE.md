# Lumaglass — provenance and third-party notices

Lumaglass is an independent fork of [Luma](https://github.com/ecklf/lg-luma),
starting from revision `8370058cacf7a0fa82b76847e65f08f5598db5e8` (version 2.9.0).
Its original launcher, renderer, translations, tooling and original artwork come
from Luma. The original **Copyright (c) 2026 Luma contributors** and complete MIT
permission and warranty notice are preserved verbatim in `LICENSE` and
`assets/licenses/luma.txt`. Lumaglass modifications are distributed under MIT.

This fork changes the project identity, package ID, source layout, documentation,
local development setup and dependency pins. The original release history is
retained in `CHANGELOG.md`; inherited entries describe Luma releases. A reference
to an upstream hardware result does not establish Lumaglass hardware compatibility.

## Glasshouse

[Glasshouse](https://github.com/rorygallagher2024/glasshouse), by Rory Gallagher,
is a functional reference for the planned TV-management features. The upstream
Luma Home takeover documentation credits Glasshouse Home's bind-mount approach.
That reference is retained in Lumaglass's TV integration and architecture docs.
The complete **Copyright (c) 2026 Rory Gallagher** MIT notice is included in
`assets/licenses/glasshouse.txt`, using Glasshouse revision
`8e2521644f646ae6e6966f7ee06148a4daea0e41`.

This restructuring does not port Glasshouse's privacy, app-management, blocker,
screensaver, web dashboard or MQTT implementations. Planned functionality is
identified as such. Any future copied or adapted implementation must record the
upstream revision, affected files, changes and applicable notices here and next
to the derived implementation. Glasshouse's web dashboard and MQTT bridge are
outside Lumaglass's project scope.

`tv/home-mounts.cjs` is an original Lumaglass ownership observer. Its known
Glasshouse Home startup paths are informed by `server/50-tvweb.sh` at that
revision; other projects' startup files are only observed, never modified.

The original Lumaglass adapters in `tv/read-privacy.cjs` and
`src/features/privacy/` use fixed setting names and consent-file keys informed
by Glasshouse's `server/lib/privacy.js` at that revision. They implement bounded
read-only observations; Glasshouse's settings mutations, consent-planning and
blocker implementations are not copied.
The native Privacy page reports firmware records, not an assurance that a
tracking service is running or stopped.

`src/features/apps/management.rs` and `tv/manage-app.cjs` implement an original
Lumaglass removal workflow. Glasshouse's `server/lib/apps.js` at the same
revision informed the LS2 endpoint names and protected application identities.
Lumaglass uses explicit removable/non-system metadata, a pinned installation
target, one preselected request and subsequent read-only inventory verification.
The complete Glasshouse MIT notice remains distributed with the application.

The additional app-management review used Glasshouse revision
`d6905c4d84d5282e3a95ebe611c0493f88ac164c`, `server/lib/apps.js`, as a protocol and
protected-identity reference, including its tile-hiding compatibility caution.
`src/features/apps/catalog.rs`, `src/ui/view/app_management.rs` and
`tv/read-app-catalog.cjs` are original Lumaglass implementations. They introduce
read-only catalog observations and launcher-local visibility restoration, without
copying Glasshouse's manifest overlays, SAM restart or persistent hidden-tile
implementation. The protected-ID additions in `src/features/apps/management.rs`
and `tv/manage-app.cjs` follow that reference. The original Rory Gallagher MIT
notice remains preserved and included in the package.

## Liquid Glass optics

The optional single-sample Light path and 720p cache budget are original Lumaglass additions. The Full path retains the attribution below.

The optical section of `src/render/shaders/paint.frag` is adapted from
[ziuus/liquid-glass](https://github.com/ziuus/liquid-glass), revision
`c75d171e0626c78f4bdb5030191d2af6795cd892`, specifically
`glass-core/src/renderer/glass.wgsl`. The complete MIT notice, including
**Copyright (c) 2026 ziuus**, is preserved verbatim in
`assets/licenses/ziuus-liquid-glass.txt` and distributed with the IPK.

Lumaglass ports the lens magnification, index-of-refraction ray offset, chromatic
dispersion and directional rim/Fresnel ideas to GLSL ES 1.00. It substitutes
analytic normals for the upstream finite-difference height field, bounds sample
offsets, uses three backdrop samples and separates tint opacity from layer fading.
The native cached-backdrop integration is original Lumaglass code. Vulkan, wgpu,
Wayland capture utilities, media-widget drawing, continuous ripple physics and the
upstream multipass blur engine are not imported. This port is not Apple's SDK or
an assurance of pixel-identical Apple rendering or verified TV performance.

## Artwork, fonts, data and runtime libraries

The MIT license for the application does not replace the licenses of its assets
or dependencies. [docs/assets.md](docs/assets.md) retains the individual sources,
authors, attribution, modification descriptions and terms for wallpapers,
curated logos, icons, fonts and weather data. Original in-file SVG attribution
and font copyright/license tables are retained. The current app icon is reused
Luma artwork, not newly authored Lumaglass artwork.

- Inter and Noto fonts retain SIL Open Font License texts in `assets/fonts/`.
  Generated subsets retain their copyright/license tables and use derived family
  names. Original font binaries are unchanged.
- Lucide icons retain their ISC notice in `assets/licenses/lucide.txt`.
- Simple Icons retain their notice in `assets/licenses/simple-icons.txt`.
- SDL-webOS retains its zlib notice in `assets/licenses/sdl2.txt`.
- Wallpapers and logos retain their per-asset terms, including CC BY-SA for
  applicable derivatives. Compiling an SVG into `.lvec`, resizing an image or
  adding a scrim does not relicense that artwork under MIT.
- Open-Meteo data attribution and CC BY 4.0 terms remain in `docs/assets.md`.
- Third-party names and logos identify their respective apps; no endorsement
  by Luma, Glasshouse, LG, Apple or any app/asset owner is implied.

`tools/release/license-notices.mjs` collects declared Rust dependency licenses and
the license/copyright/notice files shipped with the exact locked crate sources.
Packaging includes that generated bundle, this notice, the original MIT license,
asset provenance and the bundled third-party license directory. Host-only build
tools are not included as TV runtime dependencies.

## Maintaining provenance

Preserve upstream notices when moving or modifying source files. Add provenance
for newly imported code/assets before packaging them. Keep source and generated
derivatives under their applicable terms; do not relabel all bundled material as
MIT. The packaging verification requires the notice files so they cannot be
silently omitted from the IPK.
