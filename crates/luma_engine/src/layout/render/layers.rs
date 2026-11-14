//! Hierarchical [`RenderLayers`] propagation across resolved UI roots and subtrees.
//!
//! Propagates `RenderLayers` from each [`ResolvedRootUi`]'s target camera (or explicit
//! root/node overrides) down `Children` so spawned SDF nodes, `TextChildMarker`,
//! `TextSelectionHighlightMarker`, and custom child meshes render onto the right camera layers.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::layout::layout_system::ResolvedRootUi;

/// Tracks that an entity's [`RenderLayers`] component was inherited by UI propagation,
/// distinguishing propagated state from explicit user-authored `RenderLayers` overrides.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct InheritedUiRenderLayers(pub RenderLayers);

/// Propagates [`RenderLayers`] from cameras -> UI roots -> descendant entities while
/// preserving explicit per-root or per-node `RenderLayers` overrides.
pub(crate) fn sync_ui_render_layers(
    mut commands: Commands,
    explicit_sources: Query<(), (With<RenderLayers>, Without<InheritedUiRenderLayers>)>,
    inherited_entities: Query<(), With<InheritedUiRenderLayers>>,
    roots: Query<(&ResolvedRootUi, Option<&Children>)>,
    cameras: Query<&RenderLayers, Without<InheritedUiRenderLayers>>,
    nodes: Query<(
        Option<&RenderLayers>,
        Option<&InheritedUiRenderLayers>,
        Option<&Children>,
    )>,
) {
    // O(1) fast path when neither cameras nor UI entities have non-default RenderLayers.
    if explicit_sources.is_empty() && inherited_entities.is_empty() {
        return;
    }

    for (resolved, children) in roots.iter() {
        let camera_layers = resolved
            .camera_entity
            .and_then(|cam| cameras.get(cam).ok());

        let Some(effective_root_layers) = sync_entity_render_layers(
            resolved.root_entity,
            camera_layers,
            &nodes,
            &mut commands,
        ) else {
            propagate_children(children, None, &nodes, &mut commands);
            continue;
        };

        propagate_children(children, Some(effective_root_layers), &nodes, &mut commands);
    }
}

fn propagate_children(
    children: Option<&Children>,
    inherited_layers: Option<&RenderLayers>,
    nodes: &Query<(
        Option<&RenderLayers>,
        Option<&InheritedUiRenderLayers>,
        Option<&Children>,
    )>,
    commands: &mut Commands,
) {
    let Some(children) = children else {
        return;
    };
    for child in children.iter() {
        let effective = sync_entity_render_layers(child, inherited_layers, nodes, commands);
        let child_children = nodes.get(child).ok().and_then(|(_, _, ch)| ch);
        propagate_children(child_children, effective, nodes, commands);
    }
}

