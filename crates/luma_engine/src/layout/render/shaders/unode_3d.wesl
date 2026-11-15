import bevy_pbr::render::{
    mesh_bindings,
    mesh_functions,
    mesh_types,
    mesh_view_bindings,
    pbr_types,
    pbr_functions,
    forward_io::VertexOutput,
};

struct UNodeMaterial3d {
    color: vec4<f32>,
    radius: vec4<f32>,
    border_color: vec4<f32>,
    emissive: vec4<f32>,
    size: vec2<f32>,
    border_width: f32,
    border_offset: f32,
    softness: f32,
    metallic: f32,
    roughness: f32,
    use_texture: u32,
    shape_mode: u32,
    pbr_enabled: u32,
    reflectance: f32,
    lighting_flags: u32,

    bevel_params: vec4<f32>,

    grad_colors: array<vec4<f32>, 8>,
    grad_stops: array<vec4<f32>, 2>,
    grad_params: vec4<f32>,
    grad_center: vec4<f32>,

    shadow_color: vec4<f32>,
    shadow_params: vec4<f32>,

    inner_glow_color: vec4<f32>,
    inner_glow_params: vec4<f32>,

    custom_params: array<vec4<f32>, 4>,
    custom_meta: vec4<f32>,
    custom_cursor: vec4<f32>,

    clip_inv_x: array<vec4<f32>, 4>,
    clip_inv_y: array<vec4<f32>, 4>,
    clip_bounds: array<vec4<f32>, 4>,
    clip_radius: array<vec4<f32>, 4>,
}

@group(3) @binding(0) var<uniform> material: UNodeMaterial3d;
@group(3) @binding(1) var base_texture: texture_2d<f32>;
@group(3) @binding(2) var base_sampler: sampler;

fn sd_box_dynamic(p: vec2<f32>, b: vec2<f32>, r_in: vec4<f32>, mode: u32) -> f32 {
    let limit = max(min(b.x, b.y), 0.0);
    let r_vec = min(max(r_in, vec4<f32>(0.0)), vec4<f32>(limit));

    // Local coordinates have +Y up, matching (top_right, bottom_right, top_left, bottom_left).
    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;
    let r_top = select(r_vec.z, r_vec.x, is_right);
    let r_bot = select(r_vec.w, r_vec.y, is_right);
    let radius = select(r_bot, r_top, is_top);

    let q = abs(p) - b + radius;

    if (mode == 1u) {
        let d_square = max(q.x, q.y);
        let d_diagonal = (q.x + q.y) * 0.70710678;
        return max(d_square, d_diagonal) - radius;
    } else {
        let d_round = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
        return d_round - radius;
    }
}

// -----------------------------------------------------------------------------
// Multi-Stop Gradient Sampling
// -----------------------------------------------------------------------------

fn get_gradient_stop_pos(idx: u32) -> f32 {
    if (idx < 4u) {
        if (idx == 0u) { return material.grad_stops[0].x; }
        if (idx == 1u) { return material.grad_stops[0].y; }
        if (idx == 2u) { return material.grad_stops[0].z; }
        return material.grad_stops[0].w;
    } else {
        if (idx == 4u) { return material.grad_stops[1].x; }
        if (idx == 5u) { return material.grad_stops[1].y; }
        if (idx == 6u) { return material.grad_stops[1].z; }
        return material.grad_stops[1].w;
    }
}

