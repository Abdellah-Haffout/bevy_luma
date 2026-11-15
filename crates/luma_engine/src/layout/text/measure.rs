pub(crate) mod bidi;
pub(super) mod bounds;
pub(super) mod invalidation;
mod measurement;
#[cfg(test)]
mod tests;
mod truncation;

use crate::layout::geometry::{USides, UVal};
#[cfg(test)]
use crate::layout::layout_system::URootUi;
use crate::layout::query::{ComputedSize, IntrinsicSize};
#[cfg(test)]
use crate::layout::univis_node::ULayout;
use crate::layout::univis_node::UNode;
use crate::schedule::UiWorkState;
#[cfg(test)]
use crate::schedule::{UiSettlementSchedule, UnivisPostUpdateSet};
use bevy::asset::{AssetEvent, Assets};
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use bevy::text::{
    ComputedTextBlock, FontCx, FontSource, LayoutCx, LetterSpacing, LineBreak, LineHeight, RemSize,
    TextBounds, TextElement, TextFont, TextLayout, TextPipeline,
};
use std::collections::{HashMap, HashSet};
use unicode_bidi::BidiInfo;
use unicode_segmentation::UnicodeSegmentation;

use self::bounds::{
    clamp_outer_size_to_bounds, desired_text_label_intrinsic_size, label_measure_bounds,
    measured_text_outer_size, parent_label_bounds, parent_label_bounds_from_ids,
};
#[cfg(test)]
use self::invalidation::mark_text_label_layout_dirty;
use self::invalidation::{
    next_text_label_layout_cache, parent_bounds_changed,
    reset_text_label_layout_cache_preserving_bounds,
};
use self::measurement::{
    TextMeasureStage, label_text_font, label_text_layout, log_text_measure_error,
    measure_layout_for_label, measure_layout_for_text, resolve_final_measured_text,
};
use super::model::{
    TextFallbackFonts, UTextLabel, UTextLabelLayoutCache, UTextOverflow, UTextTruncateSide,
    UTextWrapMode,
};

