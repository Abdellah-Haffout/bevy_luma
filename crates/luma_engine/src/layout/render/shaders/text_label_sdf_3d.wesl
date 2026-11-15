import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_bindings,
    mesh_functions,
    mesh_types,
    mesh_view_bindings::view,
    pbr_functions,
    pbr_types,
};

struct UTextLabelSdfMaterial3d {
    color: vec4<f32>,
    outline_color: vec4<f32>,
    emissive: vec4<f32>,
    edge_softness: f32,
    outline_width: f32,
    metallic: f32,
    roughness: f32,
    reflectance: f32,
    pbr_enabled: u32,
    lighting_flags: u32,
    _pad_lighting: f32,
    bevel_params: vec4<f32>,
    clip_inv_x: array<vec4<f32>, 4>,
    clip_inv_y: array<vec4<f32>, 4>,
    clip_bounds: array<vec4<f32>, 4>,
    clip_radius: array<vec4<f32>, 4>,
};

@group(3) @binding(0) var<uniform> material: UTextLabelSdfMaterial3d;
@group(3) @binding(1) var text_texture: texture_2d<f32>;
@group(3) @binding(2) var text_sampler: sampler;

fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    let limit = max(min(b.x, b.y), 0.0);
    let r_clamped = min(max(r, vec4<f32>(0.0)), vec4<f32>(limit));

    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;

    let r_top = select(r_clamped.z, r_clamped.x, is_right);
    let r_bottom = select(r_clamped.w, r_clamped.y, is_right);
    let radius = select(r_bottom, r_top, is_top);

    let q = abs(p) - b + radius;
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn sd_cut_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    let limit = max(min(b.x, b.y), 0.0);
    let r_clamped = min(max(r, vec4<f32>(0.0)), vec4<f32>(limit));

    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;

    let r_top = select(r_clamped.z, r_clamped.x, is_right);
    let r_bottom = select(r_clamped.w, r_clamped.y, is_right);
    let radius = select(r_bottom, r_top, is_top);

    let q = abs(p) - b;
    let d_box = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0);
    let d_cut = (abs(p.x) + abs(p.y) - (b.x + b.y - radius)) * 0.70710678;
    return max(d_box, d_cut);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(text_texture));
    let uv_dx = dpdx(in.uv);
    let uv_dy = dpdy(in.uv);

    // Isotropic screen-space derivative in atlas texels per screen pixel
    let d_tex_x = uv_dx * tex_size;
    let d_tex_y = uv_dy * tex_size;
    let texels_per_px = length(vec2<f32>(length(d_tex_x), length(d_tex_y))) * 0.70710678;
    // Spread is 12 texels, distance is normalized in [-0.5, 0.5] over 24 texels
    let ddist = max(texels_per_px / 24.0, 0.0001);

    // Filter edge softness
    let edge = max(ddist * 0.5 * max(material.edge_softness, 0.1), 0.0001);

    // Rotated Grid Super-Sampling (RGSS 4-tap) offsets in screen pixels:
    // (-1/8, -3/8), (3/8, -1/8), (-3/8, 1/8), (1/8, 3/8)
    let o0 = uv_dx * -0.125 + uv_dy * -0.375;
    let o1 = uv_dx *  0.375 + uv_dy * -0.125;
    let o2 = uv_dx * -0.375 + uv_dy *  0.125;
    let o3 = uv_dx *  0.125 + uv_dy *  0.375;

    let s0 = textureSampleLevel(text_texture, text_sampler, in.uv + o0, 0.0).a;
    let s1 = textureSampleLevel(text_texture, text_sampler, in.uv + o1, 0.0).a;
    let s2 = textureSampleLevel(text_texture, text_sampler, in.uv + o2, 0.0).a;
    let s3 = textureSampleLevel(text_texture, text_sampler, in.uv + o3, 0.0).a;

    let dist0 = s0 - 0.5;
    let dist1 = s1 - 0.5;
    let dist2 = s2 - 0.5;
    let dist3 = s3 - 0.5;

    let a0 = smoothstep(-edge, edge, dist0);
    let a1 = smoothstep(-edge, edge, dist1);
    let a2 = smoothstep(-edge, edge, dist2);
    let a3 = smoothstep(-edge, edge, dist3);
    let fill_alpha = (a0 + a1 + a2 + a3) * 0.25;

@if(VERTEX_COLORS)
    let fill_color = material.color * in.color;
