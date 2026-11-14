#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use super::clip_sync::{
    ClipUniformData, ClipUniformTarget, collect_active_cursor_windows, collect_clip_stack_for_entity,
    project_screen_point_to_node, resolve_node_world_scale,
};
use crate::internal_prelude::*;
use crate::layout::query::UiPickingContext;
use bevy::camera::RenderTarget;
use bevy::window::PrimaryWindow;
use bevy::{ecs::relationship::Relationship, prelude::*};
use std::collections::HashMap;

const UNODE_HOST_SHADER_TEMPLATE_2D: &str = include_str!("shaders/unode.wgsl");
const UNODE_HOST_SHADER_TEMPLATE_3D: &str = include_str!("shaders/unode_3d.wesl");
const HOOK_START_MARKER: &str = "// __LUMA_CUSTOM_SHADER_HOOK_START__";
const HOOK_END_MARKER: &str = "// __LUMA_CUSTOM_SHADER_HOOK_END__";

fn inject_custom_shader_hook(template: &str, snippet: &str) -> String {
    let injected_hook = if snippet.contains("fn sample_custom_shader") {
        snippet.to_string()
    } else {
        format!(
            "fn sample_custom_shader(\n    uv: vec2<f32>,\n    size: vec2<f32>,\n    time: f32,\n    params: array<vec4<f32>, 4>,\n) -> vec4<f32> {{\n{snippet}\n}}"
        )
    };

    if let (Some(start), Some(end)) = (template.find(HOOK_START_MARKER), template.find(HOOK_END_MARKER)) {
        let mut composed = String::with_capacity(template.len() + injected_hook.len());
        composed.push_str(&template[..start]);
        composed.push_str(&injected_hook);
        composed.push_str(&template[end + HOOK_END_MARKER.len()..]);
        composed
    } else {
        template.to_string()
    }
}

/// Caches compiled `Handle<Shader>` instances for inline WGSL and asset-path [`UShader`] components.
#[derive(Resource, Default)]
pub struct UShaderCache {
    by_hash: HashMap<(bool, u64), Handle<Shader>>,
    by_path: HashMap<String, Handle<Shader>>,
}

impl UShaderCache {
    /// Resolves or compiles a custom 2D fragment shader handle for the given [`UShaderSource`].
    pub fn resolve(
        &mut self,
        source: &UShaderSource,
        shaders: &mut Assets<Shader>,
        asset_server: Option<&AssetServer>,
    ) -> Option<Handle<Shader>> {
        self.resolve_for_mode(source, false, shaders, asset_server)
    }

    /// Resolves or compiles a custom 2D (`is_3d == false`) or 3D (`is_3d == true`) fragment shader handle.
    pub fn resolve_for_mode(
        &mut self,
        source: &UShaderSource,
        is_3d: bool,
        shaders: &mut Assets<Shader>,
        asset_server: Option<&AssetServer>,
    ) -> Option<Handle<Shader>> {
        match source {
            UShaderSource::Handle(handle) => Some(handle.clone()),
            UShaderSource::AssetPath(path) => {
                if let Some(existing) = self.by_path.get(path) {
                    return Some(existing.clone());
                }
                let server = asset_server?;
                let handle: Handle<Shader> = server.load(path.clone());
                self.by_path.insert(path.clone(), handle.clone());
                Some(handle)
            }
            UShaderSource::Wgsl(snippet) => {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                is_3d.hash(&mut hasher);
                snippet.hash(&mut hasher);
                let hash = hasher.finish();

                if let Some(existing) = self.by_hash.get(&(is_3d, hash)) {
                    return Some(existing.clone());
                }

                let shader = if is_3d {
                    let full_wesl = inject_custom_shader_hook(UNODE_HOST_SHADER_TEMPLATE_3D, snippet);
                    Shader::from_wesl(full_wesl, format!("luma/ushader/inline3d_{hash:016x}.wesl"))
                } else {
                    let full_wgsl = inject_custom_shader_hook(UNODE_HOST_SHADER_TEMPLATE_2D, snippet);
                    Shader::from_wgsl(full_wgsl, format!("luma://ushader/inline_{hash:016x}.wgsl"))
                };

                let handle = shaders.add(shader);
                self.by_hash.insert((is_3d, hash), handle.clone());
                Some(handle)
            }
        }
    }
}

/// Cached material handles attached to a rendered node.
#[derive(Component, Default)]
pub struct MaterialHandles {
    /// Cached 2D material handle.
    pub material_2d: Option<Handle<UNodeMaterial>>,
    /// Cached 3D material handle.
    pub material_3d: Option<Handle<UNodeMaterial3d>>,
}

/// Simple statistics resource for material pooling.
#[derive(Resource, Default)]
pub struct MaterialPool {
    /// Number of material handles reused from the pool.
    pub reused_count: usize,
    /// Number of new material handles created this frame.
    pub created_count: usize,
}

