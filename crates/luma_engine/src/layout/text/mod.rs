//! Text rendering primitive for Univis UI.
//!
//! [`UText`](crate::layout::text::UText) measures text through
//! Bevy's text pipeline, then renders the final result through Univis SDF
//! materials so the same primitive stays sharp in screen and world roots.

use crate::schedule::{UiSettlementSchedule, UnivisPostUpdateSet};
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

mod measure;
mod model;
mod render;
mod selection;

use self::measure::invalidation::mark_text_label_layout_dirty;
pub use self::measure::{
    fit_node_to_text_size, measure_text_label_layout, sync_text_label_intrinsic_size,
};
pub use self::model::{
    TextChildMarker, TextSelectionHighlightMarker, UCopyText, USelectableText, UText, UTextLabel,
    UTextLabelLayoutCache, UTextOverflow, UTextSpan, UTextSpansTemplate, UTextTruncateSide,
    UTextWrapMode,
};
pub use self::selection::{
    UTextSelectionRect, compute_text_selection_rects, local_logical_to_parley,
    selectable_effective_text, selectable_text_extracted_copy, text_byte_index_from_local_point,
    text_caret_rect_from_byte_index, text_move_caret_line_edge, text_move_caret_vertical,
    text_word_range_from_local_point,
};
use self::render::atlas::UTextLabelAtlasCache;
use self::render::clip_sync::{sync_text_clipper_materials, sync_text_render_transforms};
pub(crate) use self::render::UTextLabelSdfMaterial3d;
use self::render::{
    UTextLabelSdfMaterial, sync_selectable_text_highlights, sync_text_label_meshes,
};

#[cfg(test)]
use self::measure::bidi::{
    ISOLATE_END, LTR_ISOLATE_START, TextBaseDirection, base_direction_for_text,
    build_truncate_candidate, resolve_truncate_side, text_char_boundaries,
};
#[cfg(test)]
use self::measure::bounds::{
    desired_text_label_intrinsic_size, label_measure_bounds, measured_text_outer_size,
};
#[cfg(test)]
use self::render::clip_sync::{LocalClipRect, clip_quad_to_rect, label_content_clip_rect};
#[cfg(test)]
use self::render::mesh::{
    TextGlyphQuad, build_text_material, build_text_page_mesh, text_horizontal_offset,
};
#[cfg(test)]
use crate::layout::geometry::{USides, UVal};
#[cfg(test)]
use crate::layout::query::{ComputedSize, IntrinsicSize};
#[cfg(test)]
use crate::layout::univis_node::UNode;
#[cfg(test)]
use bevy::text::TextBounds;

/// Registers the `UText` measurement and SDF rendering pipeline.
pub struct UnivisTextPlugin;

