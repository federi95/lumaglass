# Lumaglass feature highlights

Native screenshots of Lumaglass **2.9.0**, captured in
[CI run 41109837133](https://github.com/federi95/lumaglass/actions/runs/41109837133)
from revision [`7565a71`](https://github.com/federi95/lumaglass/commit/7565a7114cfa2db45ecdbb0bc43c6e6ea448d536).
These show the actual Rust/SDL/OpenGL interface, rendered on Ubuntu 24.04 under
Xvfb with software OpenGL. Apps, inputs and source folders/albums are demonstration
data. No personal account or TV was connected. TV firmware compatibility and
performance still require hardware verification.

## Home and apps

![Home with an optional clear favorites dock, combined clock/avatar and separate control-center button](home-dock.webp)

Choose the favorites dock or two rows of favorites. The dock hides app captions;
enabling two rows disables and locks the dock. Clock placement, greeting/date,
section order, initial focus and ordinary favorites captions are configurable.
See [Home personalization](../../home-personalization.md).

![App grid with categories, search, favorite markers and remote focus](apps.webp)

Browse apps by category, search, reorder favorites and manage visibility through
app options. Eligible user-app removal requires confirmation. Global LG visibility
adapters and system takeover remain subject to firmware qualification; this app
grid is not evidence of those operations on a TV.
See [app management](../../app-management.md).

## Quick controls and local profile

| Control center                                                                                       | Local profile                                                        |
| ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| ![Floating control-center lenses with shortcuts and a wallpaper-dimming slider](control-center.webp) | ![Centered local profile with name and avatar options](profile.webp) |

A dedicated header button opens and closes the Control Center. Back or a click on
its background dismisses it. The avatar opens a separate local profile; it does
not sign into an LG or streaming account. TV-only fields correctly show **Not
applicable** in these desktop captures.
See [control center and header](../../control-center.md).

## Settings and wallpapers

| Contextual settings                                                                        | Wallpaper gallery                                                                                 |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| ![Two-pane settings with a contextual illustration and focused General row](settings.webp) | ![Wallpaper gallery with a large preview, saved choice and personal-image entry](wallpapers.webp) |

Settings use remote-friendly lists, contextual previews and separate focused/saved
choice indicators. Wallpaper choices include local images, explicit HTTPS imports,
offline photo imports and optional rotation.
See [appearance](../../appearance.md) and [personal wallpapers](../../wallpapers.md).

## Appearance with and without glass

| Liquid Glass Full                                                                                | Liquid Glass Off                                                                                    |
| ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- |
| ![Appearance settings with clear header controls and padded value sliders](appearance-full.webp) | ![The same appearance settings with opaque controls and Liquid Glass disabled](appearance-off.webp) |

Off, Light and Full are user choices. Sliders support remote steps and pointer
dragging, with a visible current value and endpoints. Off removes optical sampling
and releases dedicated glass targets on the next render. The screenshots illustrate
appearance; they do not measure GPU speed or RAM on a TV.
See [Liquid Glass](../../liquid-glass.md) and [runtime resources](../../runtime-resources.md).

## Explicit photo imports

These four source screens intentionally use **demo mode**. Remote connection,
import and removal actions are disabled in the captures; folder/album selections
illustrate navigation without contacting a service. They are not captures of a
successful personal-account connection or an imported personal library.

| Immich                                                                                               | Nextcloud / WebDAV                                                                |
| ---------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| ![Demo Immich album selection with a selected album and offline-import guidance](photos-immich.webp) | ![Demo WebDAV folders with explicit load and import controls](photos-webdav.webp) |

| Google Drive                                                                             | Google Photos local export                                                                               |
| ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| ![Demo Drive folder selection with explicit load and import controls](photos-drive.webp) | ![Google Photos screen describing computer-side export and offline TV import](photos-google-export.webp) |

Immich, WebDAV and Drive use explicit, read-only source operations. Google Photos
uses a user-initiated export on a computer, followed by local import on the TV;
it does not provide automatic Google Photos library synchronization. Imported
photos can be used offline. No provider is polled at launcher startup.
See [Immich](../../immich.md) and [photo-service setup and boundaries](../../photo-services.md).

## Capture provenance and reuse

All 12 images are full-frame **1920×1080 lossless WebP** conversions of CI PNGs.
There is no resizing, compositing, retouching or replacement of UI content.
[manifest.json](manifest.json) records the source revision/run/artifact/filename,
original PNG SHA-256, published WebP SHA-256 and dimensions for each image.
Original CI artifacts have limited retention; the published files and provenance
remain in this repository.

Lumaglass UI code is MIT-licensed with upstream Luma notices preserved.
Wallpapers, fonts, icons and visible third-party marks retain their own terms;
these captures do not relicense them. See [asset provenance](../../assets.md),
[NOTICE](../../../NOTICE.md) and [licensing](../../licensing.md) when reusing images.
Streaming and console marks identify example app/input entries, without implying
affiliation or endorsement. These are Lumaglass captures, separate from the
[inherited Luma media](../README.md#upstream-luma-media).
