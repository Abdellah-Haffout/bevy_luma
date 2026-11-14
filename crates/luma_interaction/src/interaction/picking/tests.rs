use super::*;
use bevy::camera::{
    CameraProjection, ComputedCameraValues, NormalizedRenderTarget, RenderTargetInfo,
};
use bevy::ecs::message::Messages;
use bevy::ecs::system::SystemState;
use bevy::picking::pointer::{Location, PointerMap};
use luma_engine::layout::UnivisLayoutPlugin;
use luma_engine::layout::layout_system::{
    ResolvedRootStack, ResolvedRootUi, URootUi, UiCameraRef, UiCanvasSize, UiSpace,
};
use luma_engine::layout::query::UiPickingContext;
use luma_engine::layout::univis_node::{UAlignItems, UJustifyContent, ULayout};
use luma_engine::schedule::{
    UiPendingStages, UiPickingRuntimeState, UiRolloutConfig, UiSettlementSchedule,
    UiValidationMode, UiWorkState, UnivisPostUpdateSet, sync_picking_runtime_state,
};

fn sample_root(root_entity: Entity, space: UiSpace) -> (ResolvedRootUi, ResolvedRootStack) {
    let root = ResolvedRootUi {
        root_entity,
        space,
        canvas: match space {
            UiSpace::Screen => UiCanvasSize::Viewport,
            UiSpace::World2d | UiSpace::World3d => UiCanvasSize::Fixed(Vec2::new(800.0, 600.0)),
        },
        canvas_size: Vec2::new(800.0, 600.0),
        camera_entity: Some(Entity::PLACEHOLDER),
        meters_per_unit: URootUi::DEFAULT_METERS_PER_UNIT,
        resolution_scale: URootUi::DEFAULT_RESOLUTION_SCALE,
    };
    let band_width = if matches!(space, UiSpace::Screen) {
        0.004
    } else {
        root.ui_units_to_world_scale() * 0.04
    };
    (root, ResolvedRootStack::with_capsule(0.0, band_width))
}

fn sample_picking_context(
    root_entity: Entity,
    camera_entity: Entity,
    space: UiSpace,
    layout_depth: usize,
    order: i32,
    root_sort_key: f32,
    clip_ancestor: Option<Entity>,
) -> UiPickingContext {
    let (_, mut stack) = sample_root(root_entity, space);
    stack.capsule_sort_key = root_sort_key;

    UiPickingContext {
        root_entity: Some(root_entity),
        camera_entity: Some(camera_entity),
        space,
        clip_ancestor,
        root_sort_key,
        local_depth_key: stack.local_depth_key(layout_depth, order),
        world_scale: 1.0,
    }
}

fn spawn_camera(app: &mut App, perspective: bool, translation: Vec3) -> Entity {
    let mut camera = Camera::default();
    camera.computed = ComputedCameraValues {
        target_info: Some(RenderTargetInfo {
            physical_size: UVec2::new(800, 600),
            scale_factor: 1.0,
        }),
        clip_from_view: if perspective {
            PerspectiveProjection {
                fov: core::f32::consts::FRAC_PI_2,
                aspect_ratio: 800.0 / 600.0,
                near: 0.1,
                ..default()
            }
            .get_clip_from_view()
        } else {
            OrthographicProjection {
                area: Rect::new(-400.0, -300.0, 400.0, 300.0),
                ..OrthographicProjection::default_2d()
            }
            .get_clip_from_view()
        },
        ..default()
    };

    app.world_mut()
        .spawn((
            camera,
            GlobalTransform::from(Transform::from_translation(translation)),
        ))
        .id()
}

fn spawn_pointer(app: &mut App, position: Vec2) {
    app.init_resource::<PointerMap>();
    app.world_mut().spawn((
        PointerId::Mouse,
        PointerLocation::new(Location {
            target: NormalizedRenderTarget::None {
                width: 800,
                height: 600,
            },
            position,
        }),
    ));
}

