# Licensing and attribution

Lumaglass retains Luma's MIT license verbatim in [LICENSE](../LICENSE), including
the original copyright, permission condition and disclaimer. The original notice
is also copied into `assets/licenses/luma.txt`. Renaming or moving a file must
not remove its source notices or attribute upstream work to the fork's authors.
See [NOTICE.md](../NOTICE.md) for the starting revision, original projects and
the changes made by the fork.

Glasshouse is MIT-licensed, Copyright (c) 2026 Rory Gallagher. Its complete text
is preserved in `assets/licenses/glasshouse.txt` alongside the inherited credit
for the Home takeover approach. No Glasshouse management feature implementation
is imported by the reorganization. Future code imports must record their
upstream revision and modifications and retain the applicable notice.

## Asset and font terms

[assets.md](assets.md) is the per-asset attribution record, inherited from Luma
and kept with the package. Original SVG metadata, font copyright/name tables and
bundled license files remain intact. Generated font subsets retain license
tables and derived family names; original font binaries are unchanged.

Source-specific terms continue to apply to derived `.lvec`, wallpaper, splash
and thumbnail files. CC BY-SA artwork is not relicensed under the application's
MIT license. Attribution records identify source authors, URLs, changes and
license links. Third-party brand rights remain with their owners; a source
credit alone does not grant trademark rights or certify that every brand asset
has a general-purpose redistribution license.

## Rust and native dependencies

`tools/release/license-notices.mjs` reads the locked ARM webOS graph and preserves
original license/copyright/notice files from the fetched crate sources, including
nested notices. Packaging fails for a dependency with no declared license or
available text. Crates whose published tarballs omit license files use explicitly
versioned, source-pinned notices under `assets/licenses/rust/`, with URLs and hashes.
A version change must refresh those records rather than silently reuse a notice.

The generated inventory records names, versions, declarations and repositories;
its text bundle preserves original notices. Build-only crate notices may also
be included. Node/pnpm, Pixi and host dependencies are not copied into the TV IPK.
The bundled ARM SDL-webOS library retains its separate zlib notice.

## What travels in the IPK

- `LICENSE`: unchanged upstream Luma MIT license.
- `NOTICE.md`: fork provenance, Glasshouse reference and derivative notices.
- `ASSETS.md` and `docs/assets.md`: per-asset attribution and license links.
- `docs/licensing.md`: how distributed notices are maintained.
- `licenses/`: original asset/runtime/project texts and pinned notices.
- `licenses/rust-dependencies.txt` and `.json`: locked Rust inventory and texts.
- Font license files and preserved license tables in generated fonts.

Package verification requires these files. This work does not publish packages
or modify third-party project repositories.
