# Personal wallpapers and rotation

Open Settings → Background → My images, beneath the bundled gallery. Enter an
absolute folder path visible to Lumaglass, an explicit HTTPS image URL, or both.
Select Fit inside or Fill and crop, then Import / refresh images. Changing the
fit applies on the next import. OK on an imported image previews and selects it
behind Settings. Restore bundled wallpaper returns to the saved builtin image;
Clear folder and URL removes the source settings but keeps the offline cache.
The source location may need adjustment to match the app jail's visible paths.

The folder is scanned one level deep (at most 512 entries), in filename order.
PNG, JPEG and WebP files are supported; symbolic-link image files are skipped.
Imports include at most twelve images plus a retained current image. Input is
limited to 8 MiB, 24 million pixels and 8192 pixels per dimension, with a decoder
allocation limit. EXIF orientation is applied before fitting. Acquisition,
decoding and soft/strong blur derivatives run on a worker; GPU uploads remain
on SDL. The UI retains its last catalog if an import fails.

HTTPS requests have an eight-second timeout, no redirects or proxy, and an
8 MiB body limit. Credentials in URL authority are refused. Importing is
explicit: neither startup nor the screensaver fetches remote images. URLs
may contain private tokens, so protect your local preference file when sharing
it. Lumaglass makes the explicit image request; it does not upload folder images
or contact an analytics service.
Desktop/TV permission errors do not grant the app extra filesystem access.

The generated cache is beside preferences, in `preferences.wallpapers`, with a
64 MiB quota. Derivatives and the catalog are replaced atomically, and originals
are never edited or removed. Old derivatives can use quota until the cache is
cleared manually with Lumaglass closed. PNG derivatives update the installed
Home splash only after actual saved selections, through the existing bounded
worker. TV splash copying accepts only the app's known persistent cache roots.
Cached 1080p images also work in experimental 4K mode through scaling. Thumbnail
and fullscreen derivative decoding stays on workers, with a bounded queue and
at most current/previous/saver full-image buffers retained by application state.
SDL uploads their decoded pixels to the existing GPU texture cache.

## Rotation

Settings → Background → Wallpaper rotation offers Off (default), At startup,
every five/fifteen minutes or every hour, Random order, Next wallpaper, and
individual image inclusion toggles. Bundled images are initially included;
personal images can be added after import. No selection keeps the current
wallpaper. One selection that is already current does not reload or save it.

Timers use monotonic time, count only while Home is visible and focused without
an overlay, and pause during the local screensaver or app launch. Standby does
not cause catch-up changes. Random order avoids immediate repeats. Reduced
motion disables the existing crossfade. Only actual changes are persisted.
The Home timer and screensaver presentation are independent.

[Immich](immich.md) now adds an optional read-only album connector under My images,
with a private connection file, explicit imports and the same offline cache.
Google Photos/Drive and any iCloud integration remain later optional connectors.
This original
implementation does not import code from upstream Luma PR #92; upstream and
Glasshouse provenance remain in NOTICE.md.
