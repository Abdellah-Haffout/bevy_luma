use crate::layout::components::{CachedUiContext, UiLocalStacking};
use crate::layout::layout_system::UiSpace;
use crate::layout::render::material_3d::UNodeMaterial3d;
use crate::layout::text::{TextSelectionHighlightMarker, UTextLabelSdfMaterial3d};
use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::ecs::entity::EntityHashMap;
use bevy::pbr::{
    EntitiesNeedingSpecialization as EntitiesNeedingSpecialization3d, Material,
    check_entities_needing_specialization as check_3d_specializations,
};
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    render_phase::{PhaseItem, ViewSortedRenderPhases, sort_phase_system},
    sync_world::{MainEntity, MainEntityHashMap},
};
use bevy::sprite_render::{
    EntitiesNeedingSpecialization as EntitiesNeedingSpecialization2d, Material2d,
    check_entities_needing_specialization as check_2d_specializations,
};
use bevy::transform::TransformSystems;

const WORLD3D_SORT_BIAS_BAND: f32 = 0.02;
const WORLD3D_FALLBACK_LAYER_DIVISOR: f32 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum World3dUiSublayer {
    Node = 0,
    SelectionHighlight = 1,
    Text = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct World3dUiSortEntry {
    pub root_center: Vec3,
    pub sort_bias: f32,
}

#[derive(Resource, Default)]
pub(crate) struct World3dUiSortTable(pub MainEntityHashMap<World3dUiSortEntry>);

/// Updates cached transform keys for entities whose transform or mesh/material
/// changed, returning `true` when only the transform key changed and the entity
/// must be re-queued in Bevy's retained transparent render phase.
fn track_transform_key<K: Copy + PartialEq>(
    cached: &mut EntityHashMap<K>,
    entity: Entity,
    current: K,
    mesh_or_material_changed: bool,
) -> bool {
    let previous = cached.insert(entity, current);
    !mesh_or_material_changed && previous != Some(current)
}

/// Registers post-propagate depth/transform tracking for a 2D [`Material2d`] so
/// retained `Transparent2d` phase sort keys stay synchronized with
/// `GlobalTransform.translation().z`.
pub(crate) fn install_material2d_depth_sync<M: Material2d>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        sync_material2d_depth_specializations::<M>
            .after(TransformSystems::Propagate)
            .after(check_2d_specializations::<M>)
            .ambiguous_with_all(),
    );
}

/// Registers post-propagate transform tracking for a 3D [`Material`] so
/// retained `Transparent3d` phase sorting distances stay synchronized with
/// `GlobalTransform`.
pub(crate) fn install_material3d_transform_sync<M: Material>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        sync_material3d_transform_specializations::<M>
            .after(TransformSystems::Propagate)
            .after(check_3d_specializations::<M>)
            .ambiguous_with_all(),
    );
}

/// Enqueues 2D material entities whose world Z changed (without `Mesh2d` or
/// `MeshMaterial2d` changing) so Bevy's retained `Transparent2d` phase
/// refreshes their `sort_key` on the same frame.
fn sync_material2d_depth_specializations<M: Material2d>(
    mut cached_z: Local<EntityHashMap<f32>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization2d<M>>,
    mut removed_meshes: RemovedComponents<Mesh2d>,
    mut removed_materials: RemovedComponents<MeshMaterial2d<M>>,
    changed_entities: Query<
        (Entity, &GlobalTransform, Ref<Mesh2d>, Ref<MeshMaterial2d<M>>),
        Or<(
            Changed<GlobalTransform>,
            Changed<Mesh2d>,
            Changed<MeshMaterial2d<M>>,
        )>,
    >,
) {
    for entity in removed_meshes.read().chain(removed_materials.read()) {
        cached_z.remove(&entity);
    }

    for (entity, global_transform, mesh, material) in &changed_entities {
        let current_z = global_transform.translation().z;
        if !current_z.is_finite() {
            continue;
        }
        // Normalize -0.0 and +0.0 to 0.0 so PartialEq matches IEEE float value semantics.
        let normalized_z = if current_z == 0.0 { 0.0 } else { current_z };
        let mesh_or_material_changed = mesh.is_changed() || material.is_changed();
        if track_transform_key(&mut cached_z, entity, normalized_z, mesh_or_material_changed) {
            entities_needing_specialization.changed.push(entity);
        }
    }
}

