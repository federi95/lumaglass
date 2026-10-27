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

The clear treatment respects the user's wallpaper blur setting; opening the
control center does not force a softer image. Personal-photo decoding stays on
the wallpaper worker. No Apple images, shaders or SDK code are bundled;
[asset provenance](assets.md) records the original symbols.

## Clear tuning

Resting controls now use 0.22–0.34 tint opacity instead of 0.64–0.82. Raised
backplates use 0.56–0.68 instead of 0.94–0.96; clock/navigation use this stronger
backing to preserve text legibility. Focused control-center tiles use 0.32–0.48
instead of 0.70–0.90. Light focused selections retain 0.86–0.90 opacity so their
dark labels remain distinct. These values describe individual surfaces, not a
promised final transmission percentage: overlapping materials compose together.

The control center uses a 35% dimming backdrop with clear glass. Its secondary
text and slider labels are brighter, and the existing rim reflection is stronger.
These are constant/color changes in the existing path, with no extra render
passes, texture allocations or new dependencies. This increases translucency; it
does not add physical backdrop refraction.

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

## liquid-rust assessment

Reviewed [liquid-rust 0.2.3](https://github.com/al1h3n/liquid-rust/tree/0a2b2f579a33e78346cc52b47ccabc2a395682a7)
as a reference for a more complete optical effect. Its
[manifest](https://github.com/al1h3n/liquid-rust/blob/0a2b2f579a33e78346cc52b47ccabc2a395682a7/Cargo.toml)
declares Rust 1.99 and wgpu 30. Lumaglass currently uses a Rust 1.93 development/CI
toolchain and creates an OpenGL ES 2.0 context on TV. The
[wgpu GL backend](https://wgpu.rs/doc/wgpu/enum.Backend.html#variant.Gl)
requires at least OpenGL ES 3.0. Actual TV driver capabilities, formats and
surface integration would need verification before adopting that stack.

The library samples a content texture, copies it to a target, and builds/reuses a
blur pyramid; its renderer uses RGBA16Float pyramid textures and additional
backdrops for overlapping glass layers. It offers genuine refraction and more
complete effects than Lumaglass's analytic rim. Caching and idle suspension are
useful optimizations, but the published RTX 3060 Ti timing does not establish
performance on LG TV GPUs. No library benchmark, build, installation or hardware
probe was run here.

Direct integration is deferred. A selective GLSL implementation of optical ideas
would be a separate engineering change; reusing library code would also require
preserving the applicable MIT/Apache notices. This revision only adjusts the
original Lumaglass material and does not incorporate its code.
