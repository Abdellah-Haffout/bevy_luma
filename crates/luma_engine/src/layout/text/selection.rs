use bevy::prelude::*;
use bevy::text::ComputedTextBlock;
use parley::Affinity;
use parley::editing::{Cursor, Selection};
use std::borrow::Cow;

use crate::layout::query::ComputedSize;
use crate::layout::univis_node::UNode;

use super::measure::bidi::{ISOLATE_END, LTR_ISOLATE_START};
use super::model::{USelectableText, UTextLabel, UTextLabelLayoutCache};
use super::render::clip_sync::{clip_quad_to_rect, label_content_clip_rect};
use super::render::mesh::{TextGlyphQuad, text_horizontal_offset};

const RTL_ISOLATE_START: char = '\u{2067}';

/// A node-local axis-aligned rectangle representing one line/run highlight quad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UTextSelectionRect {
    /// Center position in node-local logical UI coordinates (`+Y` up, origin at node center).
    pub center: Vec2,
    /// Dimensions in logical UI pixels.
    pub size: Vec2,
}

fn snap_byte_index(text: &str, index: usize, round_up: bool) -> usize {
    let clamped = index.min(text.len());
    if text.is_char_boundary(clamped) {
        return clamped;
    }

    if round_up {
        let mut idx = clamped;
        while idx < text.len() && !text.is_char_boundary(idx) {
            idx += 1;
        }
        idx
    } else {
        let mut idx = clamped;
        while idx > 0 && !text.is_char_boundary(idx) {
            idx -= 1;
        }
        idx
    }
}

/// Returns the laid-out display string matching [`ComputedTextBlock`]'s byte indices.
pub fn selectable_effective_text<'a>(
    label: &'a UTextLabel,
    layout_cache: &'a UTextLabelLayoutCache,
) -> Cow<'a, str> {
    if !layout_cache.displayed_text.is_empty() {
        return Cow::Borrowed(layout_cache.displayed_text.as_str());
    }
    if label.text.is_empty() && !label.spans.is_empty() {
        let joined: String = label.spans.iter().map(|s| s.text.as_str()).collect();
        return Cow::Owned(joined);
    }
    Cow::Borrowed(label.text.as_str())
}

/// Converts a node-local logical UI point (`+Y` up, center-origin) to Parley layout coordinates (`+Y` down, top-left of `text_size`).
pub fn local_logical_to_parley(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    local_point: Vec2,
) -> Vec2 {
    let text_size = layout_cache.measured_size;
    let horizontal_offset = text_horizontal_offset(label, node, computed_size, text_size);
    Vec2::new(
        local_point.x - horizontal_offset + text_size.x * 0.5,
        text_size.y * 0.5 - local_point.y,
    )
}

/// Returns the leftmost horizontal origin inside `computed`'s Parley layout so container-aligned
/// RTL blocks map consistently into node-local `[0 .. text_size.x]` coordinates.
pub(crate) fn parley_layout_origin_x(computed: &ComputedTextBlock) -> f32 {
    let mut min_offset = f32::INFINITY;
    for line in computed.buffer().lines() {
        min_offset = min_offset.min(line.metrics().offset);
    }
    if min_offset.is_finite() {
        min_offset.max(0.0)
    } else {
        0.0
    }
}

/// Resolves the UTF-8 byte index in the laid-out text nearest to `local_point`.
pub fn text_byte_index_from_local_point(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    local_point: Vec2,
) -> usize {
    let text = selectable_effective_text(label, layout_cache);
    if text.is_empty() {
        return 0;
    }

    let parley_pt = local_logical_to_parley(label, node, computed_size, layout_cache, local_point);
    let origin_x = parley_layout_origin_x(computed);
    let cursor = Cursor::from_point(computed.buffer(), parley_pt.x + origin_x, parley_pt.y);
    snap_byte_index(text.as_ref(), cursor.index(), false)
}

