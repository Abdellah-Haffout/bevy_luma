use crate::layout::UnivisLayoutPlugin;
use crate::layout::components::LayoutDepth;
use crate::layout::core::layout_cache::LayoutCache;
use crate::layout::geometry::UVal;
use crate::layout::layout_system::{URootUi, UiCanvasSize, UiRootSettlementState};
use crate::layout::query::ComputedSize;
use crate::layout::univis_node::{ULayout, UNode, UZIndex};
use crate::schedule::{
    UiSettlementConfig, UiSettlementSchedule, UiWorkStage, UiWorkState, UnivisPostUpdateSet,
};
use bevy::prelude::*;

#[derive(Component)]
struct DeferredResizeTarget;

#[derive(Resource, Default)]
struct DeferredResizeOnce(bool);

fn resize_target_in_render_sync(
    mut resize_once: ResMut<DeferredResizeOnce>,
    mut query: Query<&mut UNode, With<DeferredResizeTarget>>,
) {
    if resize_once.0 {
        return;
    }

    let mut node = query
        .single_mut()
        .expect("test should only spawn one deferred resize target");
    node.width = UVal::Px(120.0);
    resize_once.0 = true;
}

#[test]
fn fresh_ui_tree_settles_on_the_first_frame() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 200.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let child = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(50.0),
                ..default()
            },
        ))
        .id();

    app.update();

    let cache = app.world().resource::<LayoutCache>();
    let root_layer = cache
        .get_entities_at_depth(0)
        .expect("root depth bucket should be built on the first frame");
    let child_layer = cache
        .get_entities_at_depth(1)
        .expect("child depth bucket should be built on the first frame");

    assert!(root_layer.contains(&root));
    assert!(child_layer.contains(&child));
    assert_eq!(
        app.world()
            .entity(child)
            .get::<LayoutDepth>()
            .copied()
            .expect("child should have a depth assigned on the first frame"),
        LayoutDepth(1)
    );

    let child_size = app
        .world()
        .entity(child)
        .get::<ComputedSize>()
        .copied()
        .expect("child should have a computed size on the first frame");
    assert_eq!(child_size.width, 100.0);
    assert_eq!(child_size.height, 50.0);

    let work_state = app.world().resource::<UiWorkState>();
    let root_settlement = app
        .world()
        .entity(root)
        .get::<UiRootSettlementState>()
        .copied()
        .expect("root should expose settlement state after the first frame");
    assert_eq!(work_state.current_generation(), 1);
    assert!(
        work_state.is_settled(),
        "pending={:?}, root_settlement={root_settlement:?}",
        work_state.pending()
    );
    assert_eq!(work_state.last_frame_iterations(), 2);
    assert!(!work_state.budget_exhausted());
    assert_eq!(work_state.completed_generation(UiWorkStage::RootResolve), 1);
    assert_eq!(work_state.completed_generation(UiWorkStage::Hierarchy), 1);
    assert_eq!(work_state.completed_generation(UiWorkStage::Measure), 1);
    assert_eq!(work_state.completed_generation(UiWorkStage::Solve), 1);
    assert_eq!(work_state.completed_generation(UiWorkStage::Render), 1);
    assert_eq!(root_settlement.current_generation, 1);
    assert_eq!(root_settlement.pending_measure, 0);
    assert_eq!(root_settlement.pending_solve, 0);
    assert_eq!(root_settlement.pending_render, 0);
    assert!(root_settlement.is_settled());
}

#[test]
fn settlement_loop_drains_follow_up_layout_work_in_the_same_frame() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));
    app.init_resource::<DeferredResizeOnce>();
    app.add_systems(
        UiSettlementSchedule,
        resize_target_in_render_sync.in_set(UnivisPostUpdateSet::RenderSync),
    );

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 200.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let child = app
        .world_mut()
        .spawn((
            ChildOf(root),
            DeferredResizeTarget,
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(50.0),
                ..default()
            },
        ))
        .id();

    app.update();

    let child_size = app
        .world()
        .entity(child)
        .get::<ComputedSize>()
        .copied()
        .expect("child should have a computed size after settlement drains");
    assert_eq!(child_size.width, 120.0);
    assert_eq!(child_size.height, 50.0);

    let work_state = app.world().resource::<UiWorkState>();
    assert!(work_state.is_settled());
    assert_eq!(work_state.current_generation(), 2);
    assert_eq!(work_state.last_frame_iterations(), 3);
    assert!(!work_state.budget_exhausted());
}