/// Enqueues 3D material entities whose `GlobalTransform` changed (without
/// `Mesh3d` or `MeshMaterial3d` changing) so Bevy's retained `Transparent3d`
/// phase refreshes their cached `mesh_center` on the same frame.
fn sync_material3d_transform_specializations<M: Material>(
    mut cached_transforms: Local<EntityHashMap<GlobalTransform>>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization3d<M>>,
    mut removed_meshes: RemovedComponents<Mesh3d>,
    mut removed_materials: RemovedComponents<MeshMaterial3d<M>>,
    changed_entities: Query<
        (Entity, &GlobalTransform, Ref<Mesh3d>, Ref<MeshMaterial3d<M>>),
        Or<(
            Changed<GlobalTransform>,
            Changed<Mesh3d>,
            Changed<MeshMaterial3d<M>>,
        )>,
    >,
) {
    for entity in removed_meshes.read().chain(removed_materials.read()) {
        cached_transforms.remove(&entity);
    }

    for (entity, global_transform, mesh, material) in &changed_entities {
        if !global_transform.translation().is_finite() {
            continue;
        }
        let mesh_or_material_changed = mesh.is_changed() || material.is_changed();
        if track_transform_key(
            &mut cached_transforms,
            entity,
            *global_transform,
            mesh_or_material_changed,
        ) {
            entities_needing_specialization.changed.push(entity);
        }
    }
}

/// Computes a monotonic positive `Transparent3d::depth_bias` for one `World3d`
/// UI element so nodes, selection highlights, and glyphs inside the same panel
/// draw in strict UI painter's order regardless of viewing angle.
pub(crate) fn world3d_layer_sort_bias(
    stacking: Option<&UiLocalStacking>,
    sublayer: World3dUiSublayer,
) -> f32 {
    let (base, span) = match stacking {
        Some(stacking) => (
            stacking.normalized.clamp(0.0, 1.0),
            if stacking.slot_span > 0.0 {
                stacking.slot_span
            } else {
                1.0 / WORLD3D_FALLBACK_LAYER_DIVISOR
            },
        ),
        None => (0.0, 1.0 / WORLD3D_FALLBACK_LAYER_DIVISOR),
    };

    let sub_offset = match sublayer {
        World3dUiSublayer::Node => 0.0,
        World3dUiSublayer::SelectionHighlight => 0.25,
        World3dUiSublayer::Text => 0.50,
    };

    (base + span * sub_offset).clamp(0.0, 1.0) * WORLD3D_SORT_BIAS_BAND
}

fn resolve_world3d_entry(
    ctx: &CachedUiContext,
    stacking: Option<&UiLocalStacking>,
    sublayer: World3dUiSublayer,
    root_transforms: &Query<&GlobalTransform>,
) -> Option<World3dUiSortEntry> {
    if !matches!(ctx.space, UiSpace::World3d) {
        return None;
    }
    let root_entity = ctx.root_entity?;
    let root_transform = root_transforms.get(root_entity).ok()?;
    Some(World3dUiSortEntry {
        root_center: root_transform.translation(),
        sort_bias: world3d_layer_sort_bias(stacking, sublayer),
    })
}

fn collect_world3d_ui_sort_entries(
    table: &mut World3dUiSortTable,
    root_transforms: &Query<&GlobalTransform>,
    ui_nodes: &Query<
        (Entity, &CachedUiContext, Option<&UiLocalStacking>),
        With<MeshMaterial3d<UNodeMaterial3d>>,
    >,
    text_children: &Query<
        (Entity, &ChildOf, Has<TextSelectionHighlightMarker>),
        With<MeshMaterial3d<UTextLabelSdfMaterial3d>>,
    >,
    parent_contexts: &Query<(&CachedUiContext, Option<&UiLocalStacking>)>,
) {
    table.0.clear();

    for (entity, ctx, stacking) in ui_nodes.iter() {
        if let Some(entry) =
            resolve_world3d_entry(ctx, stacking, World3dUiSublayer::Node, root_transforms)
        {
            table.0.insert(MainEntity::from(entity), entry);
        }
    }

    for (entity, child_of, is_highlight) in text_children.iter() {
        let Ok((ctx, stacking)) = parent_contexts.get(child_of.parent()) else {
            continue;
        };
        let sublayer = if is_highlight {
            World3dUiSublayer::SelectionHighlight
        } else {
            World3dUiSublayer::Text
        };
        if let Some(entry) = resolve_world3d_entry(ctx, stacking, sublayer, root_transforms) {
            table.0.insert(MainEntity::from(entity), entry);
        }
    }
}

fn extract_world3d_ui_sort_table(
    mut table: ResMut<World3dUiSortTable>,
    root_transforms: Extract<Query<&GlobalTransform>>,
    ui_nodes: Extract<
        Query<
            (Entity, &CachedUiContext, Option<&UiLocalStacking>),
            With<MeshMaterial3d<UNodeMaterial3d>>,
        >,
    >,
    text_children: Extract<
        Query<
            (Entity, &ChildOf, Has<TextSelectionHighlightMarker>),
            With<MeshMaterial3d<UTextLabelSdfMaterial3d>>,
        >,
    >,
    parent_contexts: Extract<Query<(&CachedUiContext, Option<&UiLocalStacking>)>>,
) {
    collect_world3d_ui_sort_entries(
        &mut table,
        &root_transforms,
        &ui_nodes,
        &text_children,
        &parent_contexts,
    );
}

