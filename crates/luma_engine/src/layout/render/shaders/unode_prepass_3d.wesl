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

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: UNodeMaterial3d;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var base_sampler: sampler;

fn sd_box_dynamic(p: vec2<f32>, b: vec2<f32>, r_in: vec4<f32>, mode: u32) -> f32 {
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

fn evaluate_sdf_alpha(in: prepass_io::VertexOutput) -> f32 {
@if(!PREPASS_FRAGMENT) {
    if ((material.lighting_flags & 4u) == 0u) {
        return -1.0;
    }
}

@if(!VERTEX_UVS_A) {
    return 0.0;
} @else {
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
    let aa_inner = max(material.softness, 0.00001);

    // Clipping stack
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
        alpha_clip = alpha_clip * (1.0 - smoothstep(-aa_inner, aa_inner, d_clip));
    }

    let border_outer_alpha = 1.0 - smoothstep(-aa_inner, aa_inner, dist_outer);
    let dist_body = dist_outer + material.border_width + material.border_offset;
    let body_coverage = select(
        border_outer_alpha,
        min(1.0 - smoothstep(-aa_inner, aa_inner, dist_body), border_outer_alpha),
        material.border_width > 0.0 || material.border_offset > 0.0,
    );
    let border_inner_alpha = 1.0 - smoothstep(-aa_inner, aa_inner, dist_outer + material.border_width);
    let border_coverage = select(
        0.0,
        max(border_outer_alpha - border_inner_alpha, 0.0),
        material.border_width > 0.0,
    );

    var fill_alpha = material.color.a;
    if (material.grad_params.z > 0.0 && fill_alpha <= 0.0001) {
        fill_alpha = 1.0;
    }
    if (material.use_texture > 0u) {
        let body_uv = (p / max(material.size, vec2<f32>(0.00001))) * vec2<f32>(1.0, -1.0) + 0.5;
        let tex_uv = clamp(body_uv, vec2<f32>(0.0), vec2<f32>(1.0));
        fill_alpha *= textureSampleLevel(base_texture, base_sampler, tex_uv, 0.0).a;
    }
    let shape_alpha = clamp(
        border_coverage * material.border_color.a + body_coverage * fill_alpha,
        0.0,
        1.0,
    );
    return shape_alpha * alpha_clip;
}
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(
    in: prepass_io::VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> prepass_io::FragmentOutput {
    if (evaluate_sdf_alpha(in) < material.bevel_params.z) {
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
    if (evaluate_sdf_alpha(in) < material.bevel_params.z) {
        discard;
    }
}