fn run_picking_for_space(space: UiSpace, perspective: bool) -> Vec<PointerHits> {
    let mut app = App::new();
    app.init_resource::<PointerMap>();
    app.add_message::<PointerHits>();
    app.add_systems(Update, univis_picking_backend);

    let camera_entity = spawn_camera(
        &mut app,
        perspective,
        Vec3::new(0.0, 0.0, if perspective { 5.0 } else { 1000.0 }),
    );
    let root = app.world_mut().spawn_empty().id();

    let node = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UInteraction::default(),
            UNode {
                width: luma_engine::layout::geometry::UVal::Px(200.0),
                height: luma_engine::layout::geometry::UVal::Px(120.0),
                ..default()
            },
            ComputedSize {
                width: 200.0,
                height: 120.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::default(),
            sample_picking_context(root, camera_entity, space, 1, 0, 0.0, None),
        ))
        .id();

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected = hits.drain().collect::<Vec<_>>();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].picks.len(), 1);
    assert_eq!(collected[0].picks[0].0, node);
    collected
}

#[test]
fn post_settle_picking_hits_first_frame_geometry_with_picking_context() {
    let mut app = App::new();
    app.add_message::<PointerHits>();
    app.init_resource::<PickingSyncState>();
    app.init_resource::<PickingValidationState>();
    app.init_resource::<UiPickingRuntimeState>();
    app.add_systems(
        Update,
        (sync_picking_runtime_state, post_settle_picking_backend).chain(),
    );

    let mut work_state = UiWorkState::default();
    work_state.begin_generation(UiPendingStages {
        render: true,
        ..Default::default()
    });
    app.insert_resource(work_state);

    let camera_entity = spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));
    let root = app.world_mut().spawn_empty().id();

    let node = app
        .world_mut()
        .spawn((
            UInteraction::default(),
            UNode {
                width: luma_engine::layout::geometry::UVal::Px(200.0),
                height: luma_engine::layout::geometry::UVal::Px(120.0),
                ..default()
            },
            ComputedSize {
                width: 200.0,
                height: 120.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::default(),
            sample_picking_context(root, camera_entity, UiSpace::Screen, 1, 0, 0.0, None),
        ))
        .id();

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.world_mut()
        .resource_mut::<PickingSyncState>()
        .pointer_generation = 1;

    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected = hits.drain().collect::<Vec<_>>();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].picks.len(), 1);
    assert_eq!(collected[0].picks[0].0, node);
}

#[test]
fn post_settle_picking_validation_records_missing_picking_contexts_without_emitting_hits() {
    let mut app = App::new();
    app.add_message::<PointerHits>();
    app.init_resource::<PickingSyncState>();
    app.init_resource::<PickingValidationState>();
    app.init_resource::<UiPickingRuntimeState>();
    app.insert_resource(UiRolloutConfig {
        use_post_settle_picking: false,
        validation: UiValidationMode::LogWarnings,
        ..default()
    });
    app.add_systems(
        Update,
        (sync_picking_runtime_state, post_settle_picking_backend).chain(),
    );

    let mut work_state = UiWorkState::default();
    work_state.begin_generation(UiPendingStages {
        render: true,
        ..Default::default()
    });
    app.insert_resource(work_state);

    spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));

    app.world_mut().spawn((
        UInteraction::default(),
        UNode {
            width: luma_engine::layout::geometry::UVal::Px(200.0),
            height: luma_engine::layout::geometry::UVal::Px(120.0),
            ..default()
        },
        ComputedSize {
            width: 200.0,
            height: 120.0,
            local_pos: Vec2::ZERO,
        },
        GlobalTransform::default(),
    ));

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.world_mut()
        .resource_mut::<PickingSyncState>()
        .pointer_generation = 1;

    app.update();

    let validation_state = app.world().resource::<PickingValidationState>();
    assert_eq!(validation_state.ui_generation(), 1);
    assert_eq!(validation_state.pointer_generation(), 1);
    assert_eq!(validation_state.missing_context_count(), 1);

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected = hits.drain().collect::<Vec<_>>();
    assert!(collected.is_empty());
}