fn apply_world3d_ui_transparent_sort_overrides(
    table: Res<World3dUiSortTable>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    if table.0.is_empty() {
        return;
    }

    for phase in phases.0.values_mut() {
        for item in phase.items.values_mut() {
            if let Some(entry) = table.0.get(&item.main_entity()) {
                item.sorting_info = TransparentSortingInfo3d::Sorted {
                    mesh_center: entry.root_center,
                    depth_bias: entry.sort_bias,
                };
            }
        }
    }
}

/// Installs unified root-plane `Transparent3d` sort synchronization so tilted
/// `World3d` panels never invert child/text draw order against backgrounds.
pub(crate) fn install_world3d_transparent_sort_sync(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app
        .init_resource::<World3dUiSortTable>()
        .add_systems(ExtractSchedule, extract_world3d_ui_sort_table)
        .add_systems(
            Render,
            apply_world3d_ui_transparent_sort_overrides
                .in_set(RenderSystems::PhaseSort)
                .before(sort_phase_system::<Transparent3d>),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LumaEnginePlugin;
    use crate::layout::render::material::UNodeMaterial;
    use crate::layout::render::material_3d::UNodeMaterial3d;
    use bevy::asset::AssetPlugin;
    use bevy::pbr::EntitiesNeedingSpecialization as Entities3d;
    use bevy::sprite_render::EntitiesNeedingSpecialization as Entities2d;
    use bevy::text::TextPlugin;

    fn test_app_with_rendering() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            TransformPlugin,
            TextPlugin,
            LumaEnginePlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>();
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        app.world_mut().spawn(Camera2d);
        app
    }

    fn test_unode_material_3d() -> UNodeMaterial3d {
        UNodeMaterial3d {
            size: Vec2::new(40.0, 40.0),
            softness: 1.0,
            ..default()
        }
    }

    #[test]
    fn xy_only_translation_does_not_requeue_2d_specializations() {
        let mut app = test_app_with_rendering();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(40.0, 40.0));
        let mat = app
            .world_mut()
            .resource_mut::<Assets<UNodeMaterial>>()
            .add(UNodeMaterial::default());

        let entity = app
            .world_mut()
            .spawn((
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(0.0, 0.0, 5.0),
            ))
            .id();

        app.update();
        assert!(app.world().resource::<Entities2d<UNodeMaterial>>().changed.contains(&entity));
        // Second frame processes Frame 1's AssetEvent::Added in `mark_2d_meshes_as_changed`.
        app.update();
        // Frame 3: stationary entity is now clean.
        app.update();
        assert!(
            !app.world()
                .resource::<Entities2d<UNodeMaterial>>()
                .changed
                .contains(&entity)
        );

        // Moving X/Y while preserving Z should not enqueue a specialization update.
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .x = 120.0;
        app.update();
        assert!(
            !app.world()
                .resource::<Entities2d<UNodeMaterial>>()
                .changed
                .contains(&entity)
        );

        // Moving Z should enqueue immediately.
        app.world_mut()
            .entity_mut(entity)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .z = 8.0;
        app.update();
        assert!(
            app.world()
                .resource::<Entities2d<UNodeMaterial>>()
                .changed
                .contains(&entity)
        );
    }

    #[test]
    fn transform_change_requeues_3d_materials_and_handles_removal() {
        let mut app = test_app_with_rendering();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(40.0, 40.0));
        let mat_3d = app
            .world_mut()
            .resource_mut::<Assets<UNodeMaterial3d>>()
            .add(test_unode_material_3d());

        let node_entity = app
            .world_mut()
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mat_3d.clone()),
                Transform::from_xyz(0.0, 0.0, 1.0),
            ))
            .id();

        app.update();
        assert!(
            app.world()
                .resource::<Entities3d<UNodeMaterial3d>>()
                .changed
                .contains(&node_entity)
        );

        // Flush Frame 1 AssetEvent::Added before checking steady-state idle frame.
        app.update();
        app.update();
        assert!(
            !app.world()
                .resource::<Entities3d<UNodeMaterial3d>>()
                .changed
                .contains(&node_entity)
        );

        app.world_mut()
            .entity_mut(node_entity)
            .get_mut::<Transform>()
            .unwrap()
            .translation
            .z = 5.0;
        app.update();
        assert!(
            app.world()
                .resource::<Entities3d<UNodeMaterial3d>>()
                .changed
                .contains(&node_entity)
        );

        // Removing material then re-inserting keeps working cleanly.
        app.world_mut()
            .entity_mut(node_entity)
            .remove::<MeshMaterial3d<UNodeMaterial3d>>();
        app.update();
        app.world_mut()
            .entity_mut(node_entity)
            .insert(MeshMaterial3d(mat_3d));
        app.update();
        assert!(
            app.world()
                .resource::<Entities3d<UNodeMaterial3d>>()
                .changed
                .contains(&node_entity)
        );
    }

    #[test]
    fn world3d_transparent_sort_bias_and_table_preserve_panel_painter_order() {
        let parent_stack = UiLocalStacking {
            normalized: 0.25,
            slot_span: 0.25,
        };
        let child_stack = UiLocalStacking {
            normalized: 0.50,
            slot_span: 0.25,
        };

        let parent_node = world3d_layer_sort_bias(Some(&parent_stack), World3dUiSublayer::Node);
        let parent_hl =
            world3d_layer_sort_bias(Some(&parent_stack), World3dUiSublayer::SelectionHighlight);
        let parent_text = world3d_layer_sort_bias(Some(&parent_stack), World3dUiSublayer::Text);
        let child_node = world3d_layer_sort_bias(Some(&child_stack), World3dUiSublayer::Node);
        let child_hl =
            world3d_layer_sort_bias(Some(&child_stack), World3dUiSublayer::SelectionHighlight);
        let child_text = world3d_layer_sort_bias(Some(&child_stack), World3dUiSublayer::Text);

        assert!(parent_node < parent_hl);
        assert!(parent_hl < parent_text);
        assert!(parent_text < child_node);
        assert!(child_node < child_hl);
        assert!(child_hl < child_text);
        assert!(child_text <= WORLD3D_SORT_BIAS_BAND + 1e-6);

        let mut world = World::new();
        world.insert_resource(World3dUiSortTable::default());

        let panel_origin = Vec3::new(3.0, 2.0, -5.0);
        let root = world
            .spawn(GlobalTransform::from_translation(panel_origin))
            .id();

        let parent_ctx = CachedUiContext {
            root_entity: Some(root),
            space: UiSpace::World3d,
            ui_to_world_scale: 0.01,
            ..default()
        };
        let ui_node = world
            .spawn((
                MeshMaterial3d::<UNodeMaterial3d>(Handle::default()),
                parent_ctx,
                parent_stack,
            ))
            .id();
        let highlight_child = world
            .spawn((
                ChildOf(ui_node),
                TextSelectionHighlightMarker,
                MeshMaterial3d::<UTextLabelSdfMaterial3d>(Handle::default()),
            ))
            .id();
        let text_child = world
            .spawn((
                ChildOf(ui_node),
                MeshMaterial3d::<UTextLabelSdfMaterial3d>(Handle::default()),
            ))
            .id();

        let screen_node = world
            .spawn((
                MeshMaterial3d::<UNodeMaterial3d>(Handle::default()),
                CachedUiContext {
                    root_entity: Some(root),
                    space: UiSpace::Screen,
                    ..default()
                },
                parent_stack,
            ))
            .id();

        use bevy::ecs::system::RunSystemOnce;
        world
            .run_system_once(
                |mut table: ResMut<World3dUiSortTable>,
                 root_transforms: Query<&GlobalTransform>,
                 ui_nodes: Query<
                    (Entity, &CachedUiContext, Option<&UiLocalStacking>),
                    With<MeshMaterial3d<UNodeMaterial3d>>,
                >,
                 text_children: Query<
                    (Entity, &ChildOf, Has<TextSelectionHighlightMarker>),
                    With<MeshMaterial3d<UTextLabelSdfMaterial3d>>,
                >,
                 parent_contexts: Query<(&CachedUiContext, Option<&UiLocalStacking>)>| {
                    collect_world3d_ui_sort_entries(
                        &mut table,
                        &root_transforms,
                        &ui_nodes,
                        &text_children,
                        &parent_contexts,
                    );
                },
            )
            .expect("collect_world3d_ui_sort_entries test system should run");

        let table = world.resource::<World3dUiSortTable>();
        assert!(!table.0.contains_key(&MainEntity::from(screen_node)));
        let e_node = table
            .0
            .get(&MainEntity::from(ui_node))
            .expect("World3d node must be in sort table");
        let e_hl = table
            .0
            .get(&MainEntity::from(highlight_child))
            .expect("World3d highlight child must be in sort table");
        let e_text = table
            .0
            .get(&MainEntity::from(text_child))
            .expect("World3d text child must be in sort table");

        assert_eq!(e_node.root_center, panel_origin);
        assert_eq!(e_hl.root_center, panel_origin);
        assert_eq!(e_text.root_center, panel_origin);
        assert!(e_node.sort_bias < e_hl.sort_bias);
        assert!(e_hl.sort_bias < e_text.sort_bias);
    }
}
