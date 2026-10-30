# Appearance

Open **Settings → General → Appearance**. Press OK on a row to cycle its value;
changes apply immediately and are saved locally without restarting the launcher.

## Settings and focus

Settings opens a category menu on the right. Choose General, Background, Weather,
Inputs or System with OK (or Right). Each category and its subpages use a compact
right-hand list; the left side shows contextual artwork, the focused setting's
full title, value and help. Long row labels are shortened visually, while the
context panel keeps the full label. Background selection adds a larger preview.
Back/Left returns through the parent pages and then to the category menu.

The header centers the current page title and provides separate Back, Home and
Apps buttons. Clock/avatar, Control Center and LG settings remain independent.
Saved choices have a checkmark; focus uses a light capsule and dark text. Focused
artwork and controls lift briefly over stable pointer hit targets. Presses compress
the surface. The transition settles after 140 ms and schedules no further frames;
Reduced motion removes interpolation. It does not change navigation or selections.

Settings uses the pre-generated strongly blurred wallpaper with a fixed contrast
scrim, independently of Home's wallpaper-blur preference. No live blur or animated
background filter is introduced. Ordinary list rows use simple translucent fills;
optical glass is reserved for navigation, floating controls and selected surfaces.
The existing Off/Light/Full renderer and cache limits remain in force.

The layout is an original Rust/SDL implementation informed by user-provided tvOS
screenshots and Apple's [tvOS design](https://developer.apple.com/design/human-interface-guidelines/designing-for-tvos),
[materials](https://developer.apple.com/design/human-interface-guidelines/materials)
and [focus](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection/)
guidance. It uses existing licensed artwork, Inter and original/Lucide icons;
Apple design resources, SF fonts and screenshot assets are not bundled.

## Card density

| Density            | Cards across the Apps grid and fully visible Home strip |
| ------------------ | ------------------------------------------------------- |
| Compact            | 8                                                       |
| Standard (default) | 6                                                       |
| Comfortable        | 5                                                       |

Card proportions and caption text size are preserved. Captions reserve their
measured height. Drawing, hit targets, wheel scrolling and remote navigation use
the same geometry; Apps Up/Down follows the selected column count. Home's optional
two-row layout keeps its row-major favorite order and shares horizontal scrolling.
Changing density resets Home's horizontal scroll to the start. Input cards and
the background gallery keep their own layouts.

Existing preference files use Standard density. Unknown values fall back to
Standard without discarding unrelated preferences.

## Accent color

Cycle Neutral, Blue, Teal, Violet, Amber and Rose. Neutral is the original default.
The palette uses light selection surfaces with dark labels. Accent colors apply
to navigation and settings selections, focus outlines and selected indicators;
the app artwork keeps its original colors. This controls color only: outline
thickness, animation speed and other focus styling are not additional settings.

## Wallpaper effects

**Wallpaper dimming** cycles Off, 20%, 40% and 60%. **Wallpaper blur** cycles Off,
Soft and Strong. Both default to Off and are independent of one another.
The dimming layer sits beneath cards, text and menus, so only the background is
darkened. Home and Apps follow the selected wallpaper effects,
including both wallpapers during a background crossfade.

The asset build prepares soft/strong JPEG variants for every bundled wallpaper
with Gaussian sigma 8/20 at 1080p, scaled proportionally for optional `--4k`
assets. The TV loads the selected variant through its existing texture cache;
there is no per-frame blur filter. The packaged variants are required by IPK
verification. Effects apply inside the launcher; LG's startup splash uses the
selected stock image. Personal image sources and offline imports are documented in the wallpaper and Immich guides.

## Liquid Glass

Cycle **Off → Light → Full**. Full is the existing appearance and remains the default.
Light keeps translucent controls with simpler reflections and a smaller backdrop
cache. Off selects opaque surfaces and releases the optical targets on the next
render. The choice applies to navigation, clock/avatar, the dock, settings,
Control Center and profile controls without changing their layout or navigation.

Legacy `liquid_glass: true` loads as Full and `false` as Off. New saves use
`"off"`, `"light"` or `"full"`; missing/invalid values keep the existing Full default.
No TV-model heuristic or automatic quality change overrides the selected mode.
See [the rendering budget and design references](liquid-glass.md).