#[test]
fn settlement_budget_exhaustion_prevents_the_frame_from_reporting_idle() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));
    app.insert_resource(UiSettlementConfig { max_iterations: 1 });
    app.init_resource::<DeferredResizeOnce>();
    app.add_systems(
        UiSettlementSchedule,
        resize_target_in_render_sync.in_set(UnivisPostUpdateSet::RenderSync),
    );

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 200.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let child = app
        .world_mut()
        .spawn((
            ChildOf(root),
            DeferredResizeTarget,
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(50.0),
                ..default()
            },
        ))
        .id();

    app.update();

    let child_size = app
        .world()
        .entity(child)
        .get::<ComputedSize>()
        .copied()
        .expect("child should still have a computed size after the first iteration");
    assert_eq!(child_size.width, 100.0);

    let work_state = app.world().resource::<UiWorkState>();
    assert_eq!(work_state.current_generation(), 1);
    assert_eq!(work_state.last_frame_iterations(), 1);
    assert!(work_state.budget_exhausted());
    assert!(!work_state.is_settled());
}

#[test]
fn deeply_nested_layout_hierarchy_settles_without_exhausting_budget() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 400.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let mut current_parent = root;
    for _ in 0..7 {
        let child = app
            .world_mut()
            .spawn((
                ChildOf(current_parent),
                UNode {
                    width: UVal::Percent(1.0),
                    height: UVal::Percent(1.0),
                    ..default()
                },
                ULayout::default(),
            ))
            .id();
        current_parent = child;
    }

    let leaf = app
        .world_mut()
        .spawn((
            ChildOf(current_parent),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
        ))
        .id();

    app.update();

    let work_state = app.world().resource::<UiWorkState>();
    assert!(work_state.is_settled(), "Frame 1 must be settled");
    assert!(
        !work_state.budget_exhausted(),
        "Frame 1 must not exhaust budget"
    );

    // Frame 2: Resize root.
    app.world_mut()
        .entity_mut(root)
        .get_mut::<URootUi>()
        .expect("root must exist")
        .canvas = UiCanvasSize::Fixed(Vec2::new(800.0, 800.0));

    app.update();

    let work_state = app.world().resource::<UiWorkState>();
    assert!(
        work_state.is_settled(),
        "Frame 2 must be settled (iterations={}, budget_exhausted={})",
        work_state.last_frame_iterations(),
        work_state.budget_exhausted()
    );
    assert!(
        !work_state.budget_exhausted(),
        "Frame 2 must not exhaust budget"
    );
    assert_eq!(work_state.last_frame_iterations(), 2);

    let leaf_size = app
        .world()
        .entity(leaf)
        .get::<ComputedSize>()
        .copied()
        .expect("leaf must have computed size");
    assert_eq!(leaf_size.width, 800.0);
    assert_eq!(leaf_size.height, 800.0);
}

#[test]
fn updating_node_zindex_immediately_updates_child_depth_without_resize() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 200.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let child_a = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(50.0),
                ..default()
            },
            UZIndex::Local(1),
        ))
        .id();

    let child_b = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(50.0),
                ..default()
            },
            UZIndex::Local(2),
        ))
        .id();

    app.update();

    let z_a_before = app.world().entity(child_a).get::<Transform>().unwrap().translation.z;
    let z_b_before = app.world().entity(child_b).get::<Transform>().unwrap().translation.z;
    assert!(z_a_before < z_b_before, "child_b should be higher than child_a (z_a={z_a_before}, z_b={z_b_before})");

    // Frame 2: Dynamically elevate child_a above child_b using UZIndex::LocalMax
    *app.world_mut().entity_mut(child_a).get_mut::<UZIndex>().unwrap() = UZIndex::LocalMax;

    app.update();

    let z_a_after = app.world().entity(child_a).get::<Transform>().unwrap().translation.z;
    let z_b_after = app.world().entity(child_b).get::<Transform>().unwrap().translation.z;
    assert!(
        z_a_after > z_b_after,
        "child_a should now be higher than child_b without window resize (z_a={z_a_after}, z_b={z_b_after})"
    );
}