fn sample_gradient(t_in: f32) -> vec4<f32> {
    let count = u32(material.grad_params.z);
    if (count == 0u) {
        return material.color;
    }
    if (count == 1u) {
        return material.grad_colors[0];
    }

    let t = clamp(t_in, 0.0, 1.0);
    let is_stepped = material.grad_params.w >= 0.5;

    if (is_stepped) {
        var color = material.grad_colors[0];
        for (var i = 0u; i < 8u; i = i + 1u) {
            if (i >= count) {
                break;
            }
            let p = get_gradient_stop_pos(i);
            if (t >= p) {
                color = material.grad_colors[i];
            } else {
                break;
            }
        }
        return color;
    }

    let p_first = get_gradient_stop_pos(0u);
    if (t <= p_first) {
        return material.grad_colors[0];
    }
    let p_last = get_gradient_stop_pos(count - 1u);
    if (t >= p_last) {
        return material.grad_colors[count - 1u];
    }

    var color = material.grad_colors[count - 1u];
    for (var i = 0u; i < 7u; i = i + 1u) {
        if (i + 1u >= count) {
            break;
        }
        let p_curr = get_gradient_stop_pos(i);
        let p_next = get_gradient_stop_pos(i + 1u);
        if (t >= p_curr && t <= p_next) {
            let span = p_next - p_curr;
            if (span <= 0.0001) {
                color = material.grad_colors[i + 1u];
            } else {
                let factor = clamp((t - p_curr) / span, 0.0, 1.0);
                color = mix(material.grad_colors[i], material.grad_colors[i + 1u], factor);
            }
            break;
        }
    }
    return color;
}

// -----------------------------------------------------------------------------
// Procedural Noise & Custom Shader Hook (UShader)
// -----------------------------------------------------------------------------

fn luma_hash21(p_in: vec2<f32>) -> f32 {
    var p = fract(p_in * vec2<f32>(123.34, 456.21));
    p = p + dot(p, p + 45.32);
    return fract(p.x * p.y);
}

fn luma_hash22(p_in: vec2<f32>) -> vec2<f32> {
    let n = sin(dot(p_in, vec2<f32>(41.0, 289.0)));
    return fract(vec2<f32>(262144.0, 32768.0) * n);
}

fn luma_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = luma_hash21(i);
    let b = luma_hash21(i + vec2<f32>(1.0, 0.0));
    let c = luma_hash21(i + vec2<f32>(0.0, 1.0));
    let d = luma_hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn luma_fbm(p_in: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var p = p_in;
    let rot = mat2x2<f32>(cos(0.5), sin(0.5), -sin(0.5), cos(0.5));
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * luma_noise(p);
        p = rot * p * 2.02 + vec2<f32>(100.0, 100.0);
        a = a * 0.5;
    }
    return v;
}

fn luma_cursor_uv() -> vec2<f32> {
    return material.custom_cursor.xy;
}

fn luma_cursor_hover() -> f32 {
    return material.custom_cursor.z;
}

fn luma_cursor_press() -> f32 {
    return material.custom_cursor.w;
}

fn luma_cursor_dist(uv: vec2<f32>, size: vec2<f32>) -> f32 {
    let aspect = size.x / max(size.y, 1.0);
    let diff = (uv - material.custom_cursor.xy) * vec2<f32>(aspect, 1.0);
    return length(diff);
}

