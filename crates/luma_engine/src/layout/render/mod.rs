use bevy::{asset::embedded_asset, prelude::*, sprite_render::Material2dPlugin};

pub(crate) mod depth_sync;

/// Single-draw instanced batch renderer for 2D SDF UI nodes.
pub mod batch;
/// Hardware SDF clipping synchronization and shared geometry helpers.
pub mod clip_sync;
/// Hierarchical [`RenderLayers`] propagation across UI roots and generated children.
pub mod layers;
/// Material definitions for the Univis UI engine.
pub mod material;
/// 3D material definitions for the Univis UI engine.
pub mod material_3d;
/// Systems for synchronizing UI node state to render materials and meshes.
pub mod system;

use crate::layout::components::UI3d;
use crate::layout::layout_system::sync_cached_ui3d;
use crate::layout::pbr::UPbr;
use crate::layout::render::batch::UnivisBatchPlugin;
use crate::layout::render::material::UNodeMaterial;
use crate::layout::render::material_3d::UNodeMaterial3d;
use crate::layout::render::system::{
    MaterialPool, MeshPool, UShaderCache, sync_animated_ushaders, update_materials_optimized,
};
use crate::schedule::{UiSettlementSchedule, UnivisPostUpdateSet};

/// Convenience exports for Univis UI rendering.
pub mod prelude {
    pub use crate::layout::render::{
        UnivisRenderPlugin, batch::*, clip_sync::*, material::*, material_3d::*, system::*,
    };
}

/// Core rendering plugin for Univis UI materials and mesh synchronization.
pub struct UnivisRenderPlugin;

impl Plugin for UnivisRenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/unode.wgsl");
        embedded_asset!(app, "shaders/unode_batch.wgsl");
        embedded_asset!(app, "shaders/unode_3d.wesl");
        embedded_asset!(app, "shaders/text_label_sdf.wgsl");
        embedded_asset!(app, "shaders/text_label_sdf_3d.wesl");

        app.add_plugins(UnivisBatchPlugin)
            .add_plugins(Material2dPlugin::<UNodeMaterial>::default())
            .add_plugins(MaterialPlugin::<UNodeMaterial3d>::default());
        depth_sync::install_material2d_depth_sync::<UNodeMaterial>(app);
        depth_sync::install_material3d_transform_sync::<UNodeMaterial3d>(app);
        depth_sync::install_world3d_transparent_sort_sync(app);
        app.register_type::<UI3d>()
            .register_type::<UPbr>()
            .register_type::<clip_sync::UActiveClip>()
            .init_resource::<MaterialPool>()
            .init_resource::<MeshPool>()
            .init_resource::<UShaderCache>()
            .add_systems(Update, sync_animated_ushaders)
            .add_systems(
                UiSettlementSchedule,
                sync_cached_ui3d
                    .in_set(UnivisPostUpdateSet::RenderSync)
                    .after(UnivisPostUpdateSet::LayoutSolve),
            )
            .add_systems(
                UiSettlementSchedule,
                update_materials_optimized
                    .in_set(UnivisPostUpdateSet::RenderSync)
                    .after(sync_cached_ui3d),
            );
    }
}