/// Resolves the UTF-8 byte range `(start, end)` of the word under `local_point`.
pub fn text_word_range_from_local_point(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    local_point: Vec2,
) -> (usize, usize) {
    let text = selectable_effective_text(label, layout_cache);
    if text.is_empty() {
        return (0, 0);
    }

    let parley_pt = local_logical_to_parley(label, node, computed_size, layout_cache, local_point);
    let origin_x = parley_layout_origin_x(computed);
    let range =
        Selection::word_from_point(computed.buffer(), parley_pt.x + origin_x, parley_pt.y)
            .text_range();
    let start = snap_byte_index(text.as_ref(), range.start, false);
    let end = snap_byte_index(text.as_ref(), range.end, true);
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

fn parley_bbox_to_local_rect(
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    text_size: Vec2,
    horizontal_offset: f32,
    origin_x: f32,
) -> UTextSelectionRect {
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    let px = (min_x + max_x) * 0.5 - origin_x;
    let py = (min_y + max_y) * 0.5;
    UTextSelectionRect {
        center: Vec2::new(
            horizontal_offset + px - text_size.x * 0.5,
            text_size.y * 0.5 - py,
        ),
        size: Vec2::new(width, height),
    }
}

/// Computes the visual caret rectangle for `byte_index` in node-local logical coordinates (`+Y` up, center-origin).
pub fn text_caret_rect_from_byte_index(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    byte_index: usize,
    caret_width: f32,
) -> UTextSelectionRect {
    let caret_w = caret_width.max(1.0);
    let fallback_h = (label.font_size * 1.2).max(1.0);
    let text = selectable_effective_text(label, layout_cache);
    let text_size = layout_cache.measured_size;
    let horizontal_offset = text_horizontal_offset(label, node, computed_size, text_size);

    if text.is_empty() || computed.buffer().is_empty() {
        return UTextSelectionRect {
            center: Vec2::new(horizontal_offset - text_size.x * 0.5 + caret_w * 0.5, 0.0),
            size: Vec2::new(caret_w, fallback_h),
        };
    }

    let snapped = snap_byte_index(text.as_ref(), byte_index, false);
    let layout = computed.buffer();
    let cursor = Cursor::from_byte_index(layout, snapped, Affinity::Downstream);
    let bbox = cursor.geometry(layout, caret_w);
    let mut rect = parley_bbox_to_local_rect(
        bbox.x0 as f32,
        bbox.x1 as f32,
        bbox.y0 as f32,
        bbox.y1 as f32,
        text_size,
        horizontal_offset,
        parley_layout_origin_x(computed),
    );
    if rect.size.x <= 0.0 {
        rect.size.x = caret_w;
    }
    if rect.size.y <= 0.0 {
        rect.size.y = fallback_h;
    }
    rect
}

/// Moves a caret UTF-8 byte index vertically by `delta_lines` visual lines (`-1` up, `+1` down).
pub fn text_move_caret_vertical(
    label: &UTextLabel,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    byte_index: usize,
    delta_lines: isize,
) -> usize {
    let text = selectable_effective_text(label, layout_cache);
    if text.is_empty() || computed.buffer().is_empty() {
        return 0;
    }
    let snapped = snap_byte_index(text.as_ref(), byte_index, false);
    let layout = computed.buffer();
    let moved = Selection::from_byte_index(layout, snapped, Affinity::Downstream)
        .move_lines(layout, delta_lines, false);
    snap_byte_index(text.as_ref(), moved.focus().index(), false)
}

/// Moves a caret UTF-8 byte index to the visual start (`to_end = false`) or end (`to_end = true`) of its current line.
pub fn text_move_caret_line_edge(
    label: &UTextLabel,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    byte_index: usize,
    to_end: bool,
) -> usize {
    let text = selectable_effective_text(label, layout_cache);
    if text.is_empty() || computed.buffer().is_empty() {
        return 0;
    }
    let snapped = snap_byte_index(text.as_ref(), byte_index, false);
    let layout = computed.buffer();
    let sel = Selection::from_byte_index(layout, snapped, Affinity::Downstream);
    let moved = if to_end {
        sel.line_end(layout, false)
    } else {
        sel.line_start(layout, false)
    };
    snap_byte_index(text.as_ref(), moved.focus().index(), false)
}

/// Computes clipped node-local logical selection rectangles for `selection`.
pub fn compute_text_selection_rects(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    selection: (usize, usize),
) -> Vec<UTextSelectionRect> {
    compute_text_selection_quads(
        label,
        node,
        computed_size,
        layout_cache,
        computed,
        selection,
    )
    .into_iter()
    .map(|quad| UTextSelectionRect {
        center: quad.center,
        size: quad.size,
    })
    .collect()
}

pub(crate) fn compute_text_selection_quads(
    label: &UTextLabel,
    node: &UNode,
    computed_size: &ComputedSize,
    layout_cache: &UTextLabelLayoutCache,
    computed: &ComputedTextBlock,
    selection: (usize, usize),
) -> Vec<TextGlyphQuad> {
    let text = selectable_effective_text(label, layout_cache);
    if text.is_empty() || layout_cache.measured_size.x <= 0.0 || layout_cache.measured_size.y <= 0.0
    {
        return Vec::new();
    }

    let (raw_start, raw_end) = if selection.0 <= selection.1 {
        selection
    } else {
        (selection.1, selection.0)
    };
    let start = snap_byte_index(text.as_ref(), raw_start, false);
    let end = snap_byte_index(text.as_ref(), raw_end, true);
    if start >= end {
        return Vec::new();
    }

    let layout = computed.buffer();
    let anchor = Cursor::from_byte_index(layout, start, Affinity::Downstream);
    let focus = Cursor::from_byte_index(layout, end, Affinity::Upstream);
    let parley_selection = Selection::new(anchor, focus);

    let text_size = layout_cache.measured_size;
    let horizontal_offset = text_horizontal_offset(label, node, computed_size, text_size);
    let content_clip = label_content_clip_rect(label, node, computed_size);

    let origin_x = parley_layout_origin_x(computed);
    let mut quads = Vec::new();
    for (bbox, _line_index) in parley_selection.geometry(layout) {
        let rect = parley_bbox_to_local_rect(
            bbox.x0 as f32,
            bbox.x1 as f32,
            bbox.y0 as f32,
            bbox.y1 as f32,
            text_size,
            horizontal_offset,
            origin_x,
        );
        if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
            continue;
        }

        let quad = TextGlyphQuad {
            center: rect.center,
            size: rect.size,
            uv_min: Vec2::splat(0.25),
            uv_max: Vec2::splat(0.75),
            color: LinearRgba::WHITE,
        };

        if let Some(clip_rect) = content_clip {
            if let Some(clipped) = clip_quad_to_rect(quad, clip_rect) {
                quads.push(clipped);
            }
        } else {
            quads.push(quad);
        }
    }

    quads
}

