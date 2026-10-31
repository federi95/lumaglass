# Liquid Glass appearance

**Settings → General → Appearance → Liquid Glass** cycles Off, Light and Full.
Full preserves the existing effect and remains the default. Saved explicit choices
remain respected: legacy booleans `true`/`false` load as Full/Off, and new saves
use `"full"`, `"off"` or `"light"`. Missing/invalid choices retain Full without
resetting unrelated appearance settings. There is no automatic TV-model heuristic.
The native Rust/OpenGL renderer keeps its SDL window, OpenGL ES 2.0 path,
existing controls and remote navigation.

| Mode  | Optical work per glass pixel                                                                               | Maximum dedicated target texels        |
| ----- | ---------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| Off   | None; opaque controls                                                                                      | None after the next render             |
| Light | One backdrop sample, bounded edge displacement and simple highlights                                       | Two 1280×720 RGBA8 targets: 7.03 MiB   |
| Full  | Three backdrop samples, magnification, IOR displacement, RGB dispersion and directional/Fresnel highlights | Two 1920×1080 RGBA8 targets: 15.82 MiB |

These are implementation bounds, not measured GPU times. Driver overhead,
artwork and the main surface are additional. Light reduces maximum target texel
payload by about 56%; that is not a 56% frame-time or overall-memory improvement.

## Optical effect

