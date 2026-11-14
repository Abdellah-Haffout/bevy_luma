use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::*;

/// Pipeline specialization key for [`UNodeMaterial`] allowing per-node custom fragment shader overrides.
#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct UNodeMaterialKey {
    /// Optional custom fragment shader overriding the default `unode.wgsl`.
    pub custom_shader: Option<Handle<Shader>>,
}

impl From<&UNodeMaterial> for UNodeMaterialKey {
    fn from(material: &UNodeMaterial) -> Self {
        Self {
            custom_shader: material.custom_shader.clone(),
        }
    }
}

/// Custom 2D material shader for Univis UI nodes.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(UNodeMaterialKey)]
pub struct UNodeMaterial {
    // Group 1: Vectors (16 bytes)
    /// Base background color.
    #[uniform(0)]
    pub color: LinearRgba,
    /// Color of the node border.
    #[uniform(0)]
    pub border_color: LinearRgba,
    /// Independent border radii (top-right, bottom-right, top-left, bottom-left).
    #[uniform(0)]
    pub radius: Vec4,

    // Group 2: Mixed
    /// Exact width and height of the node.
    #[uniform(0)]
    pub size: Vec2, // 8 bytes (Offset 48)

    // Group 3: Floats
    /// Thickness of the border stroke.
    #[uniform(0)]
    pub border_width: f32, // 4 bytes (Offset 56)
    /// Offset distance of the border from the element bounds.
    #[uniform(0)]
    pub border_offset: f32, // 4 bytes (Offset 60)
    /// Edge softness for anti-aliasing the SDF rendering.
    #[uniform(0)]
    pub softness: f32, // 4 bytes (Offset 64)

    // Group 4: Integers (u32)
    /// Shape mode selector (0 = Round, 1 = Cut).
    #[uniform(0)]
    pub shape_mode: u32, // 4 bytes (Offset 68)
    /// Flag indicating whether the texture binding should be sampled.
    #[uniform(0)]
    pub use_texture: u32, // 4 bytes (Offset 72)

    /// Final padding to close the uniform block at 80 bytes.
    #[uniform(0)]
    pub _pad: f32, // 4 bytes (Offset 76)

    /// Optional texture to draw inside the node bounds.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,

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

    // --- Multi-stop gradient extensions (Offset 128) ---
    /// Up to 8 gradient stop colors (LinearRgba packed as Vec4).
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
    pub shadow_color: LinearRgba,
    /// Shadow parameters: x = offset.x, y = offset.y, z = blur, w = spread.
    #[uniform(0)]
    pub shadow_params: Vec4,

    /// Inner holographic glow color.
    #[uniform(0)]
    pub inner_glow_color: LinearRgba,
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

    /// Optional specialized fragment shader handle compiled by `UShaderCache`.
    pub custom_shader: Option<Handle<Shader>>,
}

impl Default for UNodeMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE.into(),
            border_color: Color::BLACK.into(),
            radius: Vec4::splat(10.0),
            size: Vec2::new(100.0, 100.0),
            border_width: 0.0,
            border_offset: 0.0,
            softness: 0.5,
            _pad: 0.0,
            texture: None,
            use_texture: 0,
            shape_mode: 0,
            clip_inv_x: [Vec4::ZERO; 4],
            clip_inv_y: [Vec4::ZERO; 4],
            clip_bounds: [Vec4::ZERO; 4],
            clip_radius: [Vec4::ZERO; 4],
            grad_colors: [Vec4::ZERO; 8],
            grad_stops: [Vec4::ZERO; 2],
            grad_params: Vec4::ZERO,
            grad_center: Vec4::ZERO,
            shadow_color: LinearRgba::NONE,
            shadow_params: Vec4::ZERO,
            inner_glow_color: LinearRgba::NONE,
            inner_glow_params: Vec4::ZERO,
            custom_params: [Vec4::ZERO; 4],
            custom_meta: Vec4::ZERO,
            custom_cursor: Vec4::new(0.5, 0.5, 0.0, 0.0),
            custom_shader: None,
        }
    }
}

impl Material2d for UNodeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://luma_engine/layout/render/shaders/unode.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        // Blend keeps transparency, soft edges, and shadow falloff intact.
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(custom_shader) = &key.bind_group_data.custom_shader
            && let Some(fragment) = &mut descriptor.fragment
        {
            fragment.shader = custom_shader.clone();
        }
        Ok(())
    }
}