/// Extracts clipboard-safe selected text (clamped to UTF-8 boundaries, stripping Bidi isolate markers).
pub fn selectable_text_extracted_copy(
    label: &UTextLabel,
    layout_cache: &UTextLabelLayoutCache,
    selectable: &USelectableText,
) -> Option<String> {
    if !selectable.enabled {
        return None;
    }
    let (raw_start, raw_end) = selectable.selection?;
    let text = selectable_effective_text(label, layout_cache);
    let (s, e) = if raw_start <= raw_end {
        (raw_start, raw_end)
    } else {
        (raw_end, raw_start)
    };
    let start = snap_byte_index(text.as_ref(), s, false);
    let end = snap_byte_index(text.as_ref(), e, true);
    if start >= end {
        return None;
    }

    let cleaned: String = text[start..end]
        .chars()
        .filter(|&c| c != LTR_ISOLATE_START && c != RTL_ISOLATE_START && c != ISOLATE_END)
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::text::model::{TextSelectionHighlightMarker, UTextOverflow, UTextWrapMode};
    use bevy::text::{
        FontCx, FontSize, FontSource, LayoutCx, LetterSpacing, LineHeight, RemSize, TextBounds,
        TextElement, TextFont, TextLayout, TextPipeline,
    };

    fn layout_test_block(
        text: &str,
        label: &UTextLabel,
        bounds: TextBounds,
    ) -> (ComputedTextBlock, UTextLabelLayoutCache) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
        ));
        let font_handle = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!("../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf")
                    .to_vec(),
            ));
        // Process AssetEvent<Font> so TextPlugin registers the font alias in FontCx.
        app.update();

        let text_font = TextFont {
            font: FontSource::Handle(font_handle),
            font_size: FontSize::Px(label.font_size),
            ..default()
        };
        let text_layout = TextLayout::new(label.justify, label.linebreak);
        let mut computed = ComputedTextBlock::default();

        let world = app.world_mut();
        let cache = world.resource_scope(|world, fonts: Mut<Assets<Font>>| {
            world.resource_scope(|world, mut text_pipeline: Mut<TextPipeline>| {
                world.resource_scope(|world, mut font_cx: Mut<FontCx>| {
                    world.resource_scope(|_, mut layout_cx: Mut<LayoutCx>| {
                        let mut measure = text_pipeline
                            .create_text_measure(
                                Entity::PLACEHOLDER,
                                &fonts,
                                std::iter::once((
                                    Entity::PLACEHOLDER,
                                    0,
                                    TextElement::Text {
                                        text,
                                        font: &text_font,
                                        color: label.color,
                                        line_height: LineHeight::default(),
                                        letter_spacing: LetterSpacing::default(),
                                    },
                                )),
                                1.0,
                                &text_layout,
                                &mut computed,
                                &mut font_cx,
                                &mut layout_cx,
                                Vec2::ZERO,
                                RemSize(20.0),
                            )
                            .unwrap();
                        let size = measure.compute_size(bounds, &mut computed, &mut font_cx);
                        UTextLabelLayoutCache {
                            measured_size: if label.linebreak == LineBreak::NoWrap {
                                measure.max
                            } else {
                                size
                            },
                            min_content_size: measure.max,
                            max_content_size: measure.max,
                            displayed_text: text.to_string(),
                            line_count: computed.buffer().lines().count(),
                            dirty: false,
                            ..default()
                        }
                    })
                })
            })
        });
        (computed, cache)
    }

    #[test]
    fn coordinate_conversion_roundtrip_respects_justify() {
        let label = UTextLabel {
            text: "Hello".into(),
            justify: Justify::Left,
            ..default()
        };
        let node = UNode::default();
        let computed_size = ComputedSize {
            width: 200.0,
            height: 40.0,
            ..default()
        };
        let cache = UTextLabelLayoutCache {
            measured_size: Vec2::new(60.0, 20.0),
            ..default()
        };

        let left_edge_local = Vec2::new(-100.0, 10.0);
        let parley = local_logical_to_parley(&label, &node, &computed_size, &cache, left_edge_local);
        assert!((parley.x - 0.0).abs() < 1e-4);
        assert!((parley.y - 0.0).abs() < 1e-4);

        let center_label = UTextLabel {
            justify: Justify::Center,
            ..label
        };
        let center_parley =
            local_logical_to_parley(&center_label, &node, &computed_size, &cache, Vec2::ZERO);
        assert!((center_parley.x - 30.0).abs() < 1e-4);
        assert!((center_parley.y - 10.0).abs() < 1e-4);
    }

    #[test]
    fn selection_quads_and_word_picking_work_on_multiline_text() {
        let text = "Hello world\nSecond line";
        let label = UTextLabel {
            text: text.into(),
            font_size: 20.0,
            justify: Justify::Left,
            linebreak: LineBreak::WordBoundary,
            wrap_mode: UTextWrapMode::WrapToParent,
            overflow: UTextOverflow::Clip,
            ..default()
        };
        let node = UNode::default();
        let (computed, cache) = layout_test_block(text, &label, TextBounds::new(300.0, 200.0));
        let computed_size = ComputedSize {
            width: 300.0,
            height: cache.measured_size.y,
            ..default()
        };

        let quads =
            compute_text_selection_rects(&label, &node, &computed_size, &cache, &computed, (0, text.len()));
        assert!(
            quads.len() >= 2,
            "expected multiline selection quads across >=2 lines, got {}",
            quads.len()
        );
        assert!(
            quads[0].center.y > quads[1].center.y,
            "line 0 must sit above line 1 in +Y-up node space"
        );

        // Click inside the second word ("world") on the first line.
        let word_rects =
            compute_text_selection_rects(&label, &node, &computed_size, &cache, &computed, (6, 11));
        assert_eq!(word_rects.len(), 1);
        let word_range = text_word_range_from_local_point(
            &label,
            &node,
            &computed_size,
            &cache,
            &computed,
            word_rects[0].center,
        );
        assert_eq!(&text[word_range.0..word_range.1], "world");

        let hit_idx = text_byte_index_from_local_point(
            &label,
            &node,
            &computed_size,
            &cache,
            &computed,
            word_rects[0].center,
        );
        assert!(
            (6..=11).contains(&hit_idx),
            "expected cursor inside 'world', got {hit_idx}"
        );
    }

    #[test]
    fn selection_quads_clip_against_node_bounds() {
        let text = "Very long selectable text";
        let label = UTextLabel {
            text: text.into(),
            font_size: 20.0,
            justify: Justify::Left,
            overflow: UTextOverflow::Clip,
            ..default()
        };
        let node = UNode::default();
        let (computed, cache) = layout_test_block(text, &label, TextBounds::UNBOUNDED);
        let narrow_size = ComputedSize {
            width: 40.0,
            height: 24.0,
            ..default()
        };

        let quads =
            compute_text_selection_rects(&label, &node, &narrow_size, &cache, &computed, (0, text.len()));
        assert_eq!(quads.len(), 1);
        assert!(quads[0].size.x <= 40.0 + 1e-3);
        assert!(quads[0].size.y <= 24.0 + 1e-3);
    }

    #[test]
    fn extracted_copy_handles_utf8_and_strips_bidi_isolates() {
        let raw = format!("{LTR_ISOLATE_START}مرحبا world{ISOLATE_END}");
        let label = UTextLabel {
            text: raw.clone(),
            ..default()
        };
        let cache = UTextLabelLayoutCache {
            displayed_text: raw.clone(),
            ..default()
        };

        // Intentionally pick byte indices landing mid-UTF-8 codepoint in "مرحبا".
        let selectable = USelectableText::new().with_selection(1, raw.len());
        let extracted = selectable_text_extracted_copy(&label, &cache, &selectable).unwrap();
        assert!(!extracted.contains(LTR_ISOLATE_START));
        assert!(!extracted.contains(ISOLATE_END));
        assert!(extracted.ends_with("world"));

        let mut disabled = selectable.with_selection(0, raw.len());
        disabled.enabled = false;
        assert_eq!(
            selectable_text_extracted_copy(&label, &cache, &disabled),
            None
        );
    }

    #[test]
    fn sync_selectable_text_highlights_spawns_behind_glyphs_and_cleans_up() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            crate::LumaEnginePlugin,
        ));
        app.init_asset::<Mesh>();
        app.init_asset::<Image>();
        app.init_asset::<TextureAtlasLayout>();
        app.init_asset::<bevy::sprite_render::ColorMaterial>();
        app.init_asset::<crate::layout::render::material::UNodeMaterial>();
        app.init_asset::<crate::layout::render::material_3d::UNodeMaterial3d>();
        app.init_asset::<crate::layout::text::render::UTextLabelSdfMaterial>();
        app.init_asset::<crate::layout::text::render::UTextLabelSdfMaterial3d>();
        app.world_mut()
            .spawn((bevy::window::Window::default(), bevy::window::PrimaryWindow));

        let font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!("../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf")
                    .to_vec(),
            ));

        let root = app
            .world_mut()
            .spawn((
                crate::layout::layout_system::URootUi::screen(),
                UNode {
                    width: crate::layout::geometry::UVal::Px(320.0),
                    height: crate::layout::geometry::UVal::Px(120.0),
                    ..default()
                },
            ))
            .id();

        let label_entity = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UTextLabel {
                    text: "Selectable Line".into(),
                    font_size: 20.0,
                    font,
                    ..default()
                },
                USelectableText::new().with_selection(0, 10),
            ))
            .id();

        for _ in 0..3 {
            app.update();
        }

        let children: Vec<Entity> = app
            .world()
            .get::<Children>(label_entity)
            .expect("label should have children")
            .iter()
            .collect();
        let highlight = children
            .iter()
            .copied()
            .find(|&e| app.world().entity(e).contains::<TextSelectionHighlightMarker>())
            .expect("should spawn TextSelectionHighlightMarker child");
        let text_child = children
            .iter()
            .copied()
            .find(|&e| app.world().entity(e).contains::<crate::layout::text::model::TextChildMarker>())
            .expect("should spawn TextChildMarker child");

        let highlight_z = app.world().get::<Transform>(highlight).unwrap().translation.z;
        let text_z = app.world().get::<Transform>(text_child).unwrap().translation.z;
        assert!(highlight_z > 0.0 && highlight_z < text_z);

        // Clearing selection should despawn the highlight child cleanly.
        app.world_mut()
            .get_mut::<USelectableText>(label_entity)
            .unwrap()
            .clear_selection();
        app.update();
        assert!(app.world().get_entity(highlight).is_err());
    }

    #[test]
    fn parley_caret_geometry_and_vertical_navigation_on_multiline_text() {
        let raw = "first line\nsecond line\n";
        let label = UTextLabel {
            text: raw.into(),
            font_size: 20.0,
            wrap_mode: UTextWrapMode::WrapToParent,
            linebreak: bevy::text::LineBreak::WordBoundary,
            overflow: UTextOverflow::Clip,
            ..default()
        };
        let (computed, cache) = layout_test_block(
            raw,
            &label,
            TextBounds {
                width: Some(300.0),
                height: None,
            },
        );
        let node = UNode::default();
        let computed_size = ComputedSize {
            width: cache.measured_size.x,
            height: cache.measured_size.y,
            ..default()
        };

        // End of line 1 vs end of line 2 vs trailing blank line after `\n`
        let r0 = text_caret_rect_from_byte_index(&label, &node, &computed_size, &cache, &computed, 3, 2.0);
        let down = text_move_caret_vertical(&label, &cache, &computed, 3, 1);
        assert!(down > "first line\n".len(), "expected vertical move onto second line, got {down}");
        let r1 = text_caret_rect_from_byte_index(&label, &node, &computed_size, &cache, &computed, down, 2.0);
        assert!(r1.center.y < r0.center.y, "second line must have lower +Y coordinate than first line");

        let back_up = text_move_caret_vertical(&label, &cache, &computed, down, -1);
        assert!(back_up <= "first line".len());

        // Trailing newline should place caret on the empty 3rd line below line 2.
        let r_end = text_caret_rect_from_byte_index(
            &label,
            &node,
            &computed_size,
            &cache,
            &computed,
            raw.len(),
            2.0,
        );
        assert!(r_end.center.y < r1.center.y);
    }
}
