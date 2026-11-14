use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Pipeline specialization key for [`UNodeMaterial3d`] allowing per-node custom fragment shader overrides.
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct UNodeMaterial3dKey {
    /// Optional custom fragment shader overriding the default `unode_3d.wesl`.
    pub custom_shader: Option<Handle<Shader>>,
}

impl From<&UNodeMaterial3d> for UNodeMaterial3dKey {
    fn from(material: &UNodeMaterial3d) -> Self {
        Self {
            custom_shader: material.custom_shader.clone(),
        }
    }
}

/// Custom 3D material shader for Univis UI nodes in the 3D world.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(UNodeMaterial3dKey)]
pub struct UNodeMaterial3d {
    // --- group 1: Vec4 values (16-byte aligned) ---
    /// Base background color.
    #[uniform(0)]
    pub color: Vec4,
    /// Independent border radii.
    #[uniform(0)]
    pub radius: Vec4,
    /// Color of the node border.
    #[uniform(0)]
    pub border_color: Vec4,
    /// Emissive color glow for PBR.
    #[uniform(0)]
    pub emissive: Vec4,

    // --- group 2: 16-byte aligned scalar/vec2 blocks ---
    /// Exact width and height of the node.
    #[uniform(0)]
    pub size: Vec2,
    /// Thickness of the border stroke.
    #[uniform(0)]
    pub border_width: f32,
    /// Offset distance of the border from the element bounds.
    #[uniform(0)]
    pub border_offset: f32,

    /// Edge softness for anti-aliasing the SDF rendering.
    #[uniform(0)]
    pub softness: f32,
    /// Metallic factor for PBR.
    #[uniform(0)]
    pub metallic: f32,
    /// Roughness factor for PBR.
    #[uniform(0)]
    pub roughness: f32,
    /// Flag indicating whether the texture binding should be sampled.
    #[uniform(0)]
    pub use_texture: u32,

    /// Shape mode selector (0 = Round, 1 = Cut).
    #[uniform(0)]
    pub shape_mode: u32,
    /// Enables physical PBR scene lighting (`UPbr`); `0` uses clean self-lit UI rendering.
    #[uniform(0)]
    pub pbr_enabled: u32,
    /// Padding for 16-byte uniform alignment.
    #[uniform(0)]
    pub _pad0: f32,
    /// Padding for 16-byte uniform alignment.
    #[uniform(0)]
    pub _pad1: f32,

    // --- visual extensions (16-byte aligned blocks) ---
    /// Up to 8 gradient stop colors (RGBA).
    #[uniform(0)]
    pub grad_colors: [Vec4; 8],
    /// Up to 8 gradient stop positions packed into 2 Vec4 vectors.
    #[uniform(0)]
    pub grad_stops: [Vec4; 2],
    /// Gradient parameters: x = type (0=none, 1=linear, 2=radial), y = angle/radius, z = stop count, w = 0.0.
    #[uniform(0)]
    pub grad_params: Vec4,
    /// Radial gradient center coordinates: xy = center, zw = 0.0.
    #[uniform(0)]
    pub grad_center: Vec4,

    /// Shadow or outer glow color.
    #[uniform(0)]
    pub shadow_color: Vec4,
    /// Shadow parameters: x = offset.x, y = offset.y, z = blur, w = spread.
    #[uniform(0)]
    pub shadow_params: Vec4,

    /// Inner holographic glow color.
    #[uniform(0)]
    pub inner_glow_color: Vec4,
    /// Inner glow parameters: x = blur, y = active (1.0 or 0.0), z = spread, w = 0.0.
    #[uniform(0)]
    pub inner_glow_params: Vec4,

    /// Custom shader user uniform vectors (`params[0]..params[3]` in `UShader`).
    #[uniform(0)]
    pub custom_params: [Vec4; 4],
    /// Custom shader metadata: x = time, y = blend_mode, z = opacity, w = active (1.0 or 0.0).
    #[uniform(0)]
    pub custom_meta: Vec4,
    /// Custom shader mouse state: xy = local cursor UV, z = smooth hover (0..1), w = smooth press (0..1).
    #[uniform(0)]
    pub custom_cursor: Vec4,

    /// Row 0 of up to 4 nested clippers' inverse world transforms (`[x, y, z, w]`).
    #[uniform(0)]
    pub clip_inv_x: [Vec4; 4],
    /// Row 1 of up to 4 nested clippers' inverse world transforms (`[x, y, z, w]`).
    #[uniform(0)]
    pub clip_inv_y: [Vec4; 4],
    /// Clipper local bounds & flags: `xy = half_size`, `z = active (1.0/0.0)`, `w = shape_mode (0=round, 1=cut)`.
    #[uniform(0)]
    pub clip_bounds: [Vec4; 4],
    /// Clipper corner radii `(tr, br, tl, bl)` in clipper-local units.
    #[uniform(0)]
    pub clip_radius: [Vec4; 4],

    // --- texture bindings ---
    /// Optional texture to draw inside the node bounds.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,

    /// Optional specialized fragment shader handle compiled by `UShaderCache`.
    pub custom_shader: Option<Handle<Shader>>,
}

impl Default for UNodeMaterial3d {
    fn default() -> Self {
        Self {
            color: Vec4::ONE,
            radius: Vec4::ZERO,
            border_color: Vec4::ZERO,
            emissive: Vec4::ZERO,
            size: Vec2::new(1.0, 1.0),
            border_width: 0.0,
            border_offset: 0.0,
            softness: 1.0e-5,
            metallic: 0.0,
            roughness: 0.5,
            use_texture: 0,
            shape_mode: 0,
            pbr_enabled: 0,
            _pad0: 0.0,
            _pad1: 0.0,
            grad_colors: [Vec4::ZERO; 8],
            grad_stops: [Vec4::ZERO; 2],
            grad_params: Vec4::ZERO,
            grad_center: Vec4::ZERO,
            shadow_color: Vec4::ZERO,
            shadow_params: Vec4::ZERO,
            inner_glow_color: Vec4::ZERO,
            inner_glow_params: Vec4::ZERO,
            custom_params: [Vec4::ZERO; 4],
            custom_meta: Vec4::ZERO,
            custom_cursor: Vec4::new(0.5, 0.5, 0.0, 0.0),
            clip_inv_x: [Vec4::ZERO; 4],
            clip_inv_y: [Vec4::ZERO; 4],
            clip_bounds: [Vec4::ZERO; 4],
            clip_radius: [Vec4::ZERO; 4],
            texture: None,
            custom_shader: None,
        }
    }
}

impl Material for UNodeMaterial3d {
    fn fragment_shader() -> ShaderRef {
        "embedded://luma_engine/layout/render/shaders/unode_3d.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(custom_shader) = &key.bind_group_data.custom_shader
            && let Some(fragment) = &mut descriptor.fragment
        {
            fragment.shader = custom_shader.clone();
        }
        Ok(())
    }
}