pub fn measure_text_label_layout(
    font_events: Option<MessageReader<AssetEvent<Font>>>,
    fonts: Option<Res<Assets<Font>>>,
    global_fallbacks: Option<Res<TextFallbackFonts>>,
    mut known_ready_fonts: Local<HashSet<AssetId<Font>>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    parents_query: Query<&ChildOf>,
    parent_query: Query<(&UNode, &ComputedSize)>,
    mut query: Query<(
        Entity,
        Ref<UTextLabel>,
        Ref<UNode>,
        Ref<ComputedSize>,
        &mut ComputedTextBlock,
        &mut UTextLabelLayoutCache,
    )>,
) {
    let Some(fonts) = fonts else {
        return;
    };

    let mut changed_fonts = HashSet::new();
    if let Some(mut events) = font_events {
        for event in events.read() {
            match *event {
                AssetEvent::Added { id }
                | AssetEvent::Modified { id }
                | AssetEvent::LoadedWithDependencies { id }
                | AssetEvent::Removed { id }
                | AssetEvent::Unused { id } => {
                    changed_fonts.insert(id);
                }
            }
        }
    }

    // Bevy's TextPlugin populates `font.alias` via untracked mutation when adding fonts to
    // `FontCx`; also re-measure any label whose primary/fallback font becomes ready this frame.
    for (id, font) in fonts.iter() {
        if !font.alias.is_empty() && known_ready_fonts.insert(id) {
            changed_fonts.insert(id);
        }
    }
    known_ready_fonts.retain(|id| fonts.contains(*id));

    let fallbacks_slice: &[Handle<Font>] = global_fallbacks
        .as_ref()
        .map_or(&[], |fb| fb.0.as_slice());
    let fallbacks_res_changed = global_fallbacks
        .as_ref()
        .is_some_and(|fb| fb.is_changed() || fallbacks_slice.iter().any(|h| changed_fonts.contains(&h.id())));

    for (entity, label, node, computed_size, mut computed, mut cache) in query.iter_mut() {
        let font_changed = fallbacks_res_changed
            || changed_fonts.contains(&label.font.id())
            || label
                .fallback_fonts
                .iter()
                .any(|f| changed_fonts.contains(&f.id()))
            || label.spans.iter().any(|s| {
                s.font
                    .as_ref()
                    .map_or(false, |f| changed_fonts.contains(&f.id()))
            });
        let parent_bounds =
            parent_label_bounds(entity, &label, &node, &parents_query, &parent_query);
        let bounds_changed = parent_bounds_changed(&cache, parent_bounds);

        if !(label.is_changed()
            || node.is_changed()
            || computed_size.is_changed()
            || cache.dirty
            || bounds_changed
            || font_changed)
        {
            continue;
        }

        let text_font = label_text_font(&label, fallbacks_slice, &fonts);
        let text_layout = label_text_layout(&label);
        let text_color = label.color;
        let bounds = label_measure_bounds(&label, &node, Some(&computed_size), parent_bounds);

        match measure_layout_for_label(
            entity,
            &label,
            TextMeasureStage::FullText,
            &text_font,
            fallbacks_slice,
            &text_layout,
            text_color,
            bounds,
            &fonts,
            &mut text_pipeline,
            &mut computed,
            &mut font_cx,
            &mut layout_cx,
        ) {
            Ok(measured) => {
                let Ok((final_text, final_measured, overflowed)) = resolve_final_measured_text(
                    entity,
                    &label,
                    &text_font,
                    &text_layout,
                    text_color,
                    bounds,
                    &fonts,
                    &mut text_pipeline,
                    &mut computed,
                    &mut font_cx,
                    &mut layout_cx,
                    measured,
                ) else {
                    reset_text_label_layout_cache_preserving_bounds(&mut cache, parent_bounds);
                    continue;
                };

                let next_cache = next_text_label_layout_cache(
                    final_text,
                    final_measured,
                    parent_bounds,
                    overflowed,
                );
                if *cache != next_cache {
                    *cache = next_cache;
                }
            }
            Err(error) => {
                log_text_measure_error(error);
                if cache.measured_size == Vec2::ZERO {
                    reset_text_label_layout_cache_preserving_bounds(&mut cache, parent_bounds);
                }
            }
        }
    }
}

