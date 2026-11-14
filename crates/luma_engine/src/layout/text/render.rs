pub(super) mod atlas;
pub(super) mod clip_sync;
pub(crate) mod mesh;
mod sdf;

use crate::layout::query::{CachedUiContext, UI3d};
use crate::layout::query::{ComputedSize, ResolvedRootStack, ResolvedRootUi};
use crate::layout::univis_node::{UClip, UNode};
use bevy::asset::{Asset, AssetId, Assets, RenderAssetUsages};
use bevy::color::LinearRgba;
use bevy::ecs::relationship::Relationship;
use bevy::image::{DynamicTextureAtlasBuilder, ImageSampler, TextureAtlasLayout};
use bevy::mesh::{Indices, Mesh, Mesh2d, Mesh3d, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, MeshMaterial2d};
use bevy::text::{ComputedTextBlock, FontCx, ScaleCx};
use std::collections::HashMap;

use self::atlas::{
    UTextLabelAtlasCache, UTextLabelAtlasKey, UTextLabelGlyphAtlasInfo, UTextLabelGlyphKey,
    create_text_atlas_page, extract_mask_alpha,
};
use self::clip_sync::{clip_quad_to_rect, label_content_clip_rect};
use self::mesh::{
    TextGlyphQuad, TextPageBatch, build_solid_highlight_material, build_solid_highlight_material_3d,
    build_text_material, build_text_material_3d, build_text_page_mesh,
    resolved_render_scale_for_font, text_horizontal_offset,
};
use self::sdf::build_sdf_glyph_image;
use super::model::{
    TextChildMarker, TextSelectionHighlightMarker, USelectableText, UTextLabel,
    UTextLabelLayoutCache, UTextOverflow,
};
use super::selection::compute_text_selection_quads;

