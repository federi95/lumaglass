# Lumaglass — provenance and third-party notices

Lumaglass is an independent fork of [Luma](https://github.com/ecklf/lg-luma),
starting from revision `8370058cacf7a0fa82b76847e65f08f5598db5e8` (version 2.9.0).
Its original launcher, renderer, translations, tooling and original artwork come
from Luma. The original **Copyright (c) 2026 Luma contributors** and complete MIT
permission and warranty notice are preserved verbatim in `LICENSE` and
`assets/licenses/luma.txt`. Lumaglass modifications are distributed under MIT.

This fork changes the product identity, package ID, source layout, documentation,
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
outside Lumaglass's product scope.

`tv/home-mounts.cjs` is an original Lumaglass ownership observer. Its known
Glasshouse Home startup paths are informed by `server/50-tvweb.sh` at that
revision; other projects' startup files are only observed, never modified.

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