#[test]
fn post_settle_picking_refreshes_picking_context_after_root_state_change() {
    let mut app = App::new();
    app.add_message::<PointerHits>();
    app.add_plugins(UnivisLayoutPlugin);
    app.init_resource::<PickingSyncState>();
    app.init_resource::<PickingValidationState>();
    app.init_resource::<UiPickingRuntimeState>();
    app.add_systems(
        UiSettlementSchedule,
        (sync_picking_runtime_state, post_settle_picking_backend)
            .in_set(UnivisPostUpdateSet::ExternalPostSolve)
            .chain(),
    );

    let camera_a = spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));
    let camera_b = spawn_camera(&mut app, false, Vec3::new(250.0, 0.0, 1000.0));

    let root = app
        .world_mut()
        .spawn((
            URootUi {
                camera: UiCameraRef::Entity(camera_a),
                ..URootUi::screen()
            },
            UNode {
                width: luma_engine::layout::geometry::UVal::Percent(1.0),
                height: luma_engine::layout::geometry::UVal::Percent(1.0),
                ..default()
            },
            ULayout {
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    let node = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UInteraction::default(),
            UNode {
                width: luma_engine::layout::geometry::UVal::Px(120.0),
                height: luma_engine::layout::geometry::UVal::Px(80.0),
                ..default()
            },
        ))
        .id();

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.world_mut()
        .resource_mut::<PickingSyncState>()
        .pointer_generation = 1;

    app.update();

    {
        let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
        let collected = hits.drain().collect::<Vec<_>>();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].picks.len(), 1);
        assert_eq!(collected[0].picks[0].0, node);
    }

    let picking_before = app
        .world()
        .entity(node)
        .get::<UiPickingContext>()
        .copied()
        .expect("node should have picking context after the first update");
    assert_eq!(picking_before.camera_entity, Some(camera_a));

    app.world_mut()
        .entity_mut(root)
        .get_mut::<URootUi>()
        .expect("root should keep public root state")
        .camera = UiCameraRef::Entity(camera_b);
    app.world_mut()
        .resource_mut::<PickingSyncState>()
        .pointer_generation = 2;

    app.update();

    let picking_after = app
        .world()
        .entity(node)
        .get::<UiPickingContext>()
        .copied()
        .expect("node should keep picking context after camera reassignment");
    assert_eq!(picking_after.camera_entity, Some(camera_b));

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected = hits.drain().collect::<Vec<_>>();
    assert!(collected.is_empty());
}

#[test]
fn is_ancestor_of_walks_up_the_parent_chain() {
    let mut app = App::new();

    let root = app.world_mut().spawn_empty().id();
    let child = app.world_mut().spawn(ChildOf(root)).id();
    let grandchild = app.world_mut().spawn(ChildOf(child)).id();

    let mut state = SystemState::<Query<&ChildOf>>::new(app.world_mut());
    let parents = state.get(app.world()).unwrap();

    assert!(is_ancestor_of(root, grandchild, &parents));
    assert!(is_ancestor_of(child, grandchild, &parents));
    assert!(!is_ancestor_of(grandchild, child, &parents));
}

#[test]
fn intersect_ray_with_node_plane_returns_world_and_local_hit() {
    let ray = CachedPointerRay {
        order: 0.0,
        origin: Vec3::new(0.0, 0.0, 5.0),
        direction: Vec3::NEG_Z,
    };
    let transform = GlobalTransform::from(Transform::default());

    let (world_hit, local_hit, normal, distance) = intersect_ray_with_node_plane(&ray, &transform)
        .expect("ray should intersect the default UI plane");

    assert!(world_hit.abs_diff_eq(Vec3::ZERO, 1e-5));
    assert!(local_hit.abs_diff_eq(Vec2::ZERO, 1e-5));
    assert!(normal.abs_diff_eq(Vec3::Z, 1e-5));
    assert!((distance - 5.0).abs() <= 1e-5);
}