pub fn fit_node_to_text_size(
    parents_query: Query<&ChildOf>,
    work_state: Option<Res<UiWorkState>>,
    mut adjusted_counts: Local<HashMap<Entity, (u64, u8)>>,
    mut params: ParamSet<(
        Query<(&UNode, &ComputedSize)>,
        Query<(Entity, &UTextLabel, &mut UNode, &UTextLabelLayoutCache)>,
    )>,
) {
    let autosize_snapshot: Vec<_> = {
        let labels = params.p1();
        labels
            .iter()
            .filter_map(|(entity, label, node, _)| {
                label.autosize.then_some((
                    entity,
                    parents_query.get(entity).ok().map(|parent| parent.get()),
                    label.overflow,
                    label.is_wrapping(),
                    node.margin,
                ))
            })
            .collect()
    };

    if autosize_snapshot.is_empty() {
        return;
    }

    let current_gen = work_state.as_ref().map_or(0, |w| w.current_generation());
    if adjusted_counts.len() > 256 {
        adjusted_counts.retain(|_, (generation, _)| *generation == current_gen);
    }

    for (entity, parent_entity, overflow, is_wrapping, margin) in autosize_snapshot {
        let entry = adjusted_counts.entry(entity).or_insert((current_gen, 0));
        if entry.0 != current_gen {
            entry.0 = current_gen;
            entry.1 = 0;
        }
        if entry.1 >= 2 {
            continue;
        }

        let parent_bounds = {
            let parent_bounds_query = params.p0();
            parent_label_bounds_from_ids(
                parent_entity,
                overflow,
                is_wrapping,
                margin,
                &parent_bounds_query,
            )
        };

        let mut labels = params.p1();
        let Ok((_, _, mut node, layout_cache)) = labels.get_mut(entity) else {
            continue;
        };

        let outer_size = measured_text_outer_size(&node, layout_cache);
        let clamped_outer_size = clamp_outer_size_to_bounds(outer_size, parent_bounds);
        let target_width = (clamped_outer_size.x * 2.0).round() / 2.0;
        let target_height = (clamped_outer_size.y * 2.0).round() / 2.0;

        let current_w = match node.width {
            UVal::Px(value) => value,
            _ => -1.0,
        };
        let current_h = match node.height {
            UVal::Px(value) => value,
            _ => -1.0,
        };

        const TEXT_AUTOSIZE_EPSILON: f32 = 0.5;
        let mut changed = false;

        // When text wraps to parent, DO NOT shrink the node's width to the measured line width.
        // Shrinking the width causes oscillation loops and breaks responsive flex layouts.
        if !is_wrapping {
            if !node.width.uses_intrinsic_measurement()
                && (current_w - target_width).abs() > TEXT_AUTOSIZE_EPSILON
            {
                node.width = UVal::Px(target_width);
                changed = true;
            }
        } else if let Some(parent_w) = parent_bounds.width {
            if let UVal::Px(w) = node.width {
                if w > parent_w + TEXT_AUTOSIZE_EPSILON {
                    node.width = UVal::Px(parent_w);
                    changed = true;
                }
            }
        }

        if !node.height.uses_intrinsic_measurement()
            && (current_h - target_height).abs() > TEXT_AUTOSIZE_EPSILON
        {
            node.height = UVal::Px(target_height);
            changed = true;
        }
        if changed {
            entry.1 += 1;
        }
    }
}

pub fn sync_text_label_intrinsic_size(
    parents_query: Query<&ChildOf>,
    parent_bounds_query: Query<(&UNode, &ComputedSize)>,
    mut query: Query<
        (
            Entity,
            &UNode,
            &UTextLabel,
            &UTextLabelLayoutCache,
            &mut IntrinsicSize,
        ),
        Or<(
            Added<UTextLabel>,
            Changed<UTextLabel>,
            Changed<UNode>,
            Changed<UTextLabelLayoutCache>,
        )>,
    >,
) {
    for (entity, node, label, layout_cache, mut intrinsic) in query.iter_mut() {
        if layout_cache.dirty {
            continue;
        }

        let mut target = desired_text_label_intrinsic_size(label, node, layout_cache, *intrinsic);
        if label.autosize {
            let parent_bounds =
                parent_label_bounds(entity, label, node, &parents_query, &parent_bounds_query);
            let clamped_outer_size =
                clamp_outer_size_to_bounds(Vec2::new(target.width, target.height), parent_bounds);
            target.width = clamped_outer_size.x;
            if !label.is_wrapping() || label.overflow != UTextOverflow::Visible {
                target.height = clamped_outer_size.y;
            }
        }
        if (intrinsic.width - target.width).abs() > 0.1
            || (intrinsic.height - target.height).abs() > 0.1
        {
            *intrinsic = target;
        }
    }
}

