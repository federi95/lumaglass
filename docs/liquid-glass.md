# Liquid Glass appearance

**Settings → General → Appearance → Liquid Glass** enables the glass treatment.
It defaults to On, including when an older appearance block has no value for this
new option. Off selects opaque controls. Saved explicit choices remain respected.

This is an original, lightweight interpretation of Apple's material language for
Lumaglass's Rust/OpenGL renderer. It is not Apple's platform implementation.
Translucent surfaces, a vertical tint, directional edge reflections and a small
specular rim distinguish navigation, settings controls and the control center.
App logos, photos and other content retain their existing artwork.

## Rendering budget

Each glass surface uses one rounded paint quad: six vertices in the existing
paint batch. Focus outlines and optional shadows remain separate batched primitives. Its fill, gradient and illuminated rim share one shader pass. The
material flag lives in the existing vertex data, so it does not require another
shader program, a texture or a uniform update per control. Adjacent compatible
surfaces continue to batch. Compared with the preceding fill-plus-border design,
a resting material can replace two paint quads with one.

Lighting is analytic and fixed. The material adds no clock, continuous shimmer,
screen capture, framebuffer readback, offscreen render target, multipass blur or
runtime tessellation. The signed distance already needed by rounded controls is
reused for the rim. Glass follows the existing partial-damage bounds, clipping,
opacity and reduced-motion handling. An unchanged material does not schedule
redraws by itself.

When the control center is open, the glass style reuses the existing precomputed
soft wallpaper. Personal-photo decoding stays on the wallpaper worker. Closing
the panel restores the user's wallpaper blur setting. With glass disabled the
panel does not force a soft wallpaper. No Apple images, shaders or SDK code are
bundled; [asset provenance](assets.md) records the original symbols.

The same bounded shader path supports the existing OpenGL ES 2 renderer and its
fragment precision selection. No new GPU extension or blur buffers are required.
This controls the implementation's cost, but does not establish a frame-rate
guarantee across LG TVs. TV frame timings, 1080p/experimental 4K behavior and input
latency remain pending hardware verification; no tests or CI were run for this
revision at the owner's request.

## References

[Apple's materials guidance](https://developer.apple.com/design/human-interface-guidelines/materials)
separates navigation/control materials from content and recommends restrained use.
[Adopting Liquid Glass](https://developer.apple.com/documentation/TechnologyOverviews/adopting-liquid-glass)
describes focus-led treatment on tvOS and adapting transparency and motion.
These are design references; Lumaglass uses its own optimized implementation.
