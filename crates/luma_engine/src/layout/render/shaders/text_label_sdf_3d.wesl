import bevy_pbr::render::{
    pbr_types,
    pbr_functions,
    forward_io::VertexOutput,
};

struct UTextLabelSdfMaterial3d {
    color: vec4<f32>,
    outline_color: vec4<f32>,
    edge_softness: f32,
    outline_width: f32,
    _pad0: f32,
    _pad1: f32,
    clip_inv_x: array<vec4<f32>, 4>,
    clip_inv_y: array<vec4<f32>, 4>,
    clip_bounds: array<vec4<f32>, 4>,
    clip_radius: array<vec4<f32>, 4>,
};

@group(3) @binding(0) var<uniform> material: UTextLabelSdfMaterial3d;
@group(3) @binding(1) var text_texture: texture_2d<f32>;
@group(3) @binding(2) var text_sampler: sampler;

fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;

    let r_top = select(r.z, r.x, is_right);
    let r_bottom = select(r.w, r.y, is_right);
    let radius = select(r_bottom, r_top, is_top);

    let q = abs(p) - b + radius;
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn sd_cut_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    let is_right = p.x > 0.0;
    let is_top = p.y > 0.0;

    let r_top = select(r.z, r.x, is_right);
    let r_bottom = select(r.w, r.y, is_right);
    let radius = select(r_bottom, r_top, is_top);

    let q = abs(p) - b;
    let d_box = length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0);
    let d_cut = (abs(p.x) + abs(p.y) - (b.x + b.y - radius)) * 0.70710678;
    return max(d_box, d_cut);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
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

    if (material.outline_width > 0.0 && material.outline_color.a > 0.0) {
        let out_a0 = smoothstep(-edge, edge, dist0 + material.outline_width);
        let out_a1 = smoothstep(-edge, edge, dist1 + material.outline_width);
        let out_a2 = smoothstep(-edge, edge, dist2 + material.outline_width);
        let out_a3 = smoothstep(-edge, edge, dist3 + material.outline_width);
        let outline_alpha = (out_a0 + out_a1 + out_a2 + out_a3) * 0.25;

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

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;

    // Self-lit UI text preserves 2D sharpness/contrast while matching Bevy 3D tonemapping and exposure.
    let processed = pbr_functions::main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(shape_premul.rgb, 1.0),
    );
    if (final_alpha <= 0.001) {
        discard;
    }

    return vec4<f32>(processed.rgb * final_alpha, final_alpha);
}
