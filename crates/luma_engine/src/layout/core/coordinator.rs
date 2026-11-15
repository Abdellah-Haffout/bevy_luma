//! Internal numeric tree coordinator (`pub(crate)`) for layout and geometry settlement.
//!
//! By design, any capability that mutates an element's size ([`ComputedSize`]) or position
//! ([`Transform`]) is strictly governed by this internal coordinator API rather than public
//! external plugins. This preserves layout determinism while keeping the internal solver modular:
//!
//! 1. **Base Stage (`Base`)**: Container algorithms (`Flex`, `Grid`, `Stack`, `Masonry`, `Radial`, `Absolute`)
//!    propose initial `(base_size, base_pos)` for children.
//! 2. **Constraint Stage (`Clamp`)**: Min/Max bounds and aspect-ratio rules clamp the proposed size.
//! 3. **Offset Stage (`Delta`)**: Additive offsets (`Relative` offsets, `UScrollContent`, `UScrollbarThumb`)
//!    accumulate without overwriting base coordinates.
//! 4. **Temporal Filter Stage (`Filter`)**: Spring transitions (`UTransitionState`) filter the final
//!    target translation before committing to [`Transform`].

use bevy::prelude::*;

use crate::layout::components::{IntrinsicSize, LayoutDepth, UiLocalStacking};
use crate::layout::core::layout_cache::LayoutCache;
use crate::layout::core::pass_up::{
    MeasurePassScratch, measure_flex_intrinsic_with_hint, measure_grid_intrinsic_with_hint,
    measure_stack_intrinsic,
};
use crate::layout::core::solver::{GridScratch, SolverConfig, SolverItem, solve_flex_layout, solve_grid_layout};
use crate::layout::geometry::{BoxConstraints, ComputedSize, USides, UVal};
use crate::layout::layout_system::ResolvedRootStack;
use crate::layout::scroll::{UScrollContent, UScrollbarThumb};
use crate::layout::transition::{UTransition, UTransitionState};
use crate::layout::univis_node::{UDisplay, ULayout, UNode, USelf};

const LAYOUT_WRITE_EPSILON: f32 = 0.001;

fn z_translation_changed(current: f32, next: f32) -> bool {
    current.to_bits() != next.to_bits()
}

/// Strength tier for internal numeric proposals when two rules target the same base slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProposalStrength(pub u16);

#[allow(dead_code)]
impl ProposalStrength {
    pub const SUGGESTED: Self = Self(250);
    pub const NORMAL: Self = Self(500);
    pub const OVERRIDE: Self = Self(800);
    pub const REQUIRED: Self = Self(1000);
}

/// Numeric operation category understood by [`TreeCoordinator`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) enum ProposalOp {
    /// Establishes the base value (`pos` or `size`).
    Base {
        value: Vec2,
        strength: ProposalStrength,
        tick: u32,
    },
    /// Clamps the current value within `[min, max]`.
    Clamp { min: Vec2, max: Vec2 },
    /// Accumulates an additive offset (`+dx`, `+dy`).
    Delta(Vec2),
}

/// Coordinated numeric state for a single child node produced during the downward pass.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) struct CoordinatedChildState {
    pub entity: Entity,
    pub base_size: Vec2,
    pub base_pos: Vec2,
    pub final_size: Vec2,
    pub world_pos: Vec2,
}

/// Internal coordinator that arbitrates numeric layout proposals and applies them to the scene tree.
///
/// Exposed only as `pub(crate)` so external crates cannot inject conflicting layout solvers.
pub(crate) struct TreeCoordinator;

impl TreeCoordinator {
    /// Resolves two competing [`ProposalOp::Base`] values deterministically using `ProposalStrength`
    /// and falling back to the most recent `tick` (Last-Write-Wins).
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn arbitrate_base(
        left: (Vec2, ProposalStrength, u32),
        right: (Vec2, ProposalStrength, u32),
    ) -> Vec2 {
        match left.1.cmp(&right.1) {
            core::cmp::Ordering::Greater => left.0,
            core::cmp::Ordering::Less => right.0,
            core::cmp::Ordering::Equal => {
                if right.2 >= left.2 {
                    right.0
                } else {
                    left.0
                }
            }
        }
    }

    /// Proposes upward (bottom-up) content intrinsic bounds `(min_w, max_w, min_h, max_h)`
    /// for a container from its measured children.
    #[inline]
    pub(crate) fn propose_upward_intrinsic(
        scratch: &mut MeasurePassScratch,
        layout: &ULayout,
        width: UVal,
        padding: USides,
        available_main_hint: Option<f32>,
    ) -> (f32, f32, f32, f32) {
        match layout.display {
            UDisplay::Grid => measure_grid_intrinsic_with_hint(
                scratch,
                layout,
                width,
                padding,
                available_main_hint,
            ),
            UDisplay::Stack => measure_stack_intrinsic(&scratch.child_items),
            _ => measure_flex_intrinsic_with_hint(
                &scratch.child_items,
                layout,
                available_main_hint,
            ),
        }
    }