The optical section of the shader is a selective port from
[ziuus/liquid-glass](https://github.com/ziuus/liquid-glass/tree/c75d171e0626c78f4bdb5030191d2af6795cd892).
It adds local lens magnification, index-of-refraction edge displacement, subtle
RGB dispersion and directional/Fresnel edge highlights. Analytic rounded-rectangle
normals replace the upstream five height-field evaluations. Three backdrop texture
samples produce the Full optical color; offsets, including dispersion, stay within
the 14 logical pixel cache margin. The original Lumaglass Light shader uses a
single RGB sample, a bounded 2.5 logical pixel edge displacement and simple rim
lighting. It omits RGB dispersion, lens magnification, a normalized 3D normal,
luminance-dependent highlights and Fresnel calculations. A constant per-quad
quality flag preserves batching and routes the optical branch. No continuous
ripples or physics timer is added.
The selected tab uses its existing position/width animation, so its lens moves
with the selection. The tab track, active pill, separate hover pill, combined
clock/avatar capsule, dedicated Control Center button and optional favorites dock
use the same clear chrome. Selected, focused/hovered and pressed controls have
distinct tint, rim and shadow states. Control hit areas stay fixed; focused app
artwork lifts inside its slot unless reduced motion is enabled.

Tint opacity and layer opacity are independent: a clear lens samples the content
beneath it while panel and Recents fades can still reach exactly zero. Labels,
icons and focus outlines are drawn afterwards at the main drawable resolution;
they are not included in their own lens backdrop.

## Backdrop cache and rendering budget

There are at most two RGBA8 GPU targets, each capped at 1280×720 in Light or
1920×1080 in Full even when the main UI draws at 4K. Switching quality resizes
and refreshes the targets when their capped dimensions differ. If the drawable
or driver already forces a smaller size, both modes may allocate the same texel
payload; the shader still follows the selected mode. Off emits no glass commands
and drops both targets on the next render. Smaller windows/driver limits use
smaller 16:9 targets. Text and icons continue rendering at the original drawable
density. In a mixed-quality scene the targets use the highest requested optical
quality; the ordinary UI applies one mode consistently across all glass controls.

The base target captures lower content before the first glass surface. The
control-center target captures the attenuated parent interface with a 38% contrast
scrim, before the center's own title, tile labels and controls. The parent page's
foreground commands use 12% opacity and 65% brightness, so sharp background
Settings rows no longer compete with the center's own labels. Independent power,
network/audio and screensaver/timer lenses, four circular shortcuts and the detail
slider replace the large outer backplate. There is no opaque enclosing panel.
The profile reuses the second target for its independent clear pill controls,
without an enclosing panel. App-options menus use it for the outer optical panel;
their inner standard rows composite analytic material over that panel.
Header controls return to the base target. Other opaque dialogs use analytic
controls, avoiding a lens that would incorrectly reveal wallpaper through a solid
panel. This is a bounded two-layer design, not a recursive glass compositor.

Targets are rerendered entirely on the GPU when their relevant lower content,
coverage or resolution changes. No `grim`, CPU pixel readback, Vulkan, wgpu,
WGSL runtime or multipass blur engine is used. Keys exclude foreground labels
and lower content outside the lens coverage. Navigation-lens motion within the
existing captured coverage can reuse its backdrop. Regions include an outward-rounded sampling
margin to absorb small geometry changes, and the shader clamps reads inside valid pixels. When a backdrop changes,
partial redraw also includes the affected lenses to prevent stale refraction.

Each material is still one six-vertex paint quad; its lens sampling and rim share
one paint pass. Compatible quads batch when their texture, clipping, fade and
backdrop layer match. Focus outlines and shadows remain separate primitives.
Snapshot passes are additional work when a backdrop needs refreshing. They are
scissored to the required region, and do not run continuously over unchanged
content. At 4K the 720p/1080p backdrop is intentionally upsampled to bound memory/bandwidth;
the final glass shader still shades the control at the main drawable resolution.

The clear treatment respects the user's wallpaper blur preference. Chrome resting
tint opacity is 0.10–0.18; selected chrome uses 0.18–0.20; focus uses 0.24;
press uses 0.32–0.38. Its labels stay light. Settings retain stronger selected
faces (0.86–0.90) for dark labels. These describe individual surfaces, not the
final transmission of stacked elements. The Control Center leaves the parent
visible as a subdued context and adds a 38% dimming scrim for separation.
Control Center/profile controls now keep the same 0.10–0.18 resting tint and 0.24
focus tint as clear header chrome. App-options panels retain denser 0.34–0.50
resting tint and 0.40–0.52 focus tint. This adds no extra GPU target or
idle animation loop.

## Fallback and diagnostics

Optics require a fragment high-precision path and a complete RGBA8 framebuffer.
The renderer retries compilation with the analytic shader if optical compilation
fails. Unsupported precision or failed/capped framebuffer allocation keeps analytic
glass without optical targets in both Light and Full. Off instead uses opaque
controls without glass shader commands. An allocation failure
is not retried on every frame at the same size. No new GPU extension is required.

Rendering diagnostics include:

- `optical_glass_supported`: optical shader path compiled and precision supported.
- `glass_quality`: highest requested optical quality (`light`/`full`, null without optical commands); this does not establish successful driver support.
- `glass_background_bytes`: allocated RGBA target texel bytes.
- `glass_background_refreshes`: targets refreshed during this frame.
- `glass_background_ms`: CPU preparation/submission time, not a GPU timestamp.

Desktop CI covers rendering, while compilation and packaging do not establish
a TV frame-rate guarantee or driver compatibility. Those require the pending hardware checks.
The intended next performance step is a controlled Off/Light/Full comparison on
the target TV, using actual presentation pacing and input responsiveness during
navigation and modal transitions. CPU submission time alone cannot establish
GPU cost. Automatic quality selection remains deferred until real measurements
justify a policy; OLED/QNED branding or an Alpha generation is insufficient.

## Provenance and references

The source revision, adapted formulas and modifications are recorded in
[NOTICE](../NOTICE.md). The complete original **Copyright (c) 2026 ziuus** MIT
license is included in `assets/licenses/ziuus-liquid-glass.txt` and required in the
IPK. Native cache/layer integration is original Lumaglass code. Apple SDK code,
artwork and private implementations are not bundled.

[Apple's materials guidance](https://developer.apple.com/design/human-interface-guidelines/materials)
and [adoption guide](https://developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass)
inform functional layering and focus. This is an adaptation, not a promise of
pixel-identical Apple rendering.

The previously evaluated [liquid-rust](https://github.com/al1h3n/liquid-rust/tree/0a2b2f579a33e78346cc52b47ccabc2a395682a7)
uses wgpu 30 and declares Rust 1.99; it is not integrated. No code from that library
is included. ziuus's complete Vulkan/wgpu engine, Wayland capture, media widgets,
RK4 physics and blur subsystem are also not imported.