/// Re-applies single-line ellipsis truncation when `LayoutSolve` shrinks a label's `ComputedSize`
/// narrower than its intrinsic measured width, without mutating `IntrinsicSize` (avoiding loops).
pub(super) fn constrain_text_labels_to_computed_size(
    fonts: Option<Res<Assets<Font>>>,
    global_fallbacks: Option<Res<TextFallbackFonts>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut font_cx: ResMut<FontCx>,
    mut layout_cx: ResMut<LayoutCx>,
    mut removed_labels: RemovedComponents<UTextLabel>,
    mut last_constrained_width: Local<HashMap<Entity, f32>>,
    parents_query: Query<&ChildOf>,
    parent_query: Query<(&UNode, &ComputedSize)>,
    mut query: Query<
        (
            Entity,
            Ref<UTextLabel>,
            Ref<UNode>,
            Ref<ComputedSize>,
            &mut ComputedTextBlock,
            &mut UTextLabelLayoutCache,
        ),
        Or<(Changed<ComputedSize>, Changed<UTextLabelLayoutCache>)>,
    >,
) {
    for entity in removed_labels.read() {
        last_constrained_width.remove(&entity);
    }

    let Some(fonts) = fonts else {
        return;
    };
    let fallbacks_slice: &[Handle<Font>] = global_fallbacks
        .as_ref()
        .map_or(&[], |fb| fb.0.as_slice());

    for (entity, label, node, computed_size, mut computed, mut cache) in query.iter_mut() {
        if cache.dirty
            || label.is_wrapping()
            || label.overflow != UTextOverflow::Ellipsis
            || !label.spans.is_empty()
            || computed_size.width <= 0.5
        {
            last_constrained_width.remove(&entity);
            continue;
        }

        let avail_w = (computed_size.width - node.padding.width_sum()).max(0.0);
        if avail_w <= 0.5 {
            continue;
        }

        let needs_shrink = cache.measured_size.x > avail_w + 0.5;
        let expanded_after_shrink = last_constrained_width
            .get(&entity)
            .is_some_and(|&prev_w| avail_w > prev_w + 0.5 && cache.overflowed);

        if !needs_shrink && !expanded_after_shrink {
            if !cache.overflowed {
                last_constrained_width.remove(&entity);
            }
            continue;
        }

        if let Some(&prev_w) = last_constrained_width.get(&entity)
            && !needs_shrink
            && (avail_w - prev_w).abs() <= 0.5
        {
            continue;
        }

        let parent_bounds =
            parent_label_bounds(entity, &label, &node, &parents_query, &parent_query);
        let mut bounds = label_measure_bounds(&label, &node, Some(&computed_size), parent_bounds);
        bounds.width = Some(bounds.width.map_or(avail_w, |bw| bw.min(avail_w)));
        if label.effective_max_lines().is_none() {
            bounds.height = None;
        }

        let text_font = label_text_font(&label, fallbacks_slice, &fonts);
        let text_layout = label_text_layout(&label);
        let text_color = label.color;

        let Ok(measured) = measure_layout_for_label(
            entity,
            &label,
            TextMeasureStage::FullText,
            &text_font,
            fallbacks_slice,
            &text_layout,
            text_color,
            bounds,
            &fonts,
            &mut text_pipeline,
            &mut computed,
            &mut font_cx,
            &mut layout_cx,
        ) else {
            continue;
        };

        let Ok((final_text, final_measured, overflowed)) = resolve_final_measured_text(
            entity,
            &label,
            &text_font,
            &text_layout,
            text_color,
            bounds,
            &fonts,
            &mut text_pipeline,
            &mut computed,
            &mut font_cx,
            &mut layout_cx,
            measured,
        ) else {
            continue;
        };

        let preserved_min = cache.min_content_size;
        let preserved_max = cache.max_content_size;
        if cache.measured_size != final_measured.size
            || cache.displayed_text != final_text
            || cache.line_count != final_measured.line_count
            || cache.overflowed != overflowed
        {
            cache.measured_size = final_measured.size;
            cache.displayed_text = final_text;
            cache.line_count = final_measured.line_count;
            cache.overflowed = overflowed;
            cache.min_content_size = preserved_min;
            cache.max_content_size = preserved_max;
        }

        if avail_w + 0.5 >= preserved_max.x && !overflowed {
            last_constrained_width.remove(&entity);
        } else {
            last_constrained_width.insert(entity, avail_w);
        }
    }
}