#[test]
fn wrapped_flex_container_with_visual_plugins_settles_and_idles_cleanly() {
    use crate::layout::geometry::USides;
    use crate::layout::univis_node::{UBorder, UFlexWrap};
    use crate::layout::visual::{UGradient, UInnerGlow, UShadow};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, UnivisLayoutPlugin));

    let root = app
        .world_mut()
        .spawn((
            URootUi::world_2d(Vec2::new(400.0, 600.0)),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                ..default()
            },
            ULayout::default(),
        ))
        .id();

    let card = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                padding: USides::all(10.0),
                ..default()
            },
            ULayout {
                display: crate::layout::univis_node::UDisplay::Flex,
                flex_direction: crate::layout::univis_node::UFlexDirection::Column,
                gap: 10.0,
                ..default()
            },
        ))
        .id();

    let wrap_row = app
        .world_mut()
        .spawn((
            ChildOf(card),
            UNode {
                width: UVal::Percent(1.0),
                ..default()
            },
            UBorder::default(),
            UGradient::linear(Color::WHITE, Color::BLACK, 0.0),
            UInnerGlow::default(),
            UShadow::default(),
            ULayout {
                display: crate::layout::univis_node::UDisplay::Flex,
                flex_direction: crate::layout::univis_node::UFlexDirection::Row,
                flex_wrap: UFlexWrap::Wrap,
                gap: 10.0,
                row_gap: Some(10.0),
                column_gap: Some(10.0),
                ..default()
            },
        ))
        .id();

    for _ in 0..5 {
        app.world_mut().spawn((
            ChildOf(wrap_row),
            UNode {
                width: UVal::Px(120.0),
                height: UVal::Px(30.0),
                ..default()
            },
        ));
    }

    let sibling = app
        .world_mut()
        .spawn((
            ChildOf(card),
            UNode {
                width: UVal::Px(100.0),
                height: UVal::Px(20.0),
                ..default()
            },
        ))
        .id();

    // Frame 1: Initial layout settles (even with multi-line wrap propagation into `card`).
    app.update();
    {
        let work_state = app.world().resource::<UiWorkState>();
        assert!(
            work_state.is_settled(),
            "Wrapped container must settle on frame 1 (pending={:?})",
            work_state.pending()
        );
        assert!(!work_state.budget_exhausted());
    }

    let wrap_size = app.world().entity(wrap_row).get::<ComputedSize>().copied().unwrap();
    let card_size = app.world().entity(card).get::<ComputedSize>().copied().unwrap();
    assert!(
        wrap_size.height >= 70.0,
        "5x120px children inside 380px row must wrap to at least 2 lines, got height={}",
        wrap_size.height
    );
    assert!(
        card_size.height >= wrap_size.height + 20.0 + 10.0,
        "Parent card height ({}) must include wrapped row height ({}) plus sibling/padding",
        card_size.height,
        wrap_size.height
    );
    let _ = sibling;

    // Frames 2..=4: Steady idle frames must finish in 1 pass without bumping generation.
    let gen_after_f1 = app.world().resource::<UiWorkState>().current_generation();
    for frame in 2..=4 {
        app.update();
        let work_state = app.world().resource::<UiWorkState>();
        assert!(work_state.is_settled(), "Frame {frame} must stay settled");
        assert!(!work_state.budget_exhausted(), "Frame {frame} must not exhaust budget");
        assert_eq!(
            work_state.last_frame_iterations(),
            1,
            "Idle frame {frame} must take exactly 1 settlement pass"
        );
        assert_eq!(
            work_state.current_generation(),
            gen_after_f1,
            "Idle frame {frame} must not start a new generation"
        );
    }

    // Mutate visual plugins and confirm settlement converges immediately.
    app.world_mut()
        .entity_mut(wrap_row)
        .get_mut::<UBorder>()
        .unwrap()
        .width = 2.0;
    app.update();
    assert!(app.world().resource::<UiWorkState>().is_settled());
    assert!(!app.world().resource::<UiWorkState>().budget_exhausted());

    app.update();
    assert_eq!(app.world().resource::<UiWorkState>().last_frame_iterations(), 1);

    // Resize root wider so wrapped items collapse back to 1 line cleanly.
    app.world_mut()
        .entity_mut(root)
        .get_mut::<URootUi>()
        .unwrap()
        .canvas = UiCanvasSize::Fixed(Vec2::new(800.0, 600.0));
    app.update();
    {
        let work_state = app.world().resource::<UiWorkState>();
        assert!(work_state.is_settled());
        assert!(!work_state.budget_exhausted());
    }
    let unwrapped_size = app.world().entity(wrap_row).get::<ComputedSize>().copied().unwrap();
    assert!(
        (unwrapped_size.height - 30.0).abs() < 0.01,
        "Resized 800px row should collapse back to 1 line height 30.0, got {}",
        unwrapped_size.height
    );

    app.update();
    assert_eq!(app.world().resource::<UiWorkState>().last_frame_iterations(), 1);
}
