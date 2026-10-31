# Optional features and runtime resources

This is a source audit of the native launcher at revision
`7565a7114cfa2db45ecdbb0bc43c6e6ea448d536`, not a TV benchmark.
An unused feature can retain state or a sleeping thread without continuously
consuming CPU. A thread belongs to the launcher process; it is not an independent
service installed for that feature.

## Photo sources

Immich, Nextcloud/WebDAV, Google Drive, Google Photos local exports and ordinary
image imports share **one acquisition worker**. There is no worker per provider.
At startup it reads the local image catalog and prepares cached thumbnails, then
blocks on its job channel. With an empty local catalog, there are no photo
thumbnails to decode. It does not connect, refresh OAuth grants, scan a remote
library or download photos at startup, even when a connection file exists.
Opening a source page also does not submit a remote load automatically. Loading
folders/albums and importing images require explicit actions.

A second, shared **decoder worker** is also created at startup and blocks until
an image is needed. Full personal images are requested for the current/transitioning
wallpaper or an active photo screensaver. Decoded entries outside that needed set
are removed. Both threads exist even if no photo source has ever been configured:
there are channel structures and thread/stack resources, but no continuous decode
loop while idle. Empty provider sessions have empty collections and selections.

Previously imported photos remain on disk and their small previews are loaded
at startup, independently of whether their original service is connected. Source
folder/album metadata can remain in the session after leaving its page. Forgetting
a connection preserves offline images. Closing a page is not cancellation of an
already submitted import; bounded in-flight work can finish in the background.

The Google authorization and Photos export scripts run only when explicitly
started on a computer. The launcher does not start Node or a browser for them.
Google Photos on the TV reads an exported local folder; it has no Google API
connection. See [photo services](photo-services.md) and [Immich](immich.md).

Implementation: [worker and decoder](../src/features/wallpapers/mod.rs),
[cloud session](../src/features/wallpapers/cloud/state.rs),
[requested images](../src/app/runtime.rs).

## Liquid Glass

Liquid Glass has **no dedicated thread or process**. Its optical work is part of
the ordinary OpenGL render pass. Off emits no glass commands and releases the
dedicated framebuffer/texture caches on the next render. It performs no backdrop
refresh or optical sampling. The common renderer, paint shader program, UI
artwork and ordinary animation state still exist; Off does not unload the entire
renderer or imply that all UI animations are disabled.

| Mode  | Maximum dedicated RGBA target payload  |
| ----- | -------------------------------------- |
| Off   | 0 after the next render                |
| Light | Two 1280×720 targets, 7.03 MiB total   |
| Full  | Two 1920×1080 targets, 15.82 MiB total |

These are upper bounds for texture pixels, excluding driver overhead and all
other rendering resources. Actual allocations can be smaller or absent, depending
on the scene and driver. Unchanged backdrop content reuses the cache; the effect
does not add a permanent animation timer. Reducing its quality removes GPU work
and target memory, but the change in frame time or responsiveness needs measurement
on the actual TV. See [Liquid Glass](liquid-glass.md) for shader costs and diagnostics.

Implementation: [target release](../src/render/renderer.rs),
[cache allocation and destruction](../src/render/backdrop.rs),
[finite animation state](../src/app/state.rs).

## Other baseline workers and wakeups

The current launcher starts several shared workers eagerly. Optional settings
usually gate their jobs, rather than creating and destroying their threads.

| Worker                                            | Created in the normal launcher              | Work when the related feature is unused                                                                                                |
| ------------------------------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Photo acquisition and image decoder               | Two threads                                 | Local catalog/previews once, then blocking job waits                                                                                   |
| Weather search, forecast and cache writer         | Three threads                               | Blocking waits; no forecast request while weather is hidden, Home is not visible or no location is configured                          |
| Platform actions, observations and app management | Three shared threads                        | Startup inventory/splash work, then blocking request waits; management/observation work is submitted on demand                         |
| Preferences writer                                | One thread                                  | Blocks until a changed preference snapshot needs saving                                                                                |
| Local clock                                       | One thread, omitted for fixed-time captures | Timezone-offset refresh once a minute and on foreground events; each refresh starts a bounded helper invocation of the same executable |
| Installed app artwork                             | One thread on webOS                         | Decodes requested/prefetched app icons, then waits on a condition variable                                                             |

An already running weather request is not necessarily aborted by hiding weather;
its obsolete generation is ignored. Native webOS lifecycle/subscription plumbing,
SDL and the graphics driver may add resources outside this table. It is not a
measurement of the operating system's total thread count or resident memory.
The dedicated system screensaver entry point is only used when that executable
mode is invoked; its power-monitor worker does not run alongside a normal launcher
just because the module is compiled into the application.

The UI does not continuously redraw a still screen for glass or cloud connectors.
It still wakes about every 100 ms in its ordinary visible idle path to inspect
state and worker replies, or every 250 ms while hidden; animations, held buttons,
pending icons and startup preparation use different intervals. Other pages are
prewarmed once after the screen settles, which intentionally spends some work and
retains ordinary text/icon/geometry caches for subsequent navigation. Clock,
enabled weather and configured wallpaper rotation can also trigger later updates.

Implementation: [launcher startup and event loop](../src/app/runtime.rs),
[weather queues](../src/features/weather/service.rs),
[weather request gating](../src/features/weather/mod.rs),
[platform queues](../src/platform/mod.rs),
[preference writer](../src/storage/writer.rs),
[clock helper](../src/platform/clock.rs),
[artwork queue](../src/features/apps/artwork.rs),
[system saver entry point](../src/features/screensaver/runtime.rs).

## Optimization priorities

There is no per-connector polling loop to remove. The concrete remaining
opportunity is lazy startup of the shared image decoder and weather workers,
followed by measuring whether their idle resident memory matters on the target
firmware. Deferred thumbnail loading could reduce startup work for a populated
local catalog. These would trade simpler worker lifetimes and quick first use
for lower unused-feature overhead; they have not been implemented by this audit.

For the first hardware comparison, keep the same scene, drawable resolution and
wallpaper, compare Off/Light/Full, and record resident memory, idle CPU, presentation
pacing and input response after settling and during navigation. Separate first-use
loading from steady-state work. A texture budget or a sleeping-thread count alone
does not establish a slowdown percentage.

## Subsequent root-Home helper update

Root Home observations and controls can now execute their bundled Node helper
directly, without asking Homebrew Channel to elevate an already-root process.
The child has a separate process group, a seven-second deadline and a 2 MiB
output limit; timeout/error cleanup kills only that job’s group. Ordinary jailed
app launches keep the elevated service path. This changes on-demand work, not
the idle worker count. Wi-Fi names stay in memory and additional SVG avatars
use the ordinary icon cache; neither introduces polling or an online account.
