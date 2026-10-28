// Optical glass section adapted from ziuus/liquid-glass (MIT), revision
// c75d171e0626c78f4bdb5030191d2af6795cd892, glass-core/src/renderer/glass.wgsl.
// Copyright (c) 2026 ziuus. Full notice: assets/licenses/ziuus-liquid-glass.txt.
// Lumaglass modifications: GLSL ES 1.00, analytic normals, bounded offsets,
// full/light optical paths, bounded cached layers, independent tint/fade opacity.
uniform sampler2D u_texture;
uniform float u_aa;
#if GLASS_OPTICS
uniform sampler2D u_backdrop;
uniform float u_optical;
uniform vec4 u_backdrop_bounds;
#endif
uniform vec4 u_edge_fade;
uniform vec4 u_edge_fade_x;
varying PAINT_PRECISION vec2 v_uv;
varying PAINT_PRECISION vec4 v_color;
varying PAINT_PRECISION vec4 v_shape;
varying PAINT_PRECISION vec2 v_params;
varying PAINT_PRECISION float v_logical_y;
varying PAINT_PRECISION float v_logical_x;
float edge_coverage() {
    float edge = 1.0;
    if (u_edge_fade.y > u_edge_fade.x)
        edge = smoothstep(u_edge_fade.x, u_edge_fade.y, v_logical_y) *
            (1.0 - smoothstep(u_edge_fade.z, u_edge_fade.w, v_logical_y));
    if (u_edge_fade_x.y > u_edge_fade_x.x)
        edge *= smoothstep(u_edge_fade_x.x, u_edge_fade_x.y, v_logical_x);
    if (u_edge_fade_x.w > u_edge_fade_x.z)
        edge *= 1.0 - smoothstep(u_edge_fade_x.z, u_edge_fade_x.w, v_logical_x);
    return edge;
}
#if GLASS_OPTICS
// Original Lumaglass light path: one texture read, no RGB dispersion, lens zoom,
// normalized 3D normal or Fresnel. It shares the full path's tint and fade contract.
vec3 light_glass(float d, vec2 half_size, float radius) {
    float inside = max(-d, 0.0);
    float bevel = clamp(min(radius, min(half_size.x, half_size.y) * 0.55), 4.0, 14.0);
    float edge = 1.0 - clamp(inside / bevel, 0.0, 1.0);
    vec2 local = v_shape.xy / max(half_size, vec2(1.0));
    vec2 direction = local / max(max(abs(local.x), abs(local.y)), 0.001);
    vec2 ray = direction * edge * edge * 2.5;
    vec2 uv = vec2(v_logical_x / 1920.0, 1.0 - v_logical_y / 1080.0);
    uv = clamp(uv + ray * vec2(1.0 / 1920.0, -1.0 / 1080.0),
               u_backdrop_bounds.xy, u_backdrop_bounds.zw);
    vec3 color = mix(texture2D(u_backdrop, uv).rgb, v_color.rgb, clamp(v_color.a, 0.0, 1.0));
    float rim = 1.0 - smoothstep(0.35, 1.5, inside);
    float light = clamp(0.55 - local.y * 0.32 - local.x * 0.18, 0.0, 1.0);
    return clamp(color + vec3(rim * (0.04 + light * 0.08)), vec3(0.0), vec3(1.0));
}
// Adapted optical model: lens magnification, IOR ray deflection and dispersion.
// The normal is analytic; five height-field evaluations and continuous ripples
// from the upstream shader are deliberately omitted for older TV GPUs.
vec3 optical_glass(float d, vec2 half_size, float radius) {
    vec2 positive = max(abs(v_shape.xy) - half_size + vec2(radius), vec2(0.0));
    vec2 sides = step(vec2(0.0), v_shape.xy) * 2.0 - 1.0;
    vec2 outward;
    float corner_length = length(positive);
    if (corner_length > 0.001) {
        outward = positive / corner_length * sides;
    } else {
        vec2 margin = half_size - abs(v_shape.xy);
        outward = margin.x < margin.y ? vec2(sides.x, 0.0) : vec2(0.0, sides.y);
    }
    float inside = max(-d, 0.0);
    float bevel = clamp(min(radius, min(half_size.x, half_size.y) * 0.55), 4.0, 14.0);
    float edge = 1.0 - clamp(inside / bevel, 0.0, 1.0);
    float edge2 = edge * edge;
    vec3 normal = normalize(vec3(outward * edge2 * 0.8, 1.0));
    float dome = clamp(inside / bevel, 0.0, 1.0);
    float zoom = 1.0 + dome * 0.025;
    float refractive_power = 1.0 - 1.0 / 1.48;
    vec2 ray = v_shape.xy * (1.0 / zoom - 1.0) + normal.xy * refractive_power * 8.0;
    // Together with dispersion this stays inside the cache's 14 logical px margin.
    ray = clamp(ray, vec2(-12.0), vec2(12.0));
    vec2 dispersion = outward * edge2 * 0.45;
    vec2 unit = vec2(1.0 / 1920.0, -1.0 / 1080.0);
    vec2 uv = vec2(v_logical_x / 1920.0, 1.0 - v_logical_y / 1080.0);
    vec2 uv_r = clamp(uv + (ray + dispersion) * unit, u_backdrop_bounds.xy, u_backdrop_bounds.zw);
    vec2 uv_g = clamp(uv + ray * unit, u_backdrop_bounds.xy, u_backdrop_bounds.zw);
    vec2 uv_b = clamp(uv + (ray - dispersion) * unit, u_backdrop_bounds.xy, u_backdrop_bounds.zw);
    vec3 backdrop = vec3(texture2D(u_backdrop, uv_r).r, texture2D(u_backdrop, uv_g).g, texture2D(u_backdrop, uv_b).b);
    vec3 color = mix(backdrop, v_color.rgb, clamp(v_color.a, 0.0, 1.0));
    float luminance = dot(backdrop, vec3(0.299, 0.587, 0.114));
    float primary = max(dot(normal.xy, vec2(-0.5665, -0.8240)), 0.0);
    float secondary = max(dot(normal.xy, vec2(0.5665, 0.8240)), 0.0);
    float fresnel = 1.0 - normal.z;
    fresnel *= fresnel;
    fresnel *= fresnel;
    float stroke = 1.0 - smoothstep(0.35, 1.5, inside);
    float glint = edge2 * primary * 0.24 + edge2 * edge * secondary * 0.08 + fresnel * 0.12 + stroke * 0.10;
    glint *= mix(1.0, 0.30, smoothstep(0.40, 0.70, luminance));
    return clamp(color + vec3(glint), vec3(0.0), vec3(1.0));
}
#endif
void main() {
    if (v_params.x < 0.0) {
        vec4 color = texture2D(u_texture, v_uv) * v_color;
        // -2: wallpaper with folded full-screen fills, p * m + k (k in v_shape).
        if (v_params.x < -1.5)
            color.rgb += v_shape.rgb;
        gl_FragColor = color * vec4(1.0, 1.0, 1.0, edge_coverage());
        return;
    }
    // Constant per-quad flags (-1024 full, -2048 light) preserve paint batching.
    // Fixed analytic lighting: no time uniform, texture blur, derivatives or offscreen pass.
    bool glass = v_params.y < -1000.0;
    float blur = glass ? 0.0 : max(-v_params.y, 0.0);
    vec2 half_size = v_shape.zw - vec2(blur);
    float radius = min(v_params.x, min(half_size.x, half_size.y));
    vec2 q = abs(v_shape.xy) - half_size + vec2(radius);
    float d = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - radius;
    // Box-filtered coverage of one drawable pixel (u_aa) centred on the edge: an edge on a
    // pixel boundary stays one crisp step, and a one-pixel border is fully opaque.
    float alpha = clamp(0.5 - d / u_aa, 0.0, 1.0);
    if (blur > 0.0) {
        alpha = 1.0 - smoothstep(-blur, blur, d);
    }
    if (v_params.y > 0.0) {
        alpha *= clamp(0.5 + (d + v_params.y) / u_aa, 0.0, 1.0);
    }
    if (glass && (alpha <= 0.0 || v_uv.x <= 0.0)) {
        gl_FragColor = vec4(0.0); return;
    }
    vec4 color = glass ? v_color : texture2D(u_texture, v_uv) * v_color;
    if (glass) {
#if GLASS_OPTICS
        if (u_optical > 0.5) {
            vec3 optical;
            if (v_params.y < -1500.0) {
                optical = light_glass(d, half_size, radius);
            } else {
                optical = optical_glass(d, half_size, radius);
            }
            gl_FragColor = vec4(optical, v_uv.x * alpha * edge_coverage());
            return;
        }
#endif
        vec2 local = v_shape.xy / max(half_size, vec2(1.0));
        float rim = 1.0 - smoothstep(0.5, 2.8, -d);
        float light = clamp(0.55 - local.y * 0.32 - local.x * 0.18, 0.0, 1.0);
        float sheen = 0.035 * (1.0 - smoothstep(-1.0, -0.15, local.y));
        color.rgb = mix(color.rgb, vec3(1.0), rim * (0.16 + light * 0.62) + sheen);
        // Tint and layer fade remain independent even without a usable backdrop.
        color.a *= v_uv.x;
    }
    gl_FragColor = color * vec4(1.0, 1.0, 1.0, alpha * edge_coverage());
}