impl MaterialPool {
    /// Resets the pool statistics for the current frame.
    pub fn reset_stats(&mut self) {
        self.reused_count = 0;
        self.created_count = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MeshCacheKey {
    width_bits: u32,
    height_bits: u32,
}

impl MeshCacheKey {
    fn from_size(size: Vec2) -> Self {
        Self {
            width_bits: size.x.to_bits(),
            height_bits: size.y.to_bits(),
        }
    }
}

/// Shared rectangle mesh cache keyed by the final logical size used for
/// rendering.
#[derive(Resource, Default)]
pub struct MeshPool {
    meshes_by_size: HashMap<MeshCacheKey, Handle<Mesh>>,
    /// Number of mesh handles reused from the pool.
    pub reused_count: usize,
    /// Number of new mesh handles created this frame.
    pub created_count: usize,
}

impl MeshPool {
    /// Retrieves an existing mesh matching the size from the cache, or creates a new one.
    pub fn mesh_for_size(&mut self, size: Vec2, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        let key = MeshCacheKey::from_size(size);

        if let Some(existing) = self.meshes_by_size.get(&key) {
            self.reused_count += 1;
            return existing.clone();
        }

        let mesh = meshes.add(Rectangle::new(size.x, size.y));
        self.meshes_by_size.insert(key, mesh.clone());
        self.created_count += 1;
        mesh
    }

    /// Resets the pool statistics for the current frame.
    pub fn reset_stats(&mut self) {
        self.reused_count = 0;
        self.created_count = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResolvedRenderMode {
    Flat2d,
    World3d,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ResolvedRenderContext {
    mode: ResolvedRenderMode,
    ui_to_world_scale: f32,
}

/// Internal system that refreshes node materials without leaking handles.
#[doc(hidden)]
pub fn update_materials_optimized(
    mut commands: Commands,
    rollout: Option<Res<UiRolloutConfig>>,
    mut cache: ResMut<LayoutCache>,
    mut pool: ResMut<MaterialPool>,
    mut mesh_pool: ResMut<MeshPool>,
    mut profiler: Option<ResMut<LayoutProfiler>>,
    mut nodes: Query<(
        Entity,
        &UNode,
        &ComputedSize,
        Option<&UBorder>,
        Option<&UImage>,
        Option<&UPbr>,
        Option<&CachedUiContext>,
        Option<&mut MaterialHandles>,
        Option<&UGradient>,
        Option<&UShadow>,
        Option<&UInnerGlow>,
        Option<&UShader>,
    )>,
    // Clipping queries
    parents_query: Query<&ChildOf>,
    clipper_query: Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: Query<&ResolvedRootUi>,

    // Render resources grouped to stay within Bevy's 16-parameter system limit
    render_assets: (
        Option<ResMut<Assets<Mesh>>>,
        Option<ResMut<Assets<UNodeMaterial>>>,
        Option<ResMut<Assets<UNodeMaterial3d>>>,
        Option<ResMut<UShaderCache>>,
        Option<ResMut<Assets<Shader>>>,
        Option<Res<AssetServer>>,
        Option<Res<Time>>,
    ),
) {
    let (
        meshes,
        materials_2d,
        materials_3d,
        mut shader_cache,
        mut shaders,
        asset_server,
        time,
    ) = render_assets;
    let (Some(mut meshes), Some(mut materials_2d), Some(mut materials_3d)) =
        (meshes, materials_2d, materials_3d)
    else {
        return;
    };

    let elapsed_secs = time.as_ref().map_or(0.0, |t| t.elapsed_secs());
    let start = std::time::Instant::now();
    let use_cached_ui_context = rollout
        .as_ref()
        .is_none_or(|config| config.use_cached_ui_context);
    let use_incremental_render = rollout
        .as_ref()
        .is_none_or(|config| config.use_incremental_render);
    let use_mesh_cache = rollout.as_ref().is_none_or(|config| config.use_mesh_cache);
    let use_instanced_batching = rollout
        .as_ref()
        .is_none_or(|config| config.use_instanced_batching);
    let created_before = pool.created_count;
    let reused_before = pool.reused_count;
    let mesh_created_before = mesh_pool.created_count;
    let mesh_reused_before = mesh_pool.reused_count;
    let render_frontier = if use_incremental_render {
        cache.take_render_frontier()
    } else {
        cache.all_entities_top_down()
    };

    for entity in render_frontier {
        let Ok((
            entity,
            node,
            size,
            border,
            image,
            pbr_opt,
            cached_context,
            handles_opt,
            gradient_opt,
            shadow_opt,
            inner_glow_opt,
            shader_opt,
        )) = nodes.get_mut(entity)
        else {
            continue;
        };

        sync_entity_material(
            entity,
            node,
            size,
            border,
            image,
            pbr_opt,
            cached_context,
            use_cached_ui_context,
            use_instanced_batching,
            handles_opt,
            gradient_opt,
            shadow_opt,
            inner_glow_opt,
            shader_opt,
            elapsed_secs,
            shader_cache.as_deref_mut(),
            shaders.as_deref_mut(),
            asset_server.as_deref(),
            &parents_query,
            &clipper_query,
            &root_query,
            &mut commands,
            &mut pool,
            &mut mesh_pool,
            use_mesh_cache,
            &mut meshes,
            &mut materials_2d,
            &mut materials_3d,
        );
        cache.complete_render(entity);
    }

    if let Some(ref mut prof) = profiler {
        prof.materials_created = pool.created_count - created_before;
        prof.materials_reused = pool.reused_count - reused_before;
        prof.meshes_created = mesh_pool.created_count - mesh_created_before;
        prof.meshes_reused = mesh_pool.reused_count - mesh_reused_before;
        prof.material_update_time = start.elapsed().as_secs_f64() * 1000.0;
    }
}

fn sync_entity_material(
    entity: Entity,
    node: &UNode,
    size: &ComputedSize,
    border: Option<&UBorder>,
    image: Option<&UImage>,
    pbr_opt: Option<&UPbr>,
    cached_context: Option<&CachedUiContext>,
    use_cached_ui_context: bool,
    use_instanced_batching: bool,
    handles_opt: Option<Mut<'_, MaterialHandles>>,
    gradient_opt: Option<&UGradient>,
    shadow_opt: Option<&UShadow>,
    inner_glow_opt: Option<&UInnerGlow>,
    shader_opt: Option<&UShader>,
    elapsed_secs: f32,
    shader_cache: Option<&mut UShaderCache>,
    shaders: Option<&mut Assets<Shader>>,
    asset_server: Option<&AssetServer>,
    parents_query: &Query<&ChildOf>,
    clipper_query: &Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    root_query: &Query<&ResolvedRootUi>,
    commands: &mut Commands,
    pool: &mut MaterialPool,
    mesh_pool: &mut MeshPool,
    use_mesh_cache: bool,
    meshes: &mut Assets<Mesh>,
    materials_2d: &mut Assets<UNodeMaterial>,
    materials_3d: &mut Assets<UNodeMaterial3d>,
) {
    let logical_size = Vec2::new(size.width, size.height);
    if logical_size.x <= 0.0 || logical_size.y <= 0.0 {
        if handles_opt.is_some() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_remove::<(
                    Mesh2d,
                    MeshMaterial2d<UNodeMaterial>,
                    Mesh3d,
                    MeshMaterial3d<UNodeMaterial3d>,
                    MaterialHandles,
                )>();
            }
        }
        return;
    }

    let cached_context = if use_cached_ui_context {
        cached_context
    } else {
        None
    };
    let render_context = resolve_render_context(entity, cached_context, parents_query, root_query);

    let world_scale = render_context.ui_to_world_scale;
    let clip_data = collect_clip_stack_for_entity(
        entity,
        Some(world_scale),
        parents_query,
        clipper_query,
        root_query,
    );

    // Fast-path: Pure 2D SDF nodes without textures, gradients, custom shaders, or complex clips
    // are rendered via the single-draw instanced batch pipeline.
    if use_instanced_batching
        && render_context.mode == ResolvedRenderMode::Flat2d
        && !clip_data.is_complex_for_batch
        && image.is_none()
        && gradient_opt.is_none()
        && inner_glow_opt.is_none()
        && shader_opt.is_none()
    {
        if handles_opt.is_some() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_remove::<(Mesh2d, MeshMaterial2d<UNodeMaterial>, MaterialHandles)>();
            }
        }
        return;
    }

    let size_vec = logical_size * world_scale;
    let is_world_3d = render_context.mode == ResolvedRenderMode::World3d;
    // In perspective 3D, let screen-space derivatives drive crisp pixel-exact AA instead of clamping
    // softness to world-scale fractions that blur zoomed-in panels and clips.
    let softness = if is_world_3d {
        1.0e-5
    } else {
        minimum_local_softness(world_scale)
    };

    // --- shared material inputs ---
    let (tex_handle, use_tex, base_color) = if let Some(img) = image {
        (Some(img.texture.clone()), 1, LinearRgba::from(img.color))
    } else {
        (None, 0, LinearRgba::from(node.background_color))
    };

    let (b_color, b_offset, b_width) = if let Some(b) = border {
        (
            LinearRgba::from(b.color),
            b.offset * world_scale,
            b.width * world_scale,
        )
    } else {
        (LinearRgba::NONE, 0.0, 0.0)
    };

    let effective_radius = if node.border_radius != UCornerRadius::default() {
        node.border_radius
    } else if let Some(b) = border && b.radius != UCornerRadius::default() {
        b.radius
    } else {
        node.border_radius
    };

    let radius = Vec4::new(
        effective_radius.top_right,
        effective_radius.bottom_right,
        effective_radius.top_left,
        effective_radius.bottom_left,
    ) * world_scale;

    let shape_mode = match node.shape_mode {
        UShapeMode::Round => 0,
        UShapeMode::Cut => 1,
    };

    // --- visual inputs ---
    let (grad_colors, grad_stops, grad_params, grad_center) = if let Some(grad) = gradient_opt {
        let (grad_type, param_y, center) = match grad.kind {
            UGradientKind::Linear { angle } => (1.0, angle, Vec2::ZERO),
            UGradientKind::Radial { center, radius } => (2.0, radius, center),
            UGradientKind::Angular { center } => (3.0, 0.0, center),
        };
        let count = grad.stops.len().min(8);
        let mut colors = [Vec4::ZERO; 8];
        let mut positions = [0.0f32; 8];
        for (i, stop) in grad.stops.iter().take(8).enumerate() {
            colors[i] = LinearRgba::from(stop.color).to_vec4();
            positions[i] = stop.position;
        }
        let stops = [
            Vec4::new(positions[0], positions[1], positions[2], positions[3]),
            Vec4::new(positions[4], positions[5], positions[6], positions[7]),
        ];
        let interp_mode = match grad.interpolation {
            UGradientInterpolation::Smooth => 0.0,
            UGradientInterpolation::Stepped => 1.0,
        };
        (
            colors,
            stops,
            Vec4::new(grad_type, param_y, count as f32, interp_mode),
            Vec4::new(center.x, center.y, grad.offset, 0.0),
        )
    } else {
        ([Vec4::ZERO; 8], [Vec4::ZERO; 2], Vec4::ZERO, Vec4::ZERO)
    };

    let (shadow_color, shadow_params) = if let Some(s) = shadow_opt {
        if !s.is_inner {
            (
                LinearRgba::from(s.color),
                Vec4::new(
                    s.offset.x * world_scale,
                    s.offset.y * world_scale,
                    s.blur * world_scale,
                    s.spread * world_scale,
                ),
            )
        } else {
            (LinearRgba::NONE, Vec4::ZERO)
        }
    } else {
        (LinearRgba::NONE, Vec4::ZERO)
    };

    let (inner_glow_color, inner_glow_params) = if let Some(ig) = inner_glow_opt {
        (
            LinearRgba::from(ig.color),
            Vec4::new(ig.blur * world_scale, 1.0, ig.spread * world_scale, 0.0),
        )
    } else if let Some(s) = shadow_opt
        && s.is_inner
    {
        (
            LinearRgba::from(s.color),
            Vec4::new(s.blur * world_scale, 1.0, s.spread * world_scale, 0.0),
        )
    } else {
        (LinearRgba::NONE, Vec4::ZERO)
    };

    let (custom_params, custom_meta, custom_shader) = if let Some(shader) = shader_opt {
        let t = if shader.animate {
            elapsed_secs * shader.speed + shader.time_offset
        } else {
            shader.time_offset
        };
        let meta = Vec4::new(t, shader.blend_mode.as_f32(), shader.opacity, 1.0);
        let handle = if let (Some(cache), Some(shader_assets)) = (shader_cache, shaders) {
            cache.resolve_for_mode(&shader.source, is_world_3d, shader_assets, asset_server)
        } else {
            None
        };
        (shader.params, meta, handle)
    } else {
        ([Vec4::ZERO; 4], Vec4::ZERO, None)
    };

    let shadow_blur = shadow_params.z;
    let shadow_spread = shadow_params.w;
    let shadow_offset = shadow_params.xy();
    let shadow_margin = if shadow_blur > 0.0 {
        shadow_blur * 2.5 + shadow_spread + shadow_offset.abs().max_element()
    } else {
        0.0
    };
    let mesh_size = size_vec + Vec2::splat(shadow_margin * 2.0);

    let mesh = if use_mesh_cache {
        mesh_pool.mesh_for_size(mesh_size, meshes)
    } else {
        meshes.add(Rectangle::new(mesh_size.x, mesh_size.y))
    };

    match render_context.mode {
        ResolvedRenderMode::World3d => {
            let (pbr_enabled, metallic, roughness, emissive_val) = if let Some(pbr) = pbr_opt {
                (1u32, pbr.metallic, pbr.roughness, pbr.emissive.to_vec4())
            } else {
                (0u32, 0.0, 0.5, Vec4::ZERO)
            };

            let build_mat_3d = || {
                create_3d_material(
                    base_color,
                    size_vec,
                    radius,
                    b_color,
                    emissive_val,
                    b_width,
                    b_offset,
                    softness,
                    metallic,
                    roughness,
                    use_tex,
                    shape_mode,
                    pbr_enabled,
                    tex_handle.clone(),
                    clip_data,
                    grad_colors,
                    grad_stops,
                    grad_params,
                    grad_center,
                    shadow_color.to_vec4(),
                    shadow_params,
                    inner_glow_color.to_vec4(),
                    inner_glow_params,
                    custom_params,
                    custom_meta,
                    custom_shader.clone(),
                )
            };

            let material_handle = if let Some(mut handles) = handles_opt {
                if let Some(existing_handle) = &handles.material_3d {
                    if let Some(mut existing_mat) = materials_3d.get_mut(existing_handle) {
                        existing_mat.color = base_color.to_vec4();
                        existing_mat.size = size_vec;
                        existing_mat.radius = radius;
                        existing_mat.border_color = b_color.to_vec4();
                        existing_mat.emissive = emissive_val;
                        existing_mat.border_width = b_width;
                        existing_mat.border_offset = b_offset;
                        existing_mat.softness = softness;
                        existing_mat.metallic = metallic;
                        existing_mat.roughness = roughness;
                        existing_mat.use_texture = use_tex;
                        existing_mat.shape_mode = shape_mode;
                        existing_mat.pbr_enabled = pbr_enabled;
                        existing_mat.texture = tex_handle.clone();
                        existing_mat.grad_colors = grad_colors;
                        existing_mat.grad_stops = grad_stops;
                        existing_mat.grad_params = grad_params;
                        existing_mat.grad_center = grad_center;
                        existing_mat.shadow_color = shadow_color.to_vec4();
                        existing_mat.shadow_params = shadow_params;
                        existing_mat.inner_glow_color = inner_glow_color.to_vec4();
                        existing_mat.inner_glow_params = inner_glow_params;
                        existing_mat.custom_params = custom_params;
                        existing_mat.custom_meta = custom_meta;
                        existing_mat.custom_shader = custom_shader.clone();
                        existing_mat.apply_clip_uniforms(&clip_data);

                        pool.reused_count += 1;
                        existing_handle.clone()
                    } else {
                        let new_mat = materials_3d.add(build_mat_3d());
                        handles.material_3d = Some(new_mat.clone());
                        pool.created_count += 1;
                        new_mat
                    }
                } else {
                    let new_mat = materials_3d.add(build_mat_3d());
                    handles.material_3d = Some(new_mat.clone());
                    pool.created_count += 1;
                    new_mat
                }
            } else {
                let new_mat = materials_3d.add(build_mat_3d());
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.try_insert(MaterialHandles {
                        material_2d: None,
                        material_3d: Some(new_mat.clone()),
                    });
                }
                pool.created_count += 1;
                new_mat
            };

            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert((Mesh3d(mesh), MeshMaterial3d(material_handle)))
                    .try_remove::<(Mesh2d, MeshMaterial2d<UNodeMaterial>)>();
            }
        }
        ResolvedRenderMode::Flat2d => {
            let material_handle = if let Some(mut handles) = handles_opt {
                if let Some(existing_handle) = &handles.material_2d {
                    if let Some(mut existing_mat) = materials_2d.get_mut(existing_handle) {
                        existing_mat.color = base_color;
                        existing_mat.radius = radius;
                        existing_mat.border_color = b_color;
                        existing_mat.size = size_vec;
                        existing_mat.border_width = b_width;
                        existing_mat.border_offset = b_offset;
                        existing_mat.softness = softness;
                        existing_mat.use_texture = use_tex;
                        existing_mat.shape_mode = shape_mode;
                        existing_mat.texture = tex_handle.clone();
                        existing_mat.apply_clip_uniforms(&clip_data);
                        existing_mat.grad_colors = grad_colors;
                        existing_mat.grad_stops = grad_stops;
                        existing_mat.grad_params = grad_params;
                        existing_mat.grad_center = grad_center;
                        existing_mat.shadow_color = shadow_color;
                        existing_mat.shadow_params = shadow_params;
                        existing_mat.inner_glow_color = inner_glow_color;
                        existing_mat.inner_glow_params = inner_glow_params;
                        existing_mat.custom_params = custom_params;
                        existing_mat.custom_meta = custom_meta;
                        existing_mat.custom_shader = custom_shader.clone();

                        pool.reused_count += 1;
                        existing_handle.clone()
                    } else {
                        let new_mat = materials_2d.add(create_2d_material(
                            base_color,
                            radius,
                            b_color,
                            size_vec,
                            b_width,
                            b_offset,
                            softness,
                            use_tex,
                            shape_mode,
                            tex_handle.clone(),
                            clip_data,
                            grad_colors,
                            grad_stops,
                            grad_params,
                            grad_center,
                            shadow_color,
                            shadow_params,
                            inner_glow_color,
                            inner_glow_params,
                            custom_params,
                            custom_meta,
                            custom_shader.clone(),
                        ));
                        handles.material_2d = Some(new_mat.clone());
                        pool.created_count += 1;
                        new_mat
                    }
                } else {
                    let new_mat = materials_2d.add(create_2d_material(
                        base_color,
                        radius,
                        b_color,
                        size_vec,
                        b_width,
                        b_offset,
                        softness,
                        use_tex,
                        shape_mode,
                        tex_handle.clone(),
                        clip_data,
                        grad_colors,
                        grad_stops,
                        grad_params,
                        grad_center,
                        shadow_color,
                        shadow_params,
                        inner_glow_color,
                        inner_glow_params,
                        custom_params,
                        custom_meta,
                        custom_shader.clone(),
                    ));
                    handles.material_2d = Some(new_mat.clone());
                    pool.created_count += 1;
                    new_mat
                }
            } else {
                let new_mat = materials_2d.add(create_2d_material(
                    base_color,
                    radius,
                    b_color,
                    size_vec,
                    b_width,
                    b_offset,
                    softness,
                    use_tex,
                    shape_mode,
                    tex_handle.clone(),
                    clip_data,
                    grad_colors,
                    grad_stops,
                    grad_params,
                    grad_center,
                    shadow_color,
                    shadow_params,
                    inner_glow_color,
                    inner_glow_params,
                    custom_params,
                    custom_meta,
                    custom_shader,
                ));
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.try_insert(MaterialHandles {
                        material_2d: Some(new_mat.clone()),
                        material_3d: None,
                    });
                }
                pool.created_count += 1;
                new_mat
            };

            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.try_insert((Mesh2d(mesh), MeshMaterial2d(material_handle)))
                    .try_remove::<(Mesh3d, MeshMaterial3d<UNodeMaterial3d>)>();
            }
        }
    }
}