const DEFAULT_TEXT_EDGE_SOFTNESS: f32 = 0.45;
const TEXT_SDF_PADDING: u32 = 12;
const TEXT_SDF_ATLAS_SIZE: u32 = 1024;
const DISTANCE_FIELD_INF: f32 = 1.0e20;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct TextRenderPageMarker {
    texture_id: AssetId<Image>,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(super) struct UTextLabelSdfMaterial {
    #[uniform(0)]
    pub(crate) color: LinearRgba,
    #[uniform(0)]
    pub(crate) outline_color: LinearRgba,
    #[uniform(0)]
    pub(crate) edge_softness: f32,
    #[uniform(0)]
    pub(crate) outline_width: f32,
    #[uniform(0)]
    pub(crate) _pad0: f32,
    #[uniform(0)]
    pub(crate) _pad1: f32,
    #[uniform(0)]
    pub(crate) clip_inv_x: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_inv_y: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_bounds: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_radius: [Vec4; 4],
    #[texture(1)]
    #[sampler(2)]
    pub(crate) texture: Handle<Image>,
}

impl Material2d for UTextLabelSdfMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://luma_engine/layout/render/shaders/text_label_sdf.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn depth_bias(&self) -> f32 {
        0.0
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct UTextLabelSdfMaterial3d {
    #[uniform(0)]
    pub(crate) color: LinearRgba,
    #[uniform(0)]
    pub(crate) outline_color: LinearRgba,
    #[uniform(0)]
    pub(crate) edge_softness: f32,
    #[uniform(0)]
    pub(crate) outline_width: f32,
    #[uniform(0)]
    pub(crate) _pad0: f32,
    #[uniform(0)]
    pub(crate) _pad1: f32,
    #[uniform(0)]
    pub(crate) clip_inv_x: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_inv_y: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_bounds: [Vec4; 4],
    #[uniform(0)]
    pub(crate) clip_radius: [Vec4; 4],
    #[texture(1)]
    #[sampler(2)]
    pub(crate) texture: Handle<Image>,
}

impl Material for UTextLabelSdfMaterial3d {
    fn fragment_shader() -> ShaderRef {
        "embedded://luma_engine/layout/render/shaders/text_label_sdf_3d.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

fn resolve_text_mesh_world_scale(
    entity: Entity,
    is_3d: bool,
    cached_context: Option<&CachedUiContext>,
    parents_query: &Query<&ChildOf>,
    root_query: &Query<&ResolvedRootUi>,
) -> f32 {
    if !is_3d {
        return 1.0;
    }

    if let Some(ctx) = cached_context
        && matches!(ctx.space, crate::layout::layout_system::UiSpace::World3d)
    {
        return ctx.ui_to_world_scale;
    }

    let mut current = entity;
    while let Ok(child_of) = parents_query.get(current) {
        current = child_of.parent();
        if let Ok(root_ui) = root_query.get(current) {
            return root_ui.ui_units_to_world_scale();
        }
    }

    0.01
}

pub(super) fn sync_text_label_meshes(
    mut commands: Commands,
    mut atlas_cache: ResMut<UTextLabelAtlasCache>,
    images: Option<ResMut<Assets<Image>>>,
    texture_atlases: Option<ResMut<Assets<TextureAtlasLayout>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<UTextLabelSdfMaterial>>>,
    materials_3d: Option<ResMut<Assets<UTextLabelSdfMaterial3d>>>,
    mut _font_cx: ResMut<FontCx>,
    mut scale_cx: ResMut<ScaleCx>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    label_query: Query<
        (
            Entity,
            Ref<UTextLabel>,
            Ref<UNode>,
            Ref<ComputedSize>,
            Ref<ComputedTextBlock>,
            Ref<UTextLabelLayoutCache>,
            Option<&Children>,
            Has<UI3d>,
            Option<&CachedUiContext>,
        ),
        Or<(
            Added<UTextLabel>,
            Changed<UTextLabel>,
            Changed<UNode>,
            Changed<ComputedSize>,
            Changed<ComputedTextBlock>,
            Changed<UTextLabelLayoutCache>,
            Added<UI3d>,
            Changed<CachedUiContext>,
        )>,
    >,
    child_query: Query<
        (
            Entity,
            &ChildOf,
            &TextRenderPageMarker,
            Option<&Mesh2d>,
            Option<&Mesh3d>,
            Option<&MeshMaterial2d<UTextLabelSdfMaterial>>,
            Option<&MeshMaterial3d<UTextLabelSdfMaterial3d>>,
        ),
        With<TextChildMarker>,
    >,
) {
    let (
        Some(mut images),
        Some(mut texture_atlases),
        Some(mut meshes),
        Some(mut materials),
        Some(mut materials_3d),
    ) = (images, texture_atlases, meshes, materials, materials_3d)
    else {
        return;
    };

    for (
        entity,
        label,
        node,
        computed_size,
        computed,
        layout_cache,
        children,
        is_3d,
        cached_context,
    ) in label_query.iter()
    {
        let mut existing_children = HashMap::new();
        if let Some(children) = children {
            for child in children {
                if let Ok((child_entity, child_of, page_marker, mesh_2d, mesh_3d, mat_2d, mat_3d)) =
                    child_query.get(*child)
                {
                    if child_of.get() == entity {
                        let child_is_3d = mesh_3d.is_some() || mat_3d.is_some();
                        if child_is_3d == is_3d {
                            let mesh_handle = if child_is_3d {
                                mesh_3d.map(|m| m.0.clone())
                            } else {
                                mesh_2d.map(|m| m.0.clone())
                            };
                            let mat_handle_2d = mat_2d.map(|m| m.0.clone());
                            let mat_handle_3d = mat_3d.map(|m| m.0.clone());

                            if let Some(m) = mesh_handle {
                                existing_children.insert(
                                    page_marker.texture_id,
                                    (child_entity, m, mat_handle_2d, mat_handle_3d),
                                );
                            }
                        }
                    }
                }
            }
        }

        let text_size = layout_cache.measured_size;
        let content_clip = label_content_clip_rect(&label, &node, &computed_size);
        let horizontal_offset = text_horizontal_offset(&label, &node, &computed_size, text_size);
        let parley_origin_x = super::selection::parley_layout_origin_x(&computed);

        let ui_to_world_scale = resolve_text_mesh_world_scale(
            entity,
            is_3d,
            cached_context,
            &parents_query,
            &root_query,
        );

        let mut page_batches: HashMap<AssetId<Image>, TextPageBatch> = HashMap::default();

        if !layout_cache.displayed_text.is_empty() && text_size.x > 0.0 && text_size.y > 0.0 {
            use bevy::platform::hash::FixedHasher;
            use parley::PositionedLayoutItem;
            use std::hash::BuildHasher;

            for line in computed.buffer().lines() {
                for item in line.items() {
                    if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        let section_index = glyph_run.style().brush.section_index as usize;
                        let glyph_color: LinearRgba = if label.spans.is_empty() {
                            LinearRgba::WHITE
                        } else {
                            label
                                .spans
                                .get(section_index)
                                .and_then(|s| s.color)
                                .unwrap_or(label.color)
                                .into()
                        };
                        let run = glyph_run.run();
                        let font = run.font();
                        let font_size = run.font_size();
                        let coords = run.normalized_coords();
                        let variations_hash = FixedHasher.hash_one(coords);

                        let glyph_render_scale =
                            resolved_render_scale_for_font(label.render_scale, font_size);
                        let font_size_physical = font_size * glyph_render_scale;

                        let Some(font_ref) =
                            swash::FontRef::from_index(font.data.as_ref(), font.index as usize)
                        else {
                            continue;
                        };

                        let mut scaler = scale_cx
                            .0
                            .builder(font_ref)
                            .size(font_size_physical)
                            .hint(false)
                            .normalized_coords(coords)
                            .build();

                        let atlas_key = UTextLabelAtlasKey {
                            font_data_id: font.data.id(),
                            font_size_bits: font_size_physical.to_bits(),
                            index: font.index,
                            variations_hash,
                        };

                        for glyph in glyph_run.positioned_glyphs() {
                            let Ok(glyph_id) = u16::try_from(glyph.id) else {
                                continue;
                            };

                            let glyph_key = UTextLabelGlyphKey {
                                glyph_id,
                                x_bin_bits: 0,
                                y_bin_bits: 0,
                            };

                            let atlas_pages =
                                atlas_cache.pages_by_font.entry(atlas_key).or_default();
                            let atlas_info = if let Some(info) = atlas_pages
                                .iter()
                                .find_map(|page| page.glyphs.get(&glyph_key))
                                .cloned()
                            {
                                info
                            } else {
                                let image = swash::scale::Render::new(&[
                                    swash::scale::Source::ColorOutline(0),
                                    swash::scale::Source::ColorBitmap(
                                        swash::scale::StrikeWith::BestFit,
                                    ),
                                    swash::scale::Source::Outline,
                                ])
                                .format(swash::zeno::Format::Alpha)
                                .render(&mut scaler, glyph_id);

                                let Some(image) = image else {
                                    continue;
                                };

                                let width = image.placement.width;
                                let height = image.placement.height;
                                if width == 0 || height == 0 {
                                    continue;
                                }

                                let mask = extract_mask_alpha(
                                    &image.data,
                                    width as usize,
                                    height as usize,
                                );
                                if mask.iter().all(|alpha| *alpha == 0) {
                                    continue;
                                }

                                let glyph_image = build_sdf_glyph_image(&mask, width, height);
                                let glyph_offset = IVec2::new(
                                    image.placement.left - TEXT_SDF_PADDING as i32,
                                    image.placement.top + TEXT_SDF_PADDING as i32,
                                );

                                let mut stored = None;
                                for page in atlas_pages.iter_mut() {
                                    let Some(mut atlas_layout) =
                                        texture_atlases.get_mut(&page.texture_atlas)
                                    else {
                                        continue;
                                    };
                                    let Some(mut atlas_image) = images.get_mut(&page.texture)
                                    else {
                                        continue;
                                    };
                                    if let Ok(glyph_index) = page.builder.add_texture(
                                        &mut *atlas_layout,
                                        &glyph_image,
                                        &mut *atlas_image,
                                    ) {
                                        let info = UTextLabelGlyphAtlasInfo {
                                            texture: page.texture.clone(),
                                            texture_atlas: page.texture_atlas.clone(),
                                            glyph_index,
                                            offset: glyph_offset,
                                        };
                                        page.glyphs.insert(glyph_key, info.clone());
                                        stored = Some(info);
                                        break;
                                    }
                                }

                                if let Some(info) = stored {
                                    info
                                } else {
                                    let min_size = glyph_image
                                        .texture_descriptor
                                        .size
                                        .width
                                        .max(glyph_image.texture_descriptor.size.height);
                                    let mut page = create_text_atlas_page(
                                        &mut images,
                                        &mut texture_atlases,
                                        min_size,
                                    );
                                    let Some(mut atlas_layout) =
                                        texture_atlases.get_mut(&page.texture_atlas)
                                    else {
                                        continue;
                                    };
                                    let Some(mut atlas_image) = images.get_mut(&page.texture)
                                    else {
                                        continue;
                                    };
                                    let Ok(glyph_index) = page.builder.add_texture(
                                        &mut *atlas_layout,
                                        &glyph_image,
                                        &mut *atlas_image,
                                    ) else {
                                        continue;
                                    };
                                    let info = UTextLabelGlyphAtlasInfo {
                                        texture: page.texture.clone(),
                                        texture_atlas: page.texture_atlas.clone(),
                                        glyph_index,
                                        offset: glyph_offset,
                                    };
                                    page.glyphs.insert(glyph_key, info.clone());
                                    atlas_pages.push(page);
                                    info
                                }
                            };

                            let Some(atlas_layout) = texture_atlases.get(&atlas_info.texture_atlas)
                            else {
                                continue;
                            };
                            let Some(atlas_image) = images.get(&atlas_info.texture) else {
                                continue;
                            };
                            let glyph_rect = atlas_layout.textures[atlas_info.glyph_index];
                            let glyph_size_pixels =
                                Vec2::new(glyph_rect.width() as f32, glyph_rect.height() as f32);
                            let glyph_size = glyph_size_pixels / glyph_render_scale;

                            let position_x = (glyph_size_pixels.x * 0.5
                                + atlas_info.offset.x as f32
                                + glyph.x * glyph_render_scale)
                                / glyph_render_scale;
                            let position_y = (glyph.y * glyph_render_scale
                                - atlas_info.offset.y as f32
                                + glyph_size_pixels.y * 0.5)
                                / glyph_render_scale;

                            let local_center = Vec2::new(
                                horizontal_offset + (position_x - parley_origin_x) - text_size.x * 0.5,
                                text_size.y * 0.5 - position_y,
                            );
                            let atlas_width = atlas_image.texture_descriptor.size.width as f32;
                            let atlas_height = atlas_image.texture_descriptor.size.height as f32;
                            let uv_min = Vec2::new(
                                glyph_rect.min.x as f32 / atlas_width,
                                glyph_rect.min.y as f32 / atlas_height,
                            );
                            let uv_max = Vec2::new(
                                glyph_rect.max.x as f32 / atlas_width,
                                glyph_rect.max.y as f32 / atlas_height,
                            );

                            let quad = TextGlyphQuad {
                                center: local_center,
                                size: glyph_size,
                                uv_min,
                                uv_max,
                                color: glyph_color,
                            };
                            let clipped_quad = if let Some(clip_rect) = content_clip {
                                clip_quad_to_rect(quad, clip_rect)
                            } else {
                                Some(quad)
                            };
                            if let Some(mut q) = clipped_quad {
                                q.center *= ui_to_world_scale;
                                q.size *= ui_to_world_scale;

                                page_batches
                                    .entry(atlas_info.texture.id())
                                    .and_modify(|batch| batch.quads.push(q.clone()))
                                    .or_insert_with(|| TextPageBatch {
                                        texture: atlas_info.texture.clone(),
                                        quads: vec![q],
                                    });
                            }
                        }
                    }
                }
            }
        }

        for (texture_id, batch) in page_batches {
            let existing = existing_children.remove(&texture_id);
            let mesh = build_text_page_mesh(&batch.quads);

            match existing {
                Some((child_entity, mesh_handle, mat_handle_2d, mat_handle_3d)) => {
                    if let Some(mut existing_mesh) = meshes.get_mut(&mesh_handle) {
                        *existing_mesh = mesh;
                    }
                    if is_3d {
                        if let Some(h) = mat_handle_3d {
                            if let Some(mut existing_material) = materials_3d.get_mut(&h) {
                                *existing_material =
                                    build_text_material_3d(&label, batch.texture.clone());
                            }
                        }
                    } else {
                        if let Some(h) = mat_handle_2d {
                            if let Some(mut existing_material) = materials.get_mut(&h) {
                                *existing_material =
                                    build_text_material(&label, batch.texture.clone());
                            }
                        }
                    }
                    if let Ok(mut cmd) = commands.get_entity(child_entity) {
                        cmd.try_insert(TextRenderPageMarker { texture_id });
                    }
                }
                None => {
                    if let Ok(mut cmd) = commands.get_entity(entity) {
                        cmd.with_children(|parent| {
                            let mut child_cmd = parent.spawn((
                                TextRenderPageMarker { texture_id },
                                TextChildMarker,
                                Transform::default(),
                                Visibility::Inherited,
                            ));
                            if is_3d {
                                child_cmd.insert((
                                    Mesh3d(meshes.add(mesh)),
                                    MeshMaterial3d(
                                        materials_3d
                                            .add(build_text_material_3d(&label, batch.texture.clone())),
                                    ),
                                ));
                            } else {
                                child_cmd.insert((
                                    Mesh2d(meshes.add(mesh)),
                                    MeshMaterial2d(
                                        materials
                                            .add(build_text_material(&label, batch.texture.clone())),
                                    ),
                                ));
                            }
                        });
                    }
                }
            }
        }

        for (_, (stale_child, _, _, _)) in existing_children {
            if let Ok(mut cmd) = commands.get_entity(stale_child) {
                cmd.try_despawn();
            }
        }
    }
}

/// Synchronizes selection highlight meshes behind [`USelectableText`] labels.
pub(crate) fn sync_selectable_text_highlights(
    mut commands: Commands,
    mut atlas_cache: ResMut<UTextLabelAtlasCache>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    mut materials: Option<ResMut<Assets<UTextLabelSdfMaterial>>>,
    mut materials_3d: Option<ResMut<Assets<UTextLabelSdfMaterial3d>>>,
    mut removed_selectable: RemovedComponents<USelectableText>,
    parents_query: Query<&ChildOf>,
    root_query: Query<&ResolvedRootUi>,
    label_query: Query<
        (
            Entity,
            &UTextLabel,
            &USelectableText,
            &UNode,
            &ComputedSize,
            &ComputedTextBlock,
            &UTextLabelLayoutCache,
            Has<UI3d>,
            Option<&CachedUiContext>,
        ),
        Or<(
            Added<USelectableText>,
            Changed<USelectableText>,
            Changed<UTextLabel>,
            Changed<UNode>,
            Changed<ComputedSize>,
            Changed<ComputedTextBlock>,
            Changed<UTextLabelLayoutCache>,
            Added<UI3d>,
            Changed<CachedUiContext>,
        )>,
    >,
    highlight_children: Query<
        (
            Entity,
            &ChildOf,
            Option<&Mesh2d>,
            Option<&Mesh3d>,
            Option<&MeshMaterial2d<UTextLabelSdfMaterial>>,
            Option<&MeshMaterial3d<UTextLabelSdfMaterial3d>>,
        ),
        With<TextSelectionHighlightMarker>,
    >,
) {
    let mut existing_by_parent: HashMap<
        Entity,
        (
            Entity,
            Option<Handle<Mesh>>,
            Option<Handle<UTextLabelSdfMaterial>>,
            Option<Handle<UTextLabelSdfMaterial3d>>,
        ),
    > = HashMap::new();

    for (child, parent, mesh_2d, mesh_3d, mat_2d, mat_3d) in highlight_children.iter() {
        let mesh_handle = mesh_2d.map(|m| m.0.clone()).or_else(|| mesh_3d.map(|m| m.0.clone()));
        let previous = existing_by_parent.insert(
            parent.parent(),
            (
                child,
                mesh_handle,
                mat_2d.map(|m| m.0.clone()),
                mat_3d.map(|m| m.0.clone()),
            ),
        );
        if let Some((duplicate, _, _, _)) = previous
            && let Ok(mut cmd) = commands.get_entity(duplicate)
        {
            cmd.try_despawn();
        }
    }

    for removed_parent in removed_selectable.read() {
        if let Some((stale_child, _, _, _)) = existing_by_parent.remove(&removed_parent)
            && let Ok(mut cmd) = commands.get_entity(stale_child)
        {
            cmd.try_despawn();
        }
    }

    let (Some(ref mut images), Some(ref mut meshes), Some(ref mut materials), Some(ref mut materials_3d)) = (
        images.as_deref_mut(),
        meshes.as_deref_mut(),
        materials.as_deref_mut(),
        materials_3d.as_deref_mut(),
    ) else {
        return;
    };

    for (
        entity,
        label,
        selectable,
        node,
        computed_size,
        computed,
        layout_cache,
        is_3d,
        cached_context,
    ) in label_query.iter()
    {
        let quads = if selectable.enabled {
            selectable
                .selection
                .map(|sel| {
                    compute_text_selection_quads(
                        label,
                        node,
                        computed_size,
                        layout_cache,
                        computed,
                        sel,
                    )
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if quads.is_empty() {
            if let Some((stale_child, _, _, _)) = existing_by_parent.remove(&entity)
                && let Ok(mut cmd) = commands.get_entity(stale_child)
            {
                cmd.try_despawn();
            }
            continue;
        }

        let ui_to_world_scale = resolve_text_mesh_world_scale(
            entity,
            is_3d,
            cached_context,
            &parents_query,
            &root_query,
        );
        let mut scaled_quads = quads;
        if ui_to_world_scale != 1.0 {
            for quad in &mut scaled_quads {
                quad.center *= ui_to_world_scale;
                quad.size *= ui_to_world_scale;
            }
        }

        let mesh = build_text_page_mesh(&scaled_quads);
        let solid_texture = atlas_cache.ensure_solid_highlight_texture(images);
        let highlight_color: LinearRgba = selectable.highlight_color.into();

        let existing = existing_by_parent.remove(&entity);
        let can_reuse = existing.as_ref().is_some_and(|(_, _, mat_2d, mat_3d)| {
            if is_3d {
                mat_3d.is_some()
            } else {
                mat_2d.is_some()
            }
        });

        if can_reuse && let Some((_, Some(mesh_handle), mat_2d, mat_3d)) = existing {
            if let Some(mut existing_mesh) = meshes.get_mut(&mesh_handle) {
                *existing_mesh = mesh;
            }
            if is_3d {
                if let Some(h) = mat_3d
                    && let Some(mut mat) = materials_3d.get_mut(&h)
                {
                    *mat = build_solid_highlight_material_3d(highlight_color, solid_texture);
                }
            } else if let Some(h) = mat_2d
                && let Some(mut mat) = materials.get_mut(&h)
            {
                *mat = build_solid_highlight_material(highlight_color, solid_texture);
            }
            continue;
        }

        if let Some((stale_child, _, _, _)) = existing
            && let Ok(mut cmd) = commands.get_entity(stale_child)
        {
            cmd.try_despawn();
        }

        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.with_children(|parent| {
                let mut child_cmd = parent.spawn((
                    TextSelectionHighlightMarker,
                    Transform::default(),
                    Visibility::Inherited,
                ));
                if is_3d {
                    child_cmd.insert((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(materials_3d.add(
                            build_solid_highlight_material_3d(highlight_color, solid_texture),
                        )),
                    ));
                } else {
                    child_cmd.insert((
                        Mesh2d(meshes.add(mesh)),
                        MeshMaterial2d(materials.add(build_solid_highlight_material(
                            highlight_color,
                            solid_texture,
                        ))),
                    ));
                }
            });
        }
    }
}