// __LUMA_CUSTOM_SHADER_HOOK_START__
fn sample_custom_shader(
    uv: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    params: array<vec4<f32>, 4>,
) -> vec4<f32> {
    return params[0];
}
// __LUMA_CUSTOM_SHADER_HOOK_END__

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let shadow_blur = material.shadow_params.z;
    let shadow_spread = material.shadow_params.w;
    let shadow_offset = material.shadow_params.xy;
    let shadow_margin = select(
        0.0,
        shadow_blur * 2.5 + shadow_spread + max(abs(shadow_offset.x), abs(shadow_offset.y)),
        shadow_blur > 0.0 || shadow_spread > 0.0,
    );
    let mesh_size = material.size + vec2<f32>(shadow_margin * 2.0);

    let uv_centered = in.uv - 0.5;
    let p = vec2<f32>(uv_centered.x, -uv_centered.y) * mesh_size;
    let half_size = material.size * 0.5;

    let dist_outer = sd_box_dynamic(p, half_size, material.radius, material.shape_mode);
    let softness = max(material.softness, 0.00001);
    let grad_outer = length(vec2<f32>(dpdx(dist_outer), dpdy(dist_outer)));
    let aa_width = max(grad_outer, softness);
    let aa_inner = aa_width * 0.5;

    // Multi-layer rotation/scale-aware clipping against ancestor containers
    var alpha_clip = 1.0;
    let clip_wp = vec4<f32>(in.world_position.xyz, 1.0);
    for (var i = 0u; i < 4u; i = i + 1u) {
        let bounds = material.clip_bounds[i];
        if (bounds.z < 0.5) {
            break;
        }
        let p_clip = vec2<f32>(
            dot(material.clip_inv_x[i], clip_wp),
            dot(material.clip_inv_y[i], clip_wp),
        );
        let d_clip = sd_box_dynamic(
            p_clip,
            bounds.xy,
            material.clip_radius[i],
            u32(bounds.w + 0.5),
        );
        let grad_clip = length(vec2<f32>(dpdx(d_clip), dpdy(d_clip)));
        let clip_aa = max(grad_clip, softness) * 0.5;
        alpha_clip = alpha_clip * (1.0 - smoothstep(-clip_aa, clip_aa, d_clip));
    }

    let border_width = material.border_width;
    let border_offset = material.border_offset;
    let dist_body = dist_outer + border_width + border_offset;

    let border_outer_alpha = 1.0 - smoothstep(-aa_inner, aa_inner, dist_outer);
    let border_inner_alpha = 1.0 - smoothstep(-aa_inner, aa_inner, dist_outer + border_width);

    var border_alpha = 0.0;
    if (border_width > 0.0) {
        border_alpha = max(border_outer_alpha - border_inner_alpha, 0.0);
    }

    var body_alpha = border_outer_alpha;
    if (border_width > 0.0 || border_offset > 0.0) {
        body_alpha = min(1.0 - smoothstep(-aa_inner, aa_inner, dist_body), border_outer_alpha);
    }

    // Body UV relative to the base element bounds (top-left = (0, 0), bottom-right = (1, 1))
    let body_uv = (p / max(material.size, vec2<f32>(0.00001))) * vec2<f32>(1.0, -1.0) + 0.5;

    var body_color = material.color;
    let offset = material.grad_center.z;
    if (material.grad_params.x == 1.0) {
        let angle = material.grad_params.y;
        let dir = vec2<f32>(cos(angle), sin(angle));
        let t_lin = dot(body_uv - 0.5, dir) + 0.5 - offset;
        body_color = sample_gradient(t_lin);
    } else if (material.grad_params.x == 2.0) {
        let center = material.grad_center.xy;
        let radius = max(material.grad_params.y, 0.001);
        let dist_rad = length(body_uv - center);
        let t_rad = (dist_rad / radius) - offset;
        body_color = sample_gradient(t_rad);
    } else if (material.grad_params.x == 3.0) {
        let center = material.grad_center.xy;
        let dir = body_uv - center;
        let angle = atan2(dir.x, -dir.y);
        let norm_angle = fract((angle / 6.283185307179586) + 1.0);
        let t_conic = norm_angle - offset;
        body_color = sample_gradient(t_conic);
    }

    if (material.use_texture > 0u) {
        let tex_uv = clamp(body_uv, vec2<f32>(0.0), vec2<f32>(1.0));
        let tex_sample = textureSample(base_texture, base_sampler, tex_uv);
        body_color = tex_sample * body_color;
    }

    // Custom UShader background injection / replacement
    if (material.custom_meta.w > 0.5) {
        let shader_col = sample_custom_shader(
            clamp(body_uv, vec2<f32>(0.0), vec2<f32>(1.0)),
            material.size,
            material.custom_meta.x,
            material.custom_params,
        );
        let blend_mode = u32(material.custom_meta.y + 0.5);
        let opacity = clamp(material.custom_meta.z, 0.0, 1.0);
        let eff_alpha = clamp(shader_col.a * opacity, 0.0, 1.0);

        if (blend_mode == 0u) {
            body_color = vec4<f32>(
                mix(body_color.rgb, shader_col.rgb, opacity),
                mix(body_color.a, shader_col.a, opacity),
            );
        } else if (blend_mode == 1u) {
            body_color = vec4<f32>(
                mix(body_color.rgb, shader_col.rgb, eff_alpha),
                max(body_color.a, eff_alpha),
            );
        } else if (blend_mode == 2u) {
            let mult_rgb = body_color.rgb * shader_col.rgb;
            body_color = vec4<f32>(
                mix(body_color.rgb, mult_rgb, eff_alpha),
                body_color.a,
            );
        } else {
            body_color = vec4<f32>(
                clamp(body_color.rgb + shader_col.rgb * eff_alpha, vec3<f32>(0.0), vec3<f32>(1.0)),
                max(body_color.a, eff_alpha),
            );
        }
    }

    // Holographic inner edge glow
    var inner_glow_emissive = vec3<f32>(0.0);
    if (material.inner_glow_params.y > 0.5) {
        let inner_blur = max(material.inner_glow_params.x, 0.0001);
        let inner_spread = max(material.inner_glow_params.z, 0.0);
        let dist_inward = max(-dist_body - inner_spread, 0.0);
        let inner_factor = 1.0 - smoothstep(0.0, inner_blur, dist_inward);
        let glow_alpha = inner_factor * material.inner_glow_color.a;
        body_color = vec4<f32>(
            mix(body_color.rgb, material.inner_glow_color.rgb, glow_alpha),
            max(body_color.a, glow_alpha),
        );
        inner_glow_emissive = material.inner_glow_color.rgb * (glow_alpha * body_alpha * 2.6);
    }

    // Shape compositing: Border + Body without seams or neon bleed
    var eff_border_color = material.border_color;
    if (border_width > 0.0 && material.grad_params.z > 0.0 && material.border_color.a == 0.0) {
        eff_border_color = body_color;
    }
    let shape_rgb_premul = eff_border_color.rgb * (border_alpha * eff_border_color.a)
        + body_color.rgb * (body_alpha * body_color.a);
    let shape_alpha = clamp(border_alpha * eff_border_color.a + body_alpha * body_color.a, 0.0, 1.0);
    let shape_straight_rgb = shape_rgb_premul / max(shape_alpha, 0.0001);

    // Outer shadow (premultiplied independently so shadows never adopt fill/border color)
    var shadow_alpha = 0.0;
    if (shadow_blur > 0.0 || shadow_spread > 0.0) {
        let p_shadow = p - vec2<f32>(shadow_offset.x, -shadow_offset.y);
        let shadow_radius = material.radius + vec4<f32>(shadow_spread);
        let half_size_shadow = half_size + vec2<f32>(shadow_spread);
        let dist_shadow = sd_box_dynamic(p_shadow, half_size_shadow, shadow_radius, material.shape_mode);
        let shadow_edge = max(shadow_blur, aa_inner);
        let shadow_factor = 1.0 - smoothstep(-aa_inner, shadow_edge, dist_shadow);
        shadow_alpha = shadow_factor * material.shadow_color.a;
    }

    let double_sided = (material.lighting_flags & 1u) != 0u;
    let receive_shadows = (material.lighting_flags & 2u) != 0u;

    var pbr_input = pbr_types::pbr_input_new();