fn sync_entity_render_layers<'a>(
    entity: Entity,
    parent_layers: Option<&'a RenderLayers>,
    nodes: &'a Query<(
        Option<&RenderLayers>,
        Option<&InheritedUiRenderLayers>,
        Option<&Children>,
    )>,
    commands: &mut Commands,
) -> Option<&'a RenderLayers> {
    let Ok((current_layers, inherited_marker, _)) = nodes.get(entity) else {
        return parent_layers;
    };

    let has_explicit_override = current_layers
        .is_some_and(|current| inherited_marker.is_none_or(|marker| &marker.0 != current));

    if has_explicit_override {
        if inherited_marker.is_some() {
            commands.entity(entity).remove::<InheritedUiRenderLayers>();
        }
        return current_layers;
    }

    match parent_layers {
        Some(target) => {
            let needs_update = current_layers != Some(target)
                || inherited_marker.map(|m| &m.0) != Some(target);
            if needs_update {
                commands.entity(entity).insert((
                    target.clone(),
                    InheritedUiRenderLayers(target.clone()),
                ));
            }
            Some(target)
        }
        None => {
            if inherited_marker.is_some() {
                commands
                    .entity(entity)
                    .remove::<(RenderLayers, InheritedUiRenderLayers)>();
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::geometry::UVal;
    use crate::layout::layout_system::{URootUi, UiCameraRef};
    use crate::layout::text::{
        TextChildMarker, TextSelectionHighlightMarker, USelectableText, UTextLabel,
    };
    use crate::layout::univis_node::UNode;

    #[test]
    fn camera_render_layers_propagate_to_ui_nodes_text_and_selection_highlights() {
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

        let font = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(
                include_bytes!(
                    "../../../../luma_style/src/style/assets/fonts/Inter-Regular.ttf"
                )
                .to_vec(),
            ));

        let cam_layers = RenderLayers::layer(3);
        let camera = app
            .world_mut()
            .spawn((Camera2d, cam_layers.clone()))
            .id();

        let root = app
            .world_mut()
            .spawn((
                URootUi {
                    camera: UiCameraRef::Entity(camera),
                    ..URootUi::screen()
                },
                UNode {
                    width: UVal::Percent(1.0),
                    height: UVal::Percent(1.0),
                    ..default()
                },
            ))
            .id();

        let panel = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    width: UVal::Px(300.0),
                    height: UVal::Px(120.0),
                    ..default()
                },
            ))
            .id();

        let label = app
            .world_mut()
            .spawn((
                ChildOf(panel),
                UNode::default(),
                UTextLabel {
                    text: "Hello RenderLayers".to_string(),
                    font,
                    ..default()
                },
                USelectableText::new().with_selection(0, 5),
            ))
            .id();

        // Override branch on a sibling panel to verify overrides isolate cleanly.
        let override_layers = RenderLayers::layer(7);
        let custom_panel = app
            .world_mut()
            .spawn((
                ChildOf(root),
                UNode {
                    width: UVal::Px(120.0),
                    height: UVal::Px(60.0),
                    ..default()
                },
                override_layers.clone(),
            ))
            .id();
        let custom_child = app
            .world_mut()
            .spawn((ChildOf(custom_panel), UNode::default()))
            .id();

        app.update();
        app.update();

        assert_eq!(app.world().entity(root).get::<RenderLayers>(), Some(&cam_layers));
        assert_eq!(
            app.world().entity(panel).get::<RenderLayers>(),
            Some(&cam_layers)
        );
        assert_eq!(
            app.world().entity(label).get::<RenderLayers>(),
            Some(&cam_layers)
        );

        let text_child = app
            .world()
            .entity(label)
            .get::<Children>()
            .and_then(|c| c.iter().find(|&e| app.world().entity(e).contains::<TextChildMarker>()))
            .expect("label should spawn TextChildMarker child");
        assert_eq!(
            app.world().entity(text_child).get::<RenderLayers>(),
            Some(&cam_layers)
        );

        let selection_child = app
            .world()
            .entity(label)
            .get::<Children>()
            .and_then(|c| {
                c.iter()
                    .find(|&e| app.world().entity(e).contains::<TextSelectionHighlightMarker>())
            })
            .expect("selected label should spawn TextSelectionHighlightMarker child");
        assert_eq!(
            app.world().entity(selection_child).get::<RenderLayers>(),
            Some(&cam_layers)
        );

        // Explicit override must remain intact on override subtree.
        assert_eq!(
            app.world().entity(custom_panel).get::<RenderLayers>(),
            Some(&override_layers)
        );
        assert_eq!(
            app.world().entity(custom_child).get::<RenderLayers>(),
            Some(&override_layers)
        );

        // Updating camera layers updates inherited nodes while preserving explicit override.
        let updated_layers = RenderLayers::layer(5);
        app.world_mut()
            .entity_mut(camera)
            .insert(updated_layers.clone());
        app.update();

        assert_eq!(
            app.world().entity(panel).get::<RenderLayers>(),
            Some(&updated_layers)
        );
        assert_eq!(
            app.world().entity(text_child).get::<RenderLayers>(),
            Some(&updated_layers)
        );
        assert_eq!(
            app.world().entity(custom_child).get::<RenderLayers>(),
            Some(&override_layers)
        );

        // Removing camera layers removes inherited RenderLayers cleanly while preserving explicit override.
        app.world_mut().entity_mut(camera).remove::<RenderLayers>();
        app.update();

        assert_eq!(app.world().entity(root).get::<RenderLayers>(), None);
        assert_eq!(app.world().entity(panel).get::<RenderLayers>(), None);
        assert_eq!(app.world().entity(text_child).get::<RenderLayers>(), None);
        assert_eq!(
            app.world().entity(custom_child).get::<RenderLayers>(),
            Some(&override_layers)
        );
    }
}