#[test]
fn is_clipped_by_ancestors_detects_points_outside_enabled_clipper() {
    let mut app = App::new();

    let clipper = app
        .world_mut()
        .spawn((
            GlobalTransform::default(),
            ComputedSize {
                width: 100.0,
                height: 100.0,
                ..default()
            },
            UNode::default(),
            UClip { enabled: true },
        ))
        .id();
    let child = app.world_mut().spawn(ChildOf(clipper)).id();

    let mut state = SystemState::<(
        Query<&ChildOf>,
        Query<(&GlobalTransform, &ComputedSize, &UNode, &UClip)>,
    )>::new(app.world_mut());
    let (parents, clippers) = state.get(app.world()).unwrap();

    assert!(!is_clipped_by_ancestors(
        child,
        Vec3::new(0.0, 0.0, 0.0),
        &parents,
        &clippers,
    ));
    assert!(is_clipped_by_ancestors(
        child,
        Vec3::new(80.0, 80.0, 0.0),
        &parents,
        &clippers,
    ));
}

#[test]
fn picking_prefers_the_higher_root_capsule_over_a_deeper_child_in_a_lower_root() {
    let mut app = App::new();
    app.init_resource::<PointerMap>();
    app.add_message::<PointerHits>();
    app.add_systems(Update, univis_picking_backend);

    let camera_entity = spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));

    let lower_root = app.world_mut().spawn_empty().id();
    let lower_node = app
        .world_mut()
        .spawn((
            ChildOf(lower_root),
            UInteraction::default(),
            UNode {
                width: luma_engine::layout::geometry::UVal::Px(200.0),
                height: luma_engine::layout::geometry::UVal::Px(120.0),
                ..default()
            },
            ComputedSize {
                width: 200.0,
                height: 120.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.08)),
            sample_picking_context(
                lower_root,
                camera_entity,
                UiSpace::World2d,
                3,
                32,
                0.0,
                None,
            ),
        ))
        .id();

    let upper_root = app.world_mut().spawn_empty().id();
    let upper_node = app
        .world_mut()
        .spawn((
            ChildOf(upper_root),
            UInteraction::default(),
            UNode {
                width: luma_engine::layout::geometry::UVal::Px(200.0),
                height: luma_engine::layout::geometry::UVal::Px(120.0),
                ..default()
            },
            ComputedSize {
                width: 200.0,
                height: 120.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.02)),
            sample_picking_context(
                upper_root,
                camera_entity,
                UiSpace::World2d,
                1,
                0,
                0.05,
                None,
            ),
        ))
        .id();

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected = hits.drain().collect::<Vec<_>>();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].picks.len(), 2);
    assert_eq!(collected[0].picks[0].0, upper_node);
    assert_eq!(collected[0].picks[1].0, lower_node);
}

#[test]
fn picking_backend_hits_screen_nodes() {
    let hits = run_picking_for_space(UiSpace::Screen, false);
    assert_eq!(hits[0].order, 0.0);
}

#[test]
fn picking_backend_hits_world2d_nodes() {
    let hits = run_picking_for_space(UiSpace::World2d, false);
    assert_eq!(hits[0].order, 0.0);
}

#[test]
fn picking_backend_hits_world3d_nodes_with_perspective_camera() {
    let hits = run_picking_for_space(UiSpace::World3d, true);
    assert_eq!(hits[0].order, 0.0);
}