@if(VERTEX_OUTPUT_INSTANCE_INDEX) {
    pbr_input.flags = mesh_bindings::mesh[in.instance_index].flags;
} @else {
    pbr_input.flags = mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT;
}
    if (!receive_shadows) {
        pbr_input.flags = pbr_input.flags & (~mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT);
    }

    let is_logically_front = pbr_functions::winding_corrected_front_facing(pbr_input.flags, is_front);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.is_orthographic = mesh_view_bindings::view.clip_from_view[3].w == 1.0;

    var world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        double_sided,
        is_logically_front,
    );

    // Procedural 3D SDF edge bevel normals near the panel outer perimeter
    let bevel_width = material.bevel_params.x;
    let bevel_strength = clamp(material.bevel_params.y, 0.0, 1.0);
    if (material.pbr_enabled > 0u && bevel_width > 0.00001 && bevel_strength > 0.001 && dist_outer < bevel_width) {
        let inward = clamp((bevel_width + dist_outer) / bevel_width, 0.0, 1.0);
        if (inward > 0.0001) {
            let eps = max(bevel_width * 0.08, softness);
            let dx = sd_box_dynamic(p + vec2<f32>(eps, 0.0), half_size, material.radius, material.shape_mode)
                - sd_box_dynamic(p - vec2<f32>(eps, 0.0), half_size, material.radius, material.shape_mode);
            let dy = sd_box_dynamic(p + vec2<f32>(0.0, eps), half_size, material.radius, material.shape_mode)
                - sd_box_dynamic(p - vec2<f32>(0.0, eps), half_size, material.radius, material.shape_mode);
            let grad_2d = vec2<f32>(dx, dy);
            let grad_len = length(grad_2d);
            if (grad_len > 1.0e-5) {
                let n2d = grad_2d / grad_len;
                let slope = bevel_strength * inward * 0.85;
                let z_sign = select(-1.0, 1.0, !double_sided || is_logically_front);
                let local_n = normalize(vec3<f32>(n2d.x * slope, n2d.y * slope, z_sign));
@if(VERTEX_OUTPUT_INSTANCE_INDEX) {
                let world_from_local = mesh_functions::get_world_from_local(in.instance_index);
                world_normal = normalize((world_from_local * vec4<f32>(local_n, 0.0)).xyz);
}
            }
        }
    }

    pbr_input.world_normal = world_normal;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
    pbr_input.N = normalize(pbr_input.world_normal);

    var mat_flags = pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    if (double_sided) {
        mat_flags = mat_flags | pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;
    }
    pbr_input.material.flags = mat_flags;
    pbr_input.material.reflectance = vec3<f32>(material.reflectance);

    let geom_alpha = clamp(border_alpha + body_alpha * (1.0 - border_alpha), 0.0, 1.0);
    let eff_material_alpha = shape_alpha / max(geom_alpha, 0.0001);

    var shape_premul = vec4<f32>(0.0);
    if (shape_alpha > 0.0001) {
        var lit_shape_rgb = shape_straight_rgb + inner_glow_emissive * 0.75;
        var out_rgb_scale = shape_alpha;
        if (material.pbr_enabled > 0u) {
            // Scale diffuse/metallic albedo by material opacity while keeping dielectric Fresnel
            // specular unscaled so translucent glass retains crisp glossy highlights.
            let weighted_rgb = shape_straight_rgb * select(1.0, eff_material_alpha, eff_material_alpha < 0.999);
            pbr_input.material.base_color = vec4<f32>(weighted_rgb, 1.0);
            pbr_input.material.metallic = material.metallic;
            pbr_input.material.perceptual_roughness = material.roughness;
            // Bevy PBR stores `emissive_exposure_weight` in `emissive.a` (`0.0` = HDR direct,
            // `1.0` = attenuated by `view.exposure`). Keep `.a = 0.0` so HDR neon/lamp glows
            // emit at full strength into Bloom instead of being crushed by ~0.001 default exposure.
            pbr_input.material.emissive = vec4<f32>(
                (material.emissive.rgb + inner_glow_emissive) * eff_material_alpha,
                0.0,
            );
            lit_shape_rgb = pbr_functions::apply_pbr_lighting(pbr_input).rgb;
            out_rgb_scale = geom_alpha;
        }

        let post_shape_rgb = pbr_functions::main_pass_post_lighting_processing(
            pbr_input,
            vec4<f32>(lit_shape_rgb, 1.0),
        ).rgb;
        shape_premul = vec4<f32>(post_shape_rgb * out_rgb_scale, shape_alpha);
    }

    var shadow_premul = vec4<f32>(0.0);
    if (shadow_alpha > 0.0001) {
        let post_shadow_rgb = pbr_functions::main_pass_post_lighting_processing(
            pbr_input,
            vec4<f32>(material.shadow_color.rgb, 1.0),
        ).rgb;
        shadow_premul = vec4<f32>(post_shadow_rgb * shadow_alpha, shadow_alpha);
    }

    // Porter-Duff Over (Shape over Shadow) with AlphaMode::Premultiplied output
    let out_color = (shape_premul + shadow_premul * (1.0 - shape_alpha)) * alpha_clip;
    if (out_color.a <= 0.001) {
        discard;
    }
    return out_color;
}