impl Plugin for UnivisTextPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UTextLabel>()
            .register_type::<UTextSpan>()
            .register_type::<USelectableText>()
            .register_type::<UTextWrapMode>()
            .register_type::<UTextOverflow>()
            .register_type::<UTextTruncateSide>()
            .register_type::<UTextLabelLayoutCache>()
            .init_resource::<UTextLabelAtlasCache>()
            .add_plugins(Material2dPlugin::<UTextLabelSdfMaterial>::default())
            .add_plugins(MaterialPlugin::<UTextLabelSdfMaterial3d>::default());
        crate::layout::render::depth_sync::install_material2d_depth_sync::<UTextLabelSdfMaterial>(
            app,
        );
        crate::layout::render::depth_sync::install_material3d_transform_sync::<
            UTextLabelSdfMaterial3d,
        >(app);
        app.add_systems(
                UiSettlementSchedule,
                measure_text_label_layout
                    .in_set(UnivisPostUpdateSet::ExternalPrepare)
                    .before(sync_text_label_intrinsic_size),
            )
            .add_systems(
                UiSettlementSchedule,
                sync_text_label_intrinsic_size
                    .in_set(UnivisPostUpdateSet::ExternalPrepare)
                    .before(fit_node_to_text_size),
            )
            .add_systems(
                UiSettlementSchedule,
                fit_node_to_text_size
                    .in_set(UnivisPostUpdateSet::ExternalPrepare)
                    .before(mark_text_label_layout_dirty),
            )
            .add_systems(
                UiSettlementSchedule,
                mark_text_label_layout_dirty
                    .in_set(UnivisPostUpdateSet::ExternalPrepare)
                    .before(sync_text_label_meshes),
            )
            .add_systems(
                UiSettlementSchedule,
                (
                    sync_text_label_meshes,
                    sync_selectable_text_highlights,
                    sync_text_render_transforms,
                )
                    .chain()
                    .after(crate::layout::layout_system::sync_cached_ui3d)
                    .in_set(UnivisPostUpdateSet::RenderSync),
            )
            .add_systems(
                PostUpdate,
                sync_text_clipper_materials
                    .after(crate::layout::settlement_loop::run_ui_settlement_loop)
                    .after(bevy::transform::TransformSystems::Propagate),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_quad_to_rect_clips_size_and_uvs() {
        let quad = TextGlyphQuad {
            center: Vec2::new(0.0, 0.0),
            size: Vec2::new(10.0, 6.0),
            uv_min: Vec2::new(0.1, 0.2),
            uv_max: Vec2::new(0.5, 0.8),
            color: LinearRgba::WHITE,
        };
        let clip_rect = LocalClipRect {
            min: Vec2::new(-2.0, -3.0),
            max: Vec2::new(5.0, 3.0),
        };

        let clipped = clip_quad_to_rect(quad, clip_rect).expect("quad should be clipped");

        assert_eq!(clipped.size, Vec2::new(7.0, 6.0));
        assert_eq!(clipped.center, Vec2::new(1.5, 0.0));
        assert_eq!(clipped.color, LinearRgba::WHITE);
        assert!((clipped.uv_min.x - 0.22).abs() < 0.001);
        assert!((clipped.uv_max.x - 0.5).abs() < 0.001);
    }

    #[test]
    fn clip_quad_to_rect_returns_none_when_outside() {
        let quad = TextGlyphQuad {
            center: Vec2::new(20.0, 0.0),
            size: Vec2::new(4.0, 4.0),
            uv_min: Vec2::ZERO,
            uv_max: Vec2::ONE,
            color: LinearRgba::WHITE,
        };
        let clip_rect = LocalClipRect {
            min: Vec2::new(-5.0, -5.0),
            max: Vec2::new(5.0, 5.0),
        };

        assert!(clip_quad_to_rect(quad, clip_rect).is_none());
    }

    #[test]
    fn desired_intrinsic_size_uses_content_dimensions_only() {
        let node = UNode {
            width: UVal::Content,
            height: UVal::Px(48.0),
            padding: USides::axes(6.0, 4.0),
            ..default()
        };
        let layout_cache = UTextLabelLayoutCache {
            min_content_size: Vec2::new(48.0, 14.0),
            max_content_size: Vec2::new(120.0, 22.0),
            ..default()
        };
        let current = IntrinsicSize {
            width: 1.0,
            height: 48.0,
            ..default()
        };

        let desired = desired_text_label_intrinsic_size(
            &UTextLabel::default(),
            &node,
            &layout_cache,
            current,
        );

        assert_eq!(desired.width, 132.0);
        assert_eq!(desired.height, 48.0);
        assert_eq!(desired.min_width, 60.0);
        assert_eq!(desired.max_width, 132.0);
    }

    #[test]
    fn desired_intrinsic_size_uses_min_content_dimensions_when_requested() {
        let node = UNode {
            width: UVal::MinContent,
            height: UVal::MinContent,
            padding: USides::axes(6.0, 4.0),
            ..default()
        };
        let layout_cache = UTextLabelLayoutCache {
            min_content_size: Vec2::new(48.0, 14.0),
            max_content_size: Vec2::new(120.0, 22.0),
            ..default()
        };

        let desired = desired_text_label_intrinsic_size(
            &UTextLabel::default(),
            &node,
            &layout_cache,
            IntrinsicSize::default(),
        );

        assert_eq!(desired.width, 60.0);
        assert_eq!(desired.height, 22.0);
        assert_eq!(desired.min_width, 60.0);
        assert_eq!(desired.max_width, 132.0);
        assert_eq!(desired.min_height, 22.0);
        assert_eq!(desired.max_height, 30.0);
    }

    #[test]
    fn measured_text_outer_size_includes_padding_for_empty_text() {
        let node = UNode {
            padding: USides::axes(8.0, 10.0),
            ..default()
        };
        let layout_cache = UTextLabelLayoutCache {
            measured_size: Vec2::ZERO,
            ..default()
        };

        let outer = measured_text_outer_size(&node, &layout_cache);

        assert_eq!(outer, Vec2::new(16.0, 20.0));
    }

    #[test]
    fn grapheme_boundaries_keep_joined_emoji_together() {
        let boundaries = text_char_boundaries("A🧑‍💻B");

        assert_eq!(boundaries.len(), 4);
        assert_eq!(&"A🧑‍💻B"[..boundaries[2]], "A🧑‍💻");
    }

    #[test]
    fn base_direction_detects_rtl_text() {
        assert_eq!(
            base_direction_for_text("مرحبا بالعالم"),
            TextBaseDirection::Rtl
        );
        assert_eq!(
            base_direction_for_text("hello world"),
            TextBaseDirection::Ltr
        );
    }

    #[test]
    fn auto_truncate_side_defaults_to_end_for_ltr_and_rtl() {
        assert_eq!(
            resolve_truncate_side(UTextTruncateSide::Auto, TextBaseDirection::Ltr),
            UTextTruncateSide::End
        );
        assert_eq!(
            resolve_truncate_side(UTextTruncateSide::Auto, TextBaseDirection::Rtl),
            UTextTruncateSide::End
        );
    }

    #[test]
    fn default_text_label_uses_ellipsis_overflow() {
        let label = UTextLabel::default();

        assert_eq!(label.overflow, UTextOverflow::Ellipsis);
    }

    #[test]
    fn default_text_label_uses_auto_truncate_side() {
        let label = UTextLabel::default();

        assert_eq!(label.truncate_side, UTextTruncateSide::Auto);
    }

    #[test]
    fn left_justify_offsets_text_toward_start_edge() {
        let label = UTextLabel {
            justify: Justify::Left,
            ..default()
        };
        let node = UNode {
            padding: USides::axes(10.0, 0.0),
            ..default()
        };
        let computed_size = ComputedSize {
            width: 100.0,
            height: 20.0,
            ..default()
        };

        let offset = text_horizontal_offset(&label, &node, &computed_size, Vec2::new(200.0, 20.0));

        assert_eq!(offset, 60.0);
    }

    #[test]
    fn right_justify_offsets_text_toward_end_edge() {
        let label = UTextLabel {
            justify: Justify::Right,
            ..default()
        };
        let node = UNode {
            padding: USides::axes(10.0, 0.0),
            ..default()
        };
        let computed_size = ComputedSize {
            width: 100.0,
            height: 20.0,
            ..default()
        };

        let offset = text_horizontal_offset(&label, &node, &computed_size, Vec2::new(40.0, 20.0));

        assert_eq!(offset, 20.0);
    }

    #[test]
    fn center_justify_keeps_zero_offset() {
        let label = UTextLabel {
            justify: Justify::Center,
            ..default()
        };
        let node = UNode {
            padding: USides::axes(10.0, 0.0),
            ..default()
        };
        let computed_size = ComputedSize {
            width: 100.0,
            height: 20.0,
            ..default()
        };

        let offset = text_horizontal_offset(&label, &node, &computed_size, Vec2::new(40.0, 20.0));

        assert_eq!(offset, 0.0);
    }

    #[test]
    fn local_clip_rect_exists_for_ellipsis_even_with_autosize() {
        let label = UTextLabel {
            autosize: true,
            overflow: UTextOverflow::Ellipsis,
            ..default()
        };
        let node = UNode {
            padding: USides::axes(10.0, 6.0),
            ..default()
        };
        let computed_size = ComputedSize {
            width: 120.0,
            height: 40.0,
            ..default()
        };

        let clip = label_content_clip_rect(&label, &node, &computed_size).unwrap();

        assert_eq!(clip.min, Vec2::new(-50.0, -14.0));
        assert_eq!(clip.max, Vec2::new(50.0, 14.0));
    }

    #[test]
    fn visible_overflow_disables_local_clip_rect() {
        let label = UTextLabel {
            overflow: UTextOverflow::Visible,
            ..default()
        };
        let node = UNode {
            padding: USides::all(8.0),
            ..default()
        };
        let computed_size = ComputedSize {
            width: 120.0,
            height: 40.0,
            ..default()
        };

        assert!(label_content_clip_rect(&label, &node, &computed_size).is_none());
    }

    #[test]
    fn autosize_measure_bounds_use_parent_constraints_when_available() {
        let label = UTextLabel {
            autosize: true,
            overflow: UTextOverflow::Ellipsis,
            ..default()
        };
        let node = UNode::default();
        let bounds = label_measure_bounds(
            &label,
            &node,
            None,
            TextBounds {
                width: Some(180.0),
                height: Some(44.0),
            },
        );

        assert_eq!(bounds.width, Some(180.0));
        assert_eq!(bounds.height, Some(44.0));
    }

    #[test]
    fn truncate_candidate_end_keeps_prefix() {
        let text = "abcdef";
        let boundaries = text_char_boundaries(text);
        let candidate = build_truncate_candidate(
            text,
            &boundaries,
            UTextTruncateSide::End,
            3,
            TextBaseDirection::Ltr,
        );

        assert_eq!(candidate, format!("{LTR_ISOLATE_START}abc{ISOLATE_END}..."));
    }

    #[test]
    fn truncate_candidate_start_keeps_suffix() {
        let text = "abcdef";
        let boundaries = text_char_boundaries(text);
        let candidate = build_truncate_candidate(
            text,
            &boundaries,
            UTextTruncateSide::Start,
            3,
            TextBaseDirection::Ltr,
        );

        assert_eq!(candidate, format!("...{LTR_ISOLATE_START}def{ISOLATE_END}"));
    }

    #[test]
    fn truncate_candidate_middle_keeps_edges() {
        let text = "abcdef";
        let boundaries = text_char_boundaries(text);
        let candidate = build_truncate_candidate(
            text,
            &boundaries,
            UTextTruncateSide::Middle,
            4,
            TextBaseDirection::Ltr,
        );

        assert_eq!(
            candidate,
            format!("{LTR_ISOLATE_START}ab{ISOLATE_END}...{LTR_ISOLATE_START}ef{ISOLATE_END}")
        );
    }

    #[test]
    fn wrapping_text_measure_bounds_constrains_width_to_parent_even_when_overflow_visible() {
        let label =
            UTextLabel::multiline("A long paragraph of text that should wrap to parent width");
        let node = UNode::default();
        let bounds = label_measure_bounds(
            &label,
            &node,
            None,
            TextBounds {
                width: Some(320.0),
                height: Some(100.0),
            },
        );

        // Width must be constrained to parent to force word wrapping
        assert_eq!(bounds.width, Some(320.0));
        // Height must be unconstrained because overflow is Visible, allowing vertical expansion
        assert_eq!(bounds.height, None);
    }

    #[test]
    fn wrapping_text_desired_intrinsic_size_uses_measured_width_not_unwrapped_max() {
        let label = UTextLabel::multiline("Wrapped prose");
        let node = UNode {
            width: UVal::Auto,
            height: UVal::Auto,
            padding: USides::axes(8.0, 4.0),
            ..default()
        };
        let layout_cache = UTextLabelLayoutCache {
            min_content_size: Vec2::new(60.0, 18.0),
            max_content_size: Vec2::new(1200.0, 18.0), // Unwrapped single-line would be 1200px
            measured_size: Vec2::new(300.0, 72.0),     // Wrapped 4 lines: 300px wide, 72px tall
            ..default()
        };

        let desired = desired_text_label_intrinsic_size(
            &label,
            &node,
            &layout_cache,
            IntrinsicSize::default(),
        );

        // Width should be wrapped width + padding, NOT 1200 + padding!
        assert_eq!(desired.width, 316.0);
        assert_eq!(desired.height, 80.0);
        assert_eq!(desired.min_width, 76.0);
        assert_eq!(desired.max_width, 316.0);
    }

    #[test]
    fn clamped_text_label_sets_effective_max_lines_and_word_boundary() {
        let label = UTextLabel::clamped("Three lines max", 3);

        assert_eq!(label.wrap_mode, UTextWrapMode::ClampLines(3));
        assert_eq!(label.effective_max_lines(), Some(3));
        assert!(label.is_wrapping());
        assert_eq!(label.overflow, UTextOverflow::Ellipsis);
    }

    #[test]
    fn explicit_node_width_takes_precedence_over_wider_parent() {
        let label = UTextLabel::multiline("Constrained by node width");
        let node = UNode {
            width: UVal::Px(180.0),
            padding: USides::all(10.0),
            ..default()
        };
        let bounds = label_measure_bounds(
            &label,
            &node,
            None,
            TextBounds {
                width: Some(500.0),
                height: None,
            },
        );

        // 180 - padding (20) = 160.0, which is smaller than parent 500.0
        assert_eq!(bounds.width, Some(160.0));
    }

    #[test]
    fn parent_width_takes_precedence_over_wider_explicit_node() {
        let label = UTextLabel::multiline("Constrained by parent container");
        let node = UNode {
            width: UVal::Px(600.0),
            padding: USides::all(0.0),
            ..default()
        };
        let bounds = label_measure_bounds(
            &label,
            &node,
            None,
            TextBounds {
                width: Some(350.0),
                height: None,
            },
        );

        // Parent width 350.0 is smaller than node width 600.0 -> must constrain to 350.0!
        assert_eq!(bounds.width, Some(350.0));
    }

    #[test]
    fn rich_text_spans_concatenates_text_and_preserves_spans() {
        let label = UTextLabel::rich([
            UTextSpan::new("Hello "),
            UTextSpan::new("World").with_color(Color::srgb(1.0, 0.0, 0.0)),
        ])
        .with_span(UTextSpan::new("!").with_font_size(24.0));

        assert_eq!(label.text, "Hello World!");
        assert_eq!(label.spans.len(), 3);
        assert_eq!(label.spans[0].text, "Hello ");
        assert_eq!(label.spans[1].text, "World");
        assert_eq!(label.spans[1].color, Some(Color::srgb(1.0, 0.0, 0.0)));
        assert_eq!(label.spans[2].text, "!");
        assert_eq!(label.spans[2].font_size, Some(24.0));
        assert!(label.has_spans());
    }

    #[test]
    fn outline_properties_and_builders() {
        let label = UTextLabel::new("Outlined")
            .with_outline(2.5, Color::srgb(0.0, 0.0, 0.0))
            .with_outline_width(3.0);

        assert_eq!(label.outline_width, 3.0);
        assert_eq!(label.outline_color, Color::srgb(0.0, 0.0, 0.0));
    }

    #[test]
    fn build_text_material_sets_normalized_outline_and_base_color() {
        // Plain label: base color matches label.color, outline normalized from render scale
        let plain_label = UTextLabel::new("Plain")
            .with_render_scale(8.0)
            .with_outline(1.5, Color::BLACK);
        let mat = build_text_material(&plain_label, Handle::default());
        // normalized_outline = (1.5 * 8.0) / (2.0 * 12.0) = 12.0 / 24.0 = 0.5
        assert!((mat.outline_width - 0.5).abs() < 0.001);
        assert_eq!(mat.color, LinearRgba::from(Color::WHITE));

        // Rich text label: base color is LinearRgba::WHITE to let vertex colors take over
        let rich_label = UTextLabel::rich([
            UTextSpan::new("A").with_color(Color::srgb(1.0, 0.0, 0.0)),
            UTextSpan::new("B").with_color(Color::srgb(0.0, 1.0, 0.0)),
        ]);
        let rich_mat = build_text_material(&rich_label, Handle::default());
        assert_eq!(rich_mat.color, LinearRgba::WHITE);
    }

    #[test]
    fn build_text_page_mesh_inserts_vertex_colors() {
        let quads = [
            TextGlyphQuad {
                center: Vec2::ZERO,
                size: Vec2::splat(10.0),
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
                color: LinearRgba::RED,
            },
            TextGlyphQuad {
                center: Vec2::X * 15.0,
                size: Vec2::splat(10.0),
                uv_min: Vec2::ZERO,
                uv_max: Vec2::ONE,
                color: LinearRgba::BLUE,
            },
        ];

        let mesh = build_text_page_mesh(&quads);
        let colors = mesh
            .attribute(Mesh::ATTRIBUTE_COLOR)
            .expect("Mesh must contain ATTRIBUTE_COLOR");

        match colors {
            bevy::mesh::VertexAttributeValues::Float32x4(values) => {
                assert_eq!(values.len(), 8); // 2 quads * 4 vertices = 8
                // First 4 vertices are RED
                for v in &values[0..4] {
                    assert_eq!(*v, [1.0, 0.0, 0.0, 1.0]);
                }
                // Next 4 vertices are BLUE
                for v in &values[4..8] {
                    assert_eq!(*v, [0.0, 0.0, 1.0, 1.0]);
                }
            }
            _ => panic!("Expected Float32x4 attribute values for ATTRIBUTE_COLOR"),
        }
    }

    #[test]
    fn selectable_text_range_and_extraction() {
        let mut sel = USelectableText::new().with_selection(2, 7);
        let sample = "Hello World!";
        assert_eq!(sel.selected_text(sample), Some("llo W"));

        // Inverted indices normalize automatically
        let sel_inv = USelectableText::new().with_selection(7, 2);
        assert_eq!(sel_inv.selected_text(sample), Some("llo W"));

        // Select all
        sel.select_all(sample.len());
        assert_eq!(sel.selected_text(sample), Some("Hello World!"));

        // Clear
        sel.clear();
        assert_eq!(sel.selected_text(sample), None);
    }

    #[test]
    fn text_node_depth_updates_when_zindex_changes() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            crate::layout::UnivisLayoutPlugin,
            UnivisTextPlugin,
        ));

        let root = app
            .world_mut()
            .spawn((
                crate::layout::layout_system::URootUi::screen(),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Percent(1.0),
                    height: crate::layout::geometry::UVal::Percent(1.0),
                    ..default()
                },
            ))
            .id();

        let panel_a = app
            .world_mut()
            .spawn((
                ChildOf(root),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Px(200.0),
                    height: crate::layout::geometry::UVal::Px(100.0),
                    ..default()
                },
                crate::layout::univis_node::UZIndex::Local(1),
            ))
            .id();

        let text_a = app
            .world_mut()
            .spawn((
                ChildOf(panel_a),
                UTextLabel::new("Alpha"),
            ))
            .id();

        let panel_b = app
            .world_mut()
            .spawn((
                ChildOf(root),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Px(200.0),
                    height: crate::layout::geometry::UVal::Px(100.0),
                    ..default()
                },
                crate::layout::univis_node::UZIndex::Local(2),
            ))
            .id();

        let text_b = app
            .world_mut()
            .spawn((
                ChildOf(panel_b),
                UTextLabel::new("Beta"),
            ))
            .id();

        app.update();

        let picking_a = app.world().entity(text_a).get::<crate::layout::query::UiPickingContext>().unwrap().local_depth_key;
        let picking_b = app.world().entity(text_b).get::<crate::layout::query::UiPickingContext>().unwrap().local_depth_key;
        assert!(picking_a < picking_b, "text_b should have greater depth than text_a (a={picking_a}, b={picking_b})");

        // Swap priority: panel_a gets LocalMax
        *app.world_mut().entity_mut(panel_a).get_mut::<crate::layout::univis_node::UZIndex>().unwrap() = crate::layout::univis_node::UZIndex::LocalMax;

        app.update();

        let picking_a_after = app.world().entity(text_a).get::<crate::layout::query::UiPickingContext>().unwrap().local_depth_key;
        let picking_b_after = app.world().entity(text_b).get::<crate::layout::query::UiPickingContext>().unwrap().local_depth_key;
        assert!(
            picking_a_after > picking_b_after,
            "text_a should now have greater depth than text_b after UZIndex change (a={picking_a_after}, b={picking_b_after})"
        );
    }

    #[test]
    fn dense_screen_tree_keeps_text_z_between_siblings_and_world2d_inside_capsule() {
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
        .init_asset::<TextureAtlasLayout>();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn(Camera2d);

        let font = app.world_mut().resource_mut::<Assets<Font>>().add(Font::from_bytes(
            include_bytes!("../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf")
                .to_vec(),
        ));

        let screen_root = app
            .world_mut()
            .spawn((
                crate::layout::layout_system::URootUi::screen(),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Percent(1.0),
                    height: crate::layout::geometry::UVal::Percent(1.0),
                    ..default()
                },
            ))
            .id();

        // Spawn 1300 siblings so paint_order > 1200 and slot_span = 1/total < 0.001,
        // with a UTextLabel inside the first sibling.
        let mut siblings = Vec::with_capacity(1300);
        for _ in 0..1300 {
            let panel = app
                .world_mut()
                .spawn((
                    ChildOf(screen_root),
                    crate::layout::univis_node::UNode {
                        width: crate::layout::geometry::UVal::Px(40.0),
                        height: crate::layout::geometry::UVal::Px(20.0),
                        ..default()
                    },
                ))
                .id();
            siblings.push(panel);
        }

        let dense_label = app
            .world_mut()
            .spawn((
                ChildOf(siblings[0]),
                UTextLabel {
                    text: "Alpha".to_string(),
                    font: font.clone(),
                    ..default()
                },
            ))
            .id();

        // Also spawn a World2d root with UTextLabel to verify capsule_band_step (0.00004) ceiling safety.
        let world_root = app
            .world_mut()
            .spawn((
                crate::layout::layout_system::URootUi::world_2d(Vec2::new(400.0, 200.0)),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Px(400.0),
                    height: crate::layout::geometry::UVal::Px(200.0),
                    ..default()
                },
            ))
            .id();
        let world_label = app
            .world_mut()
            .spawn((
                ChildOf(world_root),
                UTextLabel {
                    text: "World2d Text".to_string(),
                    font,
                    ..default()
                },
            ))
            .id();

        app.update();
        app.update();
        app.update();

        let text_0 = app
            .world()
            .entity(dense_label)
            .get::<Children>()
            .unwrap()
            .iter()
            .find(|e| app.world().entity(*e).contains::<TextChildMarker>())
            .expect("dense_label should have a TextChildMarker child");

        let z_label_0 = app
            .world()
            .entity(dense_label)
            .get::<GlobalTransform>()
            .unwrap()
            .translation()
            .z;
        let z_text_0 = app
            .world()
            .entity(text_0)
            .get::<GlobalTransform>()
            .unwrap()
            .translation()
            .z;
        let z_sibling_1 = app
            .world()
            .entity(siblings[1])
            .get::<GlobalTransform>()
            .unwrap()
            .translation()
            .z;
        assert!(
            z_label_0 < z_text_0 && z_text_0 < z_sibling_1,
            "dense screen text child must stay between label ({z_label_0}) and next sibling ({z_sibling_1}), got {z_text_0}"
        );

        let world_stack = *app
            .world()
            .entity(world_root)
            .get::<crate::layout::layout_system::ResolvedRootStack>()
            .unwrap();
        let world_text = app
            .world()
            .entity(world_label)
            .get::<Children>()
            .unwrap()
            .iter()
            .find(|e| app.world().entity(*e).contains::<TextChildMarker>())
            .expect("world_label should have a TextChildMarker child");
        let z_world_label = app
            .world()
            .entity(world_label)
            .get::<GlobalTransform>()
            .unwrap()
            .translation()
            .z;
        let z_world_text = app
            .world()
            .entity(world_text)
            .get::<GlobalTransform>()
            .unwrap()
            .translation()
            .z;
        assert!(
            z_world_label < z_world_text && z_world_text < world_stack.capsule_ceiling(),
            "World2d text child ({z_world_text}) must sit above parent ({z_world_label}) and inside capsule ceiling ({})",
            world_stack.capsule_ceiling()
        );
    }

    #[test]
    fn translated_clipper_updates_text_clip_uniforms_in_same_frame() {
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
        .init_asset::<TextureAtlasLayout>();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn(Camera2d);

        let font = app.world_mut().resource_mut::<Assets<Font>>().add(Font::from_bytes(
            include_bytes!("../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf")
                .to_vec(),
        ));

        let root = app
            .world_mut()
            .spawn((
                crate::layout::layout_system::URootUi::screen(),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Percent(1.0),
                    height: crate::layout::geometry::UVal::Percent(1.0),
                    ..default()
                },
            ))
            .id();

        let clipper = app
            .world_mut()
            .spawn((
                ChildOf(root),
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Px(200.0),
                    height: crate::layout::geometry::UVal::Px(100.0),
                    ..default()
                },
                crate::layout::univis_node::UClip::enabled(true),
            ))
            .id();

        let label = app
            .world_mut()
            .spawn((
                ChildOf(clipper),
                UTextLabel {
                    text: "Clipped Text".to_string(),
                    font,
                    ..default()
                },
            ))
            .id();

        app.update();
        app.update();
        app.update();

        let text_child = app
            .world()
            .entity(label)
            .get::<Children>()
            .unwrap()
            .iter()
            .find(|e| app.world().entity(*e).contains::<TextChildMarker>())
            .expect("label should spawn TextChildMarker");

        let mat_handle = app
            .world()
            .entity(text_child)
            .get::<MeshMaterial2d<render::UTextLabelSdfMaterial>>()
            .expect("TextChildMarker should have UTextLabelSdfMaterial")
            .0
            .clone();

        let before_w = app
            .world()
            .resource::<Assets<render::UTextLabelSdfMaterial>>()
            .get(&mat_handle)
            .unwrap()
            .clip_inv_x[0]
            .w;

        // Translate the clipper and run ONE app.update(): PostUpdate sync must see the updated GlobalTransform.
        app.world_mut()
            .entity_mut(clipper)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .x += 120.0;
        app.update();

        let after_w = app
            .world()
            .resource::<Assets<render::UTextLabelSdfMaterial>>()
            .get(&mat_handle)
            .unwrap()
            .clip_inv_x[0]
            .w;
        assert!(
            (after_w - (before_w - 120.0)).abs() < 1e-3,
            "PostUpdate clip sync must observe propagated clipper translation in the same frame (before={before_w}, after={after_w})"
        );
    }

    #[test]
    fn world3d_text_and_selection_highlight_scale_meshes_once_without_double_transform_scaling() {
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
        .init_asset::<TextureAtlasLayout>();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn((Camera::default(), Camera3d::default()));

        let font = app.world_mut().resource_mut::<Assets<Font>>().add(
            Font::from_bytes(
                include_bytes!("../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf")
                    .to_vec(),
            ),
        );

        let mut root_ui =
            crate::layout::layout_system::URootUi::world_3d(Vec2::new(400.0, 200.0));
        root_ui.meters_per_unit = 0.01;
        let root = app
            .world_mut()
            .spawn((
                root_ui,
                crate::layout::univis_node::UNode {
                    width: crate::layout::geometry::UVal::Percent(1.0),
                    height: crate::layout::geometry::UVal::Percent(1.0),
                    ..default()
                },
            ))
            .id();

        let label = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UTextLabel {
                    text: "Hello 3D".into(),
                    font,
                    font_size: 24.0,
                    ..default()
                },
                USelectableText::new().with_selection(0, 8),
            ))
            .id();

        for _ in 0..6 {
            app.update();
        }

        let children: Vec<Entity> = app
            .world()
            .entity(label)
            .get::<Children>()
            .expect("World3d UTextLabel should spawn text and highlight children")
            .iter()
            .collect();

        let text_child = children
            .iter()
            .copied()
            .find(|&e| app.world().entity(e).contains::<TextChildMarker>())
            .expect("should spawn TextChildMarker child");
        let highlight_child = children
            .iter()
            .copied()
            .find(|&e| app.world().entity(e).contains::<TextSelectionHighlightMarker>())
            .expect("should spawn TextSelectionHighlightMarker child");

        let text_tf = app.world().entity(text_child).get::<Transform>().unwrap();
        let hl_tf = app.world().entity(highlight_child).get::<Transform>().unwrap();
        assert_eq!(
            text_tf.scale,
            Vec3::ONE,
            "Mesh3d vertices are already scaled to meters; Transform scale must remain ONE"
        );
        assert_eq!(
            hl_tf.scale,
            Vec3::ONE,
            "Highlight Mesh3d vertices are already scaled to meters; Transform scale must remain ONE"
        );

        let mesh_width = |entity: Entity| -> f32 {
            let mesh_handle = app
                .world()
                .entity(entity)
                .get::<Mesh3d>()
                .expect("World3d child must use Mesh3d")
                .0
                .clone();
            let mesh = app.world().resource::<Assets<Mesh>>().get(&mesh_handle).unwrap();
            let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("missing positions");
            };
            let min_x = positions.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = positions
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max);
            max_x - min_x
        };

        let text_width_m = mesh_width(text_child);
        let hl_width_m = mesh_width(highlight_child);
        assert!(
            text_width_m > 0.2 && text_width_m < 3.0,
            "expected text mesh scaled once by 0.01 (~0.5-1.5m), got {text_width_m}m"
        );
        assert!(
            hl_width_m > 0.2 && hl_width_m < 3.0,
            "expected highlight mesh scaled once by 0.01 (~0.5-1.5m), got {hl_width_m}m"
        );

        let hl_mat_handle = app
            .world()
            .entity(highlight_child)
            .get::<MeshMaterial3d<UTextLabelSdfMaterial3d>>()
            .unwrap()
            .0
            .clone();
        let hl_mat = app
            .world()
            .resource::<Assets<UTextLabelSdfMaterial3d>>()
            .get(&hl_mat_handle)
            .unwrap();
        assert_eq!(
            Material::alpha_mode(hl_mat),
            AlphaMode::Premultiplied,
            "UTextLabelSdfMaterial3d must use AlphaMode::Premultiplied"
        );
    }
}