#[test]
fn picking_respects_outer_nested_clipper_and_rotated_world_scale_clip() {
    let mut app = App::new();
    app.init_resource::<PointerMap>();
    app.add_message::<PointerHits>();
    app.add_systems(Update, univis_picking_backend);

    let camera_entity = spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));
    let root = app.world_mut().spawn_empty().id();

    // Outer clipper blocks points with x > 0.5 world units (100 UI px * 0.02 world_scale = width 2.0 -> half 1.0, centered at -0.5).
    let outer_clipper = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UNode::default(),
            ComputedSize {
                width: 100.0,
                height: 100.0,
                local_pos: Vec2::ZERO,
            },
            UClip::enabled(true),
            GlobalTransform::from(Transform::from_xyz(-0.5, 0.0, 0.0)),
        ))
        .id();

    // Inner clipper centered at +0.5 overlaps part of outer, plus is rotated 45 deg.
    let inner_clipper = app
        .world_mut()
        .spawn((
            ChildOf(outer_clipper),
            UNode::default(),
            ComputedSize {
                width: 100.0,
                height: 100.0,
                local_pos: Vec2::ZERO,
            },
            UClip::enabled(true),
            GlobalTransform::from(
                Transform::from_xyz(0.5, 0.0, 0.0).with_rotation(Quat::from_rotation_z(0.5)),
            ),
        ))
        .id();

    // Child covers whole viewport around origin; picking context caches `inner_clipper` and `world_scale = 0.02`.
    let mut picking_ctx = sample_picking_context(
        root,
        camera_entity,
        UiSpace::World2d,
        3,
        0,
        0.0,
        Some(inner_clipper),
    );
    picking_ctx.world_scale = 0.02;

    let child = app
        .world_mut()
        .spawn((
            ChildOf(inner_clipper),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 40000.0,
                height: 40000.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.01)),
            picking_ctx,
        ))
        .id();

    // Point (1.2, 0.0) in world space: camera 800x600 ortho maps viewport center (400, 300) -> world (0,0).
    // Pixel (401.2, 300.0) -> world (1.2, 0.0), which is INSIDE inner_clipper (center 0.5, half 1.0 -> max x=1.5),
    // yet OUTSIDE outer_clipper (center -0.5, half 1.0 -> max x=0.5).
    spawn_pointer(&mut app, Vec2::new(401.2, 300.0));
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let outside_outer: Vec<_> = hits.drain().collect();
    assert!(
        outside_outer.iter().all(|h| h.picks.is_empty()),
        "point outside outer nested clipper must not be picked even if inside inner clipper"
    );

    // Move pointer to viewport center (400, 300) -> world (0.0, 0.0), inside both clippers.
    let mut ptr_q = app
        .world_mut()
        .query::<&mut PointerLocation>();
    for mut ptr in ptr_q.iter_mut(app.world_mut()) {
        ptr.location = Some(Location {
            target: NormalizedRenderTarget::None {
                width: 800,
                height: 600,
            },
            position: Vec2::new(400.0, 300.0),
        });
    }
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let inside_both: Vec<_> = hits.drain().collect();
    assert_eq!(inside_both.len(), 1);
    assert_eq!(inside_both[0].picks.len(), 1);
    assert_eq!(inside_both[0].picks[0].0, child);
}

#[test]
fn picking_skips_hidden_and_inherited_hidden_nodes() {
    let mut app = App::new();
    app.init_resource::<PointerMap>();
    app.add_message::<PointerHits>();
    app.add_systems(Update, univis_picking_backend);

    let camera_entity = spawn_camera(&mut app, false, Vec3::new(0.0, 0.0, 1000.0));
    let root = app.world_mut().spawn_empty().id();

    let hidden_direct = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.05)),
            Visibility::Hidden,
            sample_picking_context(root, camera_entity, UiSpace::Screen, 1, 0, 0.0, None),
        ))
        .id();

    let hidden_parent = app
        .world_mut()
        .spawn((ChildOf(root), UNode::default(), Visibility::Hidden))
        .id();

    let hidden_inherited = app
        .world_mut()
        .spawn((
            ChildOf(hidden_parent),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.04)),
            Visibility::Inherited,
            sample_picking_context(root, camera_entity, UiSpace::Screen, 2, 1, 0.0, None),
        ))
        .id();

    let visible_node = app
        .world_mut()
        .spawn((
            ChildOf(root),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.02)),
            sample_picking_context(root, camera_entity, UiSpace::Screen, 1, 2, 0.0, None),
        ))
        .id();

    spawn_pointer(&mut app, Vec2::new(400.0, 300.0));
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected: Vec<_> = hits.drain().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].picks.len(), 1);
    assert_eq!(collected[0].picks[0].0, visible_node);
    assert_ne!(collected[0].picks[0].0, hidden_direct);
    assert_ne!(collected[0].picks[0].0, hidden_inherited);
}