@else
    let fill_color = material.color;

    var shape_premul: vec4<f32>;
    var geom_alpha = fill_alpha;

    if (material.outline_width > 0.0 && material.outline_color.a > 0.0) {
        let out_a0 = smoothstep(-edge, edge, dist0 + material.outline_width);
        let out_a1 = smoothstep(-edge, edge, dist1 + material.outline_width);
        let out_a2 = smoothstep(-edge, edge, dist2 + material.outline_width);
        let out_a3 = smoothstep(-edge, edge, dist3 + material.outline_width);
        let outline_alpha = (out_a0 + out_a1 + out_a2 + out_a3) * 0.25;

        geom_alpha = clamp(fill_alpha + outline_alpha * (1.0 - fill_alpha), 0.0, 1.0);
        let fill_cov = fill_alpha * fill_color.a;
        let outline_cov = outline_alpha * (1.0 - fill_alpha) * material.outline_color.a;
        let alpha = clamp(fill_cov + outline_cov, 0.0, 1.0);
        let rgb = (fill_color.rgb * fill_cov + material.outline_color.rgb * outline_cov)
            / max(alpha, 0.0001);
        shape_premul = vec4<f32>(rgb, alpha);
    } else {
        shape_premul = vec4<f32>(fill_color.rgb, fill_color.a * fill_alpha);
    }

    var clip_alpha = 1.0;
    let clip_wp = vec4<f32>(in.world_position.xyz, 1.0);
    for (var i = 0u; i < 4u; i = i + 1u) {
        let bounds = material.clip_bounds[i];
        if (bounds.z < 0.5) {
            break;
        }
        let clip_p = vec2<f32>(
            dot(material.clip_inv_x[i], clip_wp),
            dot(material.clip_inv_y[i], clip_wp),
        );
        let d_round = sd_rounded_box(clip_p, bounds.xy, material.clip_radius[i]);
        let d_cut = sd_cut_box(clip_p, bounds.xy, material.clip_radius[i]);
        let clip_distance = select(d_round, d_cut, bounds.w > 0.5);
        let clip_ddist = length(vec2<f32>(dpdx(clip_distance), dpdy(clip_distance)));
        let clip_edge = max(clip_ddist * 0.5, 0.00001);
        clip_alpha = clip_alpha * (1.0 - smoothstep(-clip_edge, clip_edge, clip_distance));
    }
    let final_alpha = shape_premul.a * clip_alpha;
    if (final_alpha <= 0.001) {
        discard;
    }

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;

    let double_sided = (material.lighting_flags & 1u) != 0u;
    let receive_shadows = (material.lighting_flags & 2u) != 0u;

    var mesh_flags = 0u;
@if(VERTEX_OUTPUT_INSTANCE_INDEX) {
    mesh_flags = mesh_bindings::mesh[in.instance_index].flags;
    if (receive_shadows) {
        pbr_input.flags = mesh_flags;
    } else {
        pbr_input.flags = mesh_flags & (~mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT);
    }
}

    let is_logically_front = pbr_functions::winding_corrected_front_facing(mesh_flags, is_front);
    var world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        double_sided,
        is_logically_front,
    );

    let bevel_width = material.bevel_params.x;
    let bevel_strength = material.bevel_params.y;
    if (material.pbr_enabled > 0u && bevel_width > 0.0 && bevel_strength > 0.0) {
        let s_avg = (s0 + s1 + s2 + s3) * 0.25;
        let bevel_range = clamp(bevel_width * 0.035, 0.015, 0.45);
        let dist_in = s_avg - 0.5;
        if (dist_in >= -0.02 && dist_in < bevel_range) {
            let inward = smoothstep(bevel_range, 0.0, max(dist_in, 0.0));
            let texel = 0.75 / max(tex_size, vec2<f32>(1.0));
            let sx_p = textureSampleLevel(text_texture, text_sampler, in.uv + vec2<f32>(texel.x, 0.0), 0.0).a;
            let sx_n = textureSampleLevel(text_texture, text_sampler, in.uv - vec2<f32>(texel.x, 0.0), 0.0).a;
            let sy_p = textureSampleLevel(text_texture, text_sampler, in.uv + vec2<f32>(0.0, texel.y), 0.0).a;
            let sy_n = textureSampleLevel(text_texture, text_sampler, in.uv - vec2<f32>(0.0, texel.y), 0.0).a;
            let grad_2d = vec2<f32>(-(sx_p - sx_n), sy_p - sy_n);
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

    let eff_material_alpha = clamp(shape_premul.a / max(geom_alpha, 0.0001), 0.0, 1.0);
    var lit_rgb = shape_premul.rgb;
    var out_rgb_scale = final_alpha;
    if (material.pbr_enabled > 0u) {
        let weighted_rgb = shape_premul.rgb * select(1.0, eff_material_alpha, eff_material_alpha < 0.999);
        pbr_input.material.base_color = vec4<f32>(weighted_rgb, 1.0);
        pbr_input.material.metallic = material.metallic;
        pbr_input.material.perceptual_roughness = material.roughness;
        // Keep `emissive.a = 0.0` (`emissive_exposure_weight`) so HDR text glow isn't dimmed by view exposure.
        pbr_input.material.emissive = vec4<f32>(material.emissive.rgb * eff_material_alpha, 0.0);
        lit_rgb = pbr_functions::apply_pbr_lighting(pbr_input).rgb;
        out_rgb_scale = clamp(geom_alpha * clip_alpha, 0.0, 1.0);
    }

    let processed = pbr_functions::main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(lit_rgb, 1.0),
    );
    return vec4<f32>(processed.rgb * out_rgb_scale, final_alpha);
}