    /// Resolves the upward (bottom-up) [`IntrinsicSize`] for a node by combining `Base`
    /// (explicit `Px` or child content + padding), aspect-ratio derivation, and `Clamp` bounds.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_upward_intrinsic(
        width: UVal,
        height: UVal,
        padding: USides,
        width_bounds: (f32, f32),
        height_bounds: (f32, f32),
        aspect_ratio: Option<f32>,
        calculated_min_width: f32,
        calculated_max_width: f32,
        calculated_min_height: f32,
        calculated_max_height: f32,
    ) -> IntrinsicSize {
        let h_pad = padding.width_sum();
        let v_pad = padding.height_sum();

        let mut raw_min_width = match width {
            UVal::Px(v) => v,
            _ => calculated_min_width + h_pad,
        };
        let mut raw_max_width = match width {
            UVal::Px(v) => v,
            _ => calculated_max_width + h_pad,
        };
        let mut raw_min_height = match height {
            UVal::Px(v) => v,
            _ => calculated_min_height + v_pad,
        };
        let mut raw_max_height = match height {
            UVal::Px(v) => v,
            _ => calculated_max_height + v_pad,
        };

        if let Some(ratio) = aspect_ratio {
            let w_is_px = matches!(width, UVal::Px(_));
            let h_is_px = matches!(height, UVal::Px(_));

            if w_is_px && !h_is_px {
                raw_min_height = raw_min_width / ratio;
                raw_max_height = raw_max_width / ratio;
            } else if h_is_px && !w_is_px {
                raw_min_width = raw_min_height * ratio;
                raw_max_width = raw_max_height * ratio;
            } else if !w_is_px && !h_is_px {
                if raw_max_width > h_pad && raw_max_height <= v_pad {
                    raw_min_height = v_pad + (raw_min_width - h_pad).max(0.0) / ratio;
                    raw_max_height = v_pad + (raw_max_width - h_pad).max(0.0) / ratio;
                } else if raw_max_height > v_pad && raw_max_width <= h_pad {
                    raw_min_width = h_pad + (raw_min_height - v_pad).max(0.0) * ratio;
                    raw_max_width = h_pad + (raw_max_height - v_pad).max(0.0) * ratio;
                }
            }
        }

        let min_width = raw_min_width.clamp(width_bounds.0, width_bounds.1);
        let max_width = raw_max_width
            .clamp(width_bounds.0, width_bounds.1)
            .max(min_width);
        let min_height = raw_min_height.clamp(height_bounds.0, height_bounds.1);
        let max_height = raw_max_height
            .clamp(height_bounds.0, height_bounds.1)
            .max(min_height);

        let mut new_width = match width {
            UVal::MinContent => min_width,
            _ => max_width,
        };
        let mut new_height = match height {
            UVal::MinContent => min_height,
            _ => max_height,
        };

        if let Some(ratio) = aspect_ratio {
            let w_is_px = matches!(width, UVal::Px(_));
            let h_is_px = matches!(height, UVal::Px(_));
            if w_is_px && !h_is_px {
                new_height = (new_width / ratio).clamp(min_height, max_height);
            } else if h_is_px && !w_is_px {
                new_width = (new_height * ratio).clamp(min_width, max_width);
            }
        }

        IntrinsicSize {
            width: new_width,
            height: new_height,
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Runs the container distribution proposer (`Flex`, `Grid`, `Stack`, `Masonry`, `Radial`)
    /// to populate each child's `Base` size and position proposals.
    #[inline]
    pub(crate) fn propose_container_distribution(
        config: &SolverConfig,
        constraints: BoxConstraints,
        items: &mut [SolverItem],
        grid_scratch: &mut GridScratch,
    ) -> Vec2 {
        if config.display == UDisplay::Grid {
            solve_grid_layout(config, constraints, items, Some(grid_scratch))
        } else {
            solve_flex_layout(config, constraints, items)
        }
    }

    /// Coordinates the final numeric state (`Base` -> `Clamp` -> `World Transform` -> `Delta/Filter`)
    /// for a solved child and commits the result to the scene tree components.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(crate) fn apply_coordinated_child(
        entity: Entity,
        solved_item: &SolverItem,
        parent_size: Vec2,
        solve_generation: u64,
        world_scale: f32,
        root_stack: ResolvedRootStack,
        parent_target_z: f32,
        use_incremental_solve: bool,
        cache: &mut LayoutCache,
        nodes_query: &mut Query<(
            Entity,
            &UNode,
            Option<&ULayout>,
            &LayoutDepth,
            Option<&Children>,
            Option<&USelf>,
            Option<&crate::layout::components::CachedUiContext>,
            Option<&UiLocalStacking>,
            &mut ComputedSize,
            &mut Transform,
            Option<&mut Visibility>,
            Option<&UTransition>,
            Option<&mut UTransitionState>,
        )>,
        scroll_contents: &mut Query<&mut UScrollContent>,
        scrollbar_thumbs: &mut Query<&mut UScrollbarThumb>,
    ) {
        let Ok((
            _,
            node,
            _,
            layout_depth,
            children,
            uself,
            _,
            local_stacking,
            mut computed,
            mut transform,
            mut vis_opt,
            _transition_opt,
            transition_state_opt,
        )) = nodes_query.get_mut(entity)
        else {
            return;
        };

        // 1. Base + Clamp Stage: resolve final logical dimensions
        let (min_w, max_w) = node.width_bounds();
        let (min_h, max_h) = node.height_bounds();
        let child_w = solved_item.result.size.x.clamp(min_w, max_w);
        let child_h = solved_item.result.size.y.clamp(min_h, max_h);

        let size_changed = (computed.width - child_w).abs() > LAYOUT_WRITE_EPSILON
            || (computed.height - child_h).abs() > LAYOUT_WRITE_EPSILON;

        if size_changed {
            computed.width = child_w;
            computed.height = child_h;
        }

        // 2. Coordinate conversion from top-left logical space to parent-centered world space
        let next_x =
            ((-parent_size.x / 2.0) + solved_item.result.pos.x + (child_w / 2.0)) * world_scale;
        let next_y =
            ((parent_size.y / 2.0) - solved_item.result.pos.y - (child_h / 2.0)) * world_scale;

        #[allow(deprecated)]
        let order = uself.map(|value| value.order).unwrap_or(0);
        let child_target_z = local_stacking.map_or_else(
            || root_stack.local_depth_offset(layout_depth.0, order),
            |stack| root_stack.local_depth_offset_for_fraction(stack.normalized),
        );
        let next_z = if (child_target_z - parent_target_z).abs() > f32::EPSILON {
            child_target_z - parent_target_z
        } else {
            root_stack.capsule_band_step
        };

        // 3. Delta & Temporal Filter Stage (Scroll, Thumb, and Transition modifiers)
        let scroll_content_opt = scroll_contents.get_mut(entity).ok();
        let scrollbar_thumb_opt = scrollbar_thumbs.get_mut(entity).ok();
        if let Some(mut scroll_content) = scroll_content_opt {
            scroll_content.base_translation = Vec2::new(next_x, next_y);
            if !scroll_content.initialized {
                scroll_content.initialized = true;
                if (transform.translation.x - next_x).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.x = next_x;
                }
                if (transform.translation.y - next_y).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.y = next_y;
                }
            }
        } else if let Some(mut thumb) = scrollbar_thumb_opt {
            thumb.base_translation = Vec2::new(next_x, next_y);
            if !thumb.initialized {
                thumb.initialized = true;
                if (transform.translation.x - next_x).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.x = next_x;
                }
                if (transform.translation.y - next_y).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.y = next_y;
                }
            }
        } else if let Some(mut state) = transition_state_opt {
            if !state.initialized {
                state.target_translation = Vec2::new(next_x, next_y);
                state.velocity = Vec2::ZERO;
                state.initialized = true;
                if (transform.translation.x - next_x).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.x = next_x;
                }
                if (transform.translation.y - next_y).abs() > LAYOUT_WRITE_EPSILON {
                    transform.translation.y = next_y;
                }
            } else {
                state.target_translation = Vec2::new(next_x, next_y);
            }
        } else {
            if (transform.translation.x - next_x).abs() > LAYOUT_WRITE_EPSILON {
                transform.translation.x = next_x;
            }
            if (transform.translation.y - next_y).abs() > LAYOUT_WRITE_EPSILON {
                transform.translation.y = next_y;
            }
        }

        if z_translation_changed(transform.translation.z, next_z) {
            transform.translation.z = next_z;
        }
        if transform.scale == Vec3::ZERO {
            transform.scale = Vec3::ONE;
            if let Some(ref mut vis) = vis_opt
                && **vis == Visibility::Hidden
            {
                **vis = Visibility::Inherited;
            }
        }

        if use_incremental_solve
            && size_changed
            && children.is_some_and(|value| !value.is_empty())
        {
            cache.mark_solve_dirty_generation(entity, solve_generation);
        }

        if children.is_none_or(|value| value.is_empty()) {
            cache.complete_solve(entity);
        }
    }
}