fn spawn_camera_with_target(
    app: &mut App,
    translation: Vec3,
    order: isize,
    target: Option<RenderTarget>,
) -> Entity {
    let mut camera = Camera {
        order,
        computed: ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: UVec2::new(800, 600),
                scale_factor: 1.0,
            }),
            clip_from_view: OrthographicProjection {
                area: Rect::new(-400.0, -300.0, 400.0, 300.0),
                ..OrthographicProjection::default_2d()
            }
            .get_clip_from_view(),
            ..default()
        },
        ..default()
    };
    let _ = &mut camera;
    let entity = app
        .world_mut()
        .spawn((
            camera,
            GlobalTransform::from(Transform::from_translation(translation)),
        ))
        .id();
    if let Some(rt) = target {
        app.world_mut().entity_mut(entity).insert(rt);
    }
    entity
}

#[test]
fn picking_filters_cameras_by_window_and_offscreen_render_target() {
    let mut app = App::new();
    app.init_resource::<PointerMap>();
    app.add_message::<PointerHits>();
    app.add_systems(Update, univis_picking_backend);

    let win_a = app.world_mut().spawn((Window::default(), PrimaryWindow)).id();
    let win_b = app.world_mut().spawn(Window::default()).id();

    let cam_a = spawn_camera_with_target(&mut app, Vec3::new(0.0, 0.0, 1000.0), 0, None);
    let cam_b = spawn_camera_with_target(
        &mut app,
        Vec3::new(0.0, 0.0, 1000.0),
        10,
        Some(RenderTarget::Window(bevy::window::WindowRef::Entity(win_b))),
    );
    let cam_offscreen = spawn_camera_with_target(
        &mut app,
        Vec3::new(0.0, 0.0, 1000.0),
        20,
        Some(RenderTarget::Image(Handle::<Image>::default().into())),
    );

    let root_a = app.world_mut().spawn_empty().id();
    let root_b = app.world_mut().spawn_empty().id();
    let root_offscreen = app.world_mut().spawn_empty().id();

    let node_a = app
        .world_mut()
        .spawn((
            ChildOf(root_a),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.02)),
            sample_picking_context(root_a, cam_a, UiSpace::Screen, 1, 0, 0.0, None),
        ))
        .id();

    let node_b = app
        .world_mut()
        .spawn((
            ChildOf(root_b),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.02)),
            sample_picking_context(root_b, cam_b, UiSpace::Screen, 1, 0, 0.0, None),
        ))
        .id();

    let node_offscreen = app
        .world_mut()
        .spawn((
            ChildOf(root_offscreen),
            UInteraction::default(),
            UNode::default(),
            ComputedSize {
                width: 200.0,
                height: 200.0,
                local_pos: Vec2::ZERO,
            },
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.02)),
            sample_picking_context(
                root_offscreen,
                cam_offscreen,
                UiSpace::Screen,
                1,
                0,
                0.0,
                None,
            ),
        ))
        .id();

    // Pointer on Window A must hit only node_a, never node_b or node_offscreen.
    let target_a = RenderTarget::Window(bevy::window::WindowRef::Entity(win_a))
        .normalize(Some(win_a))
        .expect("normalize win_a");
    app.world_mut().spawn((
        PointerId::Mouse,
        PointerLocation::new(Location {
            target: target_a,
            position: Vec2::new(400.0, 300.0),
        }),
    ));
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected: Vec<_> = hits.drain().collect();
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].picks.len(), 1);
    assert_eq!(collected[0].picks[0].0, node_a);
    assert_ne!(collected[0].picks[0].0, node_b);
    assert_ne!(collected[0].picks[0].0, node_offscreen);

    // Move the pointer to Window B: now only node_b should be picked.
    let target_b = RenderTarget::Window(bevy::window::WindowRef::Entity(win_b))
        .normalize(Some(win_a))
        .expect("normalize win_b");
    let mut ptr_q = app.world_mut().query::<&mut PointerLocation>();
    for mut ptr in ptr_q.iter_mut(app.world_mut()) {
        ptr.location = Some(Location {
            target: target_b.clone(),
            position: Vec2::new(400.0, 300.0),
        });
    }
    app.update();

    let mut hits = app.world_mut().resource_mut::<Messages<PointerHits>>();
    let collected_b: Vec<_> = hits.drain().collect();
    assert_eq!(collected_b.len(), 1);
    assert_eq!(collected_b[0].picks.len(), 1);
    assert_eq!(collected_b[0].picks[0].0, node_b);
}
