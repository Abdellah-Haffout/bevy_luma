import bevy_pbr::prepass::io as prepass_io;

@if(NORMAL_PREPASS)
import bevy_pbr::render::{
    mesh_bindings::mesh,
    mesh_types::MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT,
};

@if(MOTION_VECTOR_PREPASS)
import bevy_pbr::render::pbr_prepass_functions;

@if(DEFERRED_PREPASS)
import bevy_pbr::render::rgb9e5;

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
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: UTextLabelSdfMaterial3d;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var sdf_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var sdf_sampler: sampler;

fn sd_clip_box_dynamic(p: vec2<f32>, b: vec2<f32>, r_in: vec4<f32>, mode: u32) -> f32 {
    let limit = max(min(b.x, b.y), 0.0);
    let r_vec = min(max(r_in, vec4<f32>(0.0)), vec4<f32>(limit));

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

fn evaluate_text_clip(world_pos: vec3<f32>) -> f32 {
    var alpha_clip = 1.0;
    let clip_wp = vec4<f32>(world_pos, 1.0);
    for (var i = 0u; i < 4u; i = i + 1u) {
        let bounds = material.clip_bounds[i];
        if (bounds.z < 0.5) {
            break;
        }
        let p_clip = vec2<f32>(
            dot(material.clip_inv_x[i], clip_wp),
            dot(material.clip_inv_y[i], clip_wp),
        );
        let d_clip = sd_clip_box_dynamic(
            p_clip,
            bounds.xy,
            material.clip_radius[i],
            u32(bounds.w + 0.5),
        );
        alpha_clip = alpha_clip * (1.0 - smoothstep(-0.001, 0.001, d_clip));
    }
    return alpha_clip;
}

fn evaluate_text_sdf_alpha(in: prepass_io::VertexOutput) -> f32 {
@if(!PREPASS_FRAGMENT) {
    if ((material.lighting_flags & 4u) == 0u) {
        return -1.0;
    }
}

@if(!VERTEX_UVS_A) {
    return 0.0;
} @else {
    let sdf_val = textureSampleLevel(sdf_texture, sdf_sampler, in.uv, 0.0).a;
    let clip_alpha = evaluate_text_clip(in.world_position.xyz);

    let half_w = max(material.edge_softness * 0.035, 0.015);
    let fill_cov = smoothstep(0.5 - half_w, 0.5 + half_w, sdf_val) * material.color.a;
    var cov = fill_cov;
    if (material.outline_width > 0.0) {
        let outer_edge = clamp(0.5 - material.outline_width, 0.02, 0.5);
        let outer_cov = smoothstep(outer_edge - half_w, outer_edge + half_w, sdf_val)
            * material.outline_color.a;
        cov = max(fill_cov, outer_cov);
    }
    return cov * clip_alpha;
}
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(
    in: prepass_io::VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> prepass_io::FragmentOutput {
    if (evaluate_text_sdf_alpha(in) < material.bevel_params.z) {
        discard;
    }

    var out: prepass_io::FragmentOutput;
@if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.frag_depth = in.unclipped_depth;
}
@if(NORMAL_PREPASS) {
    let double_sided = (material.lighting_flags & 1u) != 0u;
    let mesh_flags = mesh[in.instance_index].flags;
    let positive_determinant = (mesh_flags & MESH_FLAGS_SIGN_DETERMINANT_MODEL_3X3_BIT) != 0u;
    let is_logically_front = is_front == positive_determinant;
    let world_normal = (f32(!double_sided || is_logically_front) * 2.0 - 1.0) * in.world_normal;
    out.normal = vec4<f32>(world_normal * 0.5 + vec3<f32>(0.5), 1.0);
}
@if(MOTION_VECTOR_PREPASS) {
    out.motion_vector = pbr_prepass_functions::calculate_motion_vector(
        in.world_position,
        in.previous_world_position,
    );
}
@if(DEFERRED_PREPASS) {
    out.deferred = vec4<u32>(0u, rgb9e5::vec3_to_rgb9e5_(material.emissive.rgb), 0u, 0u);
    out.deferred_lighting_pass_id = 1u;
}
    return out;
}
@else
@fragment
fn fragment(in: prepass_io::VertexOutput) {
    if (evaluate_text_sdf_alpha(in) < material.bevel_params.z) {
        discard;
    }
}