fn resolve_render_context(
    entity: Entity,
    cached_context: Option<&CachedUiContext>,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> ResolvedRenderContext {
    if let Some(context) = cached_context
        && context.root_entity.is_some()
    {
        let mode = match context.space {
            UiSpace::World3d => ResolvedRenderMode::World3d,
            UiSpace::Screen | UiSpace::World2d => ResolvedRenderMode::Flat2d,
        };

        return ResolvedRenderContext {
            mode,
            ui_to_world_scale: context.ui_to_world_scale,
        };
    }

    let Some(root) = resolve_root_for_entity(entity, parents_query, root_query) else {
        return ResolvedRenderContext {
            mode: ResolvedRenderMode::Flat2d,
            ui_to_world_scale: 1.0,
        };
    };

    let mode = match root.space {
        UiSpace::World3d => ResolvedRenderMode::World3d,
        UiSpace::Screen | UiSpace::World2d => ResolvedRenderMode::Flat2d,
    };

    ResolvedRenderContext {
        mode,
        ui_to_world_scale: root.ui_units_to_world_scale(),
    }
}

fn resolve_root_for_entity(
    entity: Entity,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> Option<ResolvedRootUi> {
    let mut current = entity;

    loop {
        if let Ok(root) = root_query.get(current) {
            return Some(*root);
        }

        let parent = parents_query.get(current).ok()?;
        current = parent.get();
    }
}

// ===== Helper Functions =====
fn minimum_local_softness(ui_to_world_scale: f32) -> f32 {
    (0.5 * ui_to_world_scale).max(1.0e-4)
}

// 1. 2D material builder with multi-layer clip data
fn create_2d_material(
    base_color: LinearRgba,
    radius: Vec4,
    b_color: LinearRgba,
    size_vec: Vec2,
    b_width: f32,
    b_offset: f32,
    softness: f32,
    use_tex: u32,
    shape_mode: u32,
    tex: Option<Handle<Image>>,
    clip_data: ClipUniformData,
    grad_colors: [Vec4; 8],
    grad_stops: [Vec4; 2],
    grad_params: Vec4,
    grad_center: Vec4,
    shadow_color: LinearRgba,
    shadow_params: Vec4,
    inner_glow_color: LinearRgba,
    inner_glow_params: Vec4,
    custom_params: [Vec4; 4],
    custom_meta: Vec4,
    custom_shader: Option<Handle<Shader>>,
) -> UNodeMaterial {
    let mut material = UNodeMaterial {
        color: base_color,
        radius,
        border_color: b_color,
        size: size_vec,
        border_width: b_width,
        border_offset: b_offset,
        softness,
        use_texture: use_tex,
        shape_mode,
        texture: tex,
        grad_colors,
        grad_stops,
        grad_params,
        grad_center,
        shadow_color,
        shadow_params,
        inner_glow_color,
        inner_glow_params,
        custom_params,
        custom_meta,
        custom_cursor: Vec4::new(0.5, 0.5, 0.0, 0.0),
        custom_shader,
        ..default()
    };
    material.apply_clip_uniforms(&clip_data);
    material
}

// 2. 3D material builder with multi-layer clip data
fn create_3d_material(
    base_color: LinearRgba,
    size_vec: Vec2,
    radius: Vec4,
    b_color: LinearRgba,
    emissive: Vec4,
    b_width: f32,
    b_offset: f32,
    softness: f32,
    metallic: f32,
    roughness: f32,
    use_tex: u32,
    shape_mode: u32,
    pbr_enabled: u32,
    tex: Option<Handle<Image>>,
    clip_data: ClipUniformData,
    grad_colors: [Vec4; 8],
    grad_stops: [Vec4; 2],
    grad_params: Vec4,
    grad_center: Vec4,
    shadow_color: Vec4,
    shadow_params: Vec4,
    inner_glow_color: Vec4,
    inner_glow_params: Vec4,
    custom_params: [Vec4; 4],
    custom_meta: Vec4,
    custom_shader: Option<Handle<Shader>>,
) -> UNodeMaterial3d {
    let mut material = UNodeMaterial3d {
        color: base_color.to_vec4(),
        size: size_vec,
        radius,
        border_color: b_color.to_vec4(),
        emissive,
        border_width: b_width,
        border_offset: b_offset,
        softness,
        metallic,
        roughness,
        use_texture: use_tex,
        shape_mode,
        pbr_enabled,
        texture: tex,
        grad_colors,
        grad_stops,
        grad_params,
        grad_center,
        shadow_color,
        shadow_params,
        inner_glow_color,
        inner_glow_params,
        custom_params,
        custom_meta,
        custom_shader,
        ..default()
    };
    material.apply_clip_uniforms(&clip_data);
    material
}

fn apply_shader_cursor_state(
    custom_meta: &mut Vec4,
    custom_cursor: &mut Vec4,
    animate: bool,
    time_value: f32,
    target_uv: Vec2,
    target_hover: f32,
    target_press: f32,
    smooth_rate: f32,
) {
    if animate {
        custom_meta.x = time_value;
    }
    let prev = *custom_cursor;
    let next_uv = prev.xy().lerp(target_uv, smooth_rate);
    let next_hover = prev.z + (target_hover - prev.z) * smooth_rate;
    let next_press = prev.w + (target_press - prev.w) * smooth_rate;
    *custom_cursor = Vec4::new(next_uv.x, next_uv.y, next_hover, next_press);
}

/// Lightweight frame-by-frame time and mouse cursor updater for [`UShader`] nodes.
/// Updates `custom_meta.x` and `custom_cursor` directly in `Assets<UNodeMaterial>` / `Assets<UNodeMaterial3d>`
/// across 2D, `World2d`, `World3d`, and multiple windows without dirtying UI layout.
pub fn sync_animated_ushaders(
    time: Res<Time>,
    windows: Query<(Entity, &Window, Option<&PrimaryWindow>)>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, Option<&RenderTarget>)>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    query: Query<(
        Entity,
        &UShader,
        &MaterialHandles,
        Option<&GlobalTransform>,
        Option<&ComputedSize>,
        Option<&UiPickingContext>,
    )>,
    mut materials_2d: Option<ResMut<Assets<UNodeMaterial>>>,
    mut materials_3d: Option<ResMut<Assets<UNodeMaterial3d>>>,
) {
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs().clamp(0.001, 0.1);
    let is_mouse_pressed = mouse_buttons
        .as_ref()
        .is_some_and(|btn| btn.pressed(MouseButton::Left));

    let active_windows = collect_active_cursor_windows(&windows);
    let smooth_rate = (dt * 12.0).clamp(0.0, 1.0);

    for (entity, shader, handles, global_tf, computed_size, picking_ctx) in query.iter() {
        let t = if shader.animate {
            elapsed * shader.speed + shader.time_offset
        } else {
            shader.time_offset
        };

        // Compute normalized local cursor UV (0.0..1.0) and hover/press state
        let mut target_uv = Vec2::new(
            0.5 + 0.26 * (t * 0.85).cos(),
            0.5 + 0.22 * (t * 1.15).sin(),
        );
        let mut is_hovered = false;

        if !active_windows.is_empty()
            && let (Some(tf), Some(size)) = (global_tf, computed_size)
            && size.width > 1.0
            && size.height > 1.0
        {
            let world_scale =
                resolve_node_world_scale(entity, picking_ctx, &parents_query, &root_query);
            let preferred_camera = picking_ctx.and_then(|ctx| ctx.camera_entity);

            if let Some((_, _, _, local_logical, _)) =
                active_windows
                    .iter()
                    .find_map(|&(win_entity, cursor_pos, primary_window)| {
                        project_screen_point_to_node(
                            cursor_pos,
                            None,
                            Some(win_entity),
                            primary_window,
                            preferred_camera,
                            tf,
                            world_scale,
                            true,
                            &cameras,
                        )
                    })
            {
                let uv_x = local_logical.x / size.width + 0.5;
                let uv_y = 0.5 - local_logical.y / size.height;
                if (-0.15..=1.15).contains(&uv_x) && (-0.15..=1.15).contains(&uv_y) {
                    target_uv = Vec2::new(uv_x.clamp(0.0, 1.0), uv_y.clamp(0.0, 1.0));
                    is_hovered = (0.0..=1.0).contains(&uv_x) && (0.0..=1.0).contains(&uv_y);
                }
            }
        }

        let target_hover = if is_hovered { 1.0 } else { 0.0 };
        let target_press = if is_hovered && is_mouse_pressed {
            1.0
        } else {
            0.0
        };

        if let (Some(h2d), Some(mats2d)) = (&handles.material_2d, materials_2d.as_mut())
            && let Some(mut mat) = mats2d.get_mut(h2d)
        {
            let mat = &mut *mat;
            apply_shader_cursor_state(
                &mut mat.custom_meta,
                &mut mat.custom_cursor,
                shader.animate,
                t,
                target_uv,
                target_hover,
                target_press,
                smooth_rate,
            );
        }
        if let (Some(h3d), Some(mats3d)) = (&handles.material_3d, materials_3d.as_mut())
            && let Some(mut mat) = mats3d.get_mut(h3d)
        {
            let mat = &mut *mat;
            apply_shader_cursor_state(
                &mut mat.custom_meta,
                &mut mat.custom_cursor,
                shader.animate,
                t,
                target_uv,
                target_hover,
                target_press,
                smooth_rate,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo};
    use bevy::time::TimeUpdateStrategy;
    use bevy::window::WindowRef;
    use std::time::Duration;

    #[test]
    fn sync_animated_ushaders_projects_secondary_window_world3d_with_world_scale() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin, bevy::asset::AssetPlugin::default()));
        app.init_asset::<UNodeMaterial>();
        app.init_asset::<UNodeMaterial3d>();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(50)));
        app.add_systems(Update, sync_animated_ushaders);

        app.world_mut()
            .spawn((Window::default(), PrimaryWindow));
        let mut secondary_window = Window::default();
        secondary_window.resolution.set_physical_resolution(800, 600);

        let mut camera = Camera::default();
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor: 1.0,
            }),
            clip_from_view: PerspectiveProjection {
                fov: core::f32::consts::FRAC_PI_2,
                aspect_ratio: 800.0 / 600.0,
                near: 0.1,
                ..default()
            }
            .get_clip_from_view(),
            ..default()
        };
        let cam_transform = Transform::from_xyz(0.0, 0.0, 5.0);
        let cam_gt = GlobalTransform::from(cam_transform);

        let world_scale = 0.01;
        // Pick a point at (+25, -20) logical px relative to a 100x80 node => expected UV = (0.75, 0.75).
        let target_world = Vec3::new(25.0 * world_scale, -20.0 * world_scale, 0.0);
        let viewport_pos = camera.world_to_viewport(&cam_gt, target_world).unwrap();
        secondary_window.set_cursor_position(Some(viewport_pos));

        let sec_win_entity = app.world_mut().spawn(secondary_window).id();
        let sec_camera = app
            .world_mut()
            .spawn((
                camera,
                RenderTarget::Window(WindowRef::Entity(sec_win_entity)),
                cam_transform,
                cam_gt,
            ))
            .id();

        let mat3d_handle = app
            .world_mut()
            .resource_mut::<Assets<UNodeMaterial3d>>()
            .add(create_3d_material(
                LinearRgba::WHITE,
                Vec2::new(100.0 * world_scale, 80.0 * world_scale),
                Vec4::ZERO,
                LinearRgba::NONE,
                Vec4::ZERO,
                0.0,
                0.0,
                0.01,
                0.0,
                0.5,
                0,
                0,
                0,
                None,
                ClipUniformData::default(),
                [Vec4::ZERO; 8],
                [Vec4::ZERO; 2],
                Vec4::ZERO,
                Vec4::ZERO,
                Vec4::ZERO,
                Vec4::ZERO,
                Vec4::ZERO,
                Vec4::ZERO,
                [Vec4::ZERO; 4],
                Vec4::ZERO,
                None,
            ));

        let shader_node = app
            .world_mut()
            .spawn((
                UShader::default(),
                MaterialHandles {
                    material_2d: None,
                    material_3d: Some(mat3d_handle.clone()),
                },
                Transform::default(),
                GlobalTransform::default(),
                ComputedSize {
                    width: 100.0,
                    height: 80.0,
                    ..default()
                },
                UiPickingContext {
                    root_entity: None,
                    camera_entity: Some(sec_camera),
                    space: UiSpace::World3d,
                    clip_ancestor: None,
                    root_sort_key: 0.0,
                    local_depth_key: 0.0,
                    world_scale,
                },
            ))
            .id();

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        for _ in 0..8 {
            app.update();
        }

        let mat = app
            .world()
            .resource::<Assets<UNodeMaterial3d>>()
            .get(&mat3d_handle)
            .unwrap();
        assert!(app.world().get_entity(shader_node).is_ok());
        assert!(
            (mat.custom_cursor.x - 0.75).abs() < 0.02,
            "expected UV.x near 0.75, got {}",
            mat.custom_cursor.x
        );
        assert!(
            (mat.custom_cursor.y - 0.75).abs() < 0.02,
            "expected UV.y near 0.75, got {}",
            mat.custom_cursor.y
        );
        assert!(
            mat.custom_cursor.z > 0.85,
            "expected hover near 1.0, got {}",
            mat.custom_cursor.z
        );
        assert!(
            mat.custom_cursor.w > 0.85,
            "expected press near 1.0, got {}",
            mat.custom_cursor.w
        );
    }

    #[test]
    fn world3d_material_parity_gates_pbr_border_offset_softness_and_custom_shader() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            crate::LumaEnginePlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .init_asset::<Shader>();

        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn((Camera::default(), Camera3d::default()));

        let mut root_ui = crate::layout::layout_system::URootUi::world_3d(Vec2::new(400.0, 200.0));
        root_ui.meters_per_unit = 0.01;
        let world_scale = 0.01_f32;

        let root = app.world_mut().spawn(root_ui).id();
        let unlit_shader_node = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    width: UVal::Px(120.0),
                    height: UVal::Px(60.0),
                    background_color: Color::srgba(0.2, 0.4, 0.9, 0.6),
                    ..default()
                },
                crate::layout::univis_node::UBorder {
                    color: Color::WHITE,
                    width: 2.0,
                    offset: 3.0,
                    ..default()
                },
                crate::layout::visual::UShader::from_wgsl(
                    "fn fragment_color(in: LumaShaderInput) -> vec4<f32> { return vec4<f32>(in.uv, 0.5, 1.0); }",
                ),
            ))
            .id();

        let lit_pbr_node = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    width: UVal::Px(80.0),
                    height: UVal::Px(40.0),
                    background_color: Color::srgba(0.9, 0.3, 0.2, 1.0),
                    ..default()
                },
                crate::layout::pbr::UPbr {
                    metallic: 0.8,
                    roughness: 0.2,
                    emissive: LinearRgba::rgb(0.2, 0.4, 0.6),
                },
            ))
            .id();

        for _ in 0..4 {
            app.update();
        }

        let materials = app.world().resource::<Assets<UNodeMaterial3d>>();

        let unlit_mat_handle = app
            .world()
            .entity(unlit_shader_node)
            .get::<MeshMaterial3d<UNodeMaterial3d>>()
            .expect("World3d node should use UNodeMaterial3d")
            .0
            .clone();
        let unlit_mat = materials
            .get(&unlit_mat_handle)
            .expect("unlit UNodeMaterial3d asset should exist");

        assert_eq!(unlit_mat.pbr_enabled, 0);
        assert!(
            (unlit_mat.border_offset - 3.0 * world_scale).abs() < 1e-5,
            "expected scaled border_offset={}, got {}",
            3.0 * world_scale,
            unlit_mat.border_offset
        );
        assert!(
            (unlit_mat.softness - 1.0e-5).abs() < 1e-7,
            "expected World3d softness=1e-5, got {}",
            unlit_mat.softness
        );
        assert!(
            unlit_mat.custom_shader.is_some(),
            "World3d UNode with UShader must specialize custom_shader"
        );
        assert_eq!(
            Material::alpha_mode(unlit_mat),
            AlphaMode::Premultiplied,
            "UNodeMaterial3d must use AlphaMode::Premultiplied to avoid alpha^2 fringes"
        );

        let lit_mat_handle = app
            .world()
            .entity(lit_pbr_node)
            .get::<MeshMaterial3d<UNodeMaterial3d>>()
            .expect("lit World3d node should use UNodeMaterial3d")
            .0
            .clone();
        let lit_mat = materials
            .get(&lit_mat_handle)
            .expect("lit UNodeMaterial3d asset should exist");

        assert_eq!(lit_mat.pbr_enabled, 1);
        assert!((lit_mat.metallic - 0.8).abs() < 1e-5);
        assert!((lit_mat.roughness - 0.2).abs() < 1e-5);
        assert!(lit_mat.custom_shader.is_none());
    }
}
