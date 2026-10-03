//! Luma is a modern, high-performance SDF UI framework for the Bevy game engine.
//!
//! It provides:
//! - Pure GPU Signed Distance Field (SDF) anti-aliased rendering (rounded corners, borders, shadows, glow).
//! - Flexible layout solving (Flexbox, CSS Grid, Absolute, Radial) with no external Web-engine baggage.
//! - Seamless 2D Screen and 3D In-World spatial UI with interactive picking and focus navigation.
//! - Embedded typography with multi-script font fallback (Latin, Arabic, etc.) and Lucide icons.
//!
//! Import [`prelude`] to start building responsive UI in Bevy.

use bevy::prelude::*;
use luma_engine::LumaEnginePlugin;
use luma_interaction::interaction::LumaInteractionPlugin;
use luma_style::style::LumaStylePlugin;

pub use luma_engine as engine_crate;
pub use luma_interaction as interaction_crate;
pub use luma_layout as layout_crate;
pub use luma_style as style_crate;

/// Curated engine access for applications that need more than the recommended facade prelude.
pub mod engine {
    pub use luma_engine::{LumaEnginePlugin, UnivisEnginePlugin, layout, prelude, schedule};
}

/// Advanced style access, including fonts, icons, and the shared theme resource.
pub mod style {
    pub use luma_style::{prelude, style};
}

/// Advanced interaction access, including picking and interaction state types.
pub mod interaction {
    pub use luma_interaction::{interaction, prelude};
}

/// Advanced render-facing access for material and mesh integration work.
pub mod render {
    pub use luma_engine::layout::render::{
        UnivisRenderPlugin, material, material_3d, prelude,
    };
}

/// Advanced layout access for roots, node primitives, text, and layout-specific systems.
pub mod layout {
    pub use luma_engine::layout::{
        LumaLayoutPlugin, UnivisLayoutPlugin, geometry, image, invalidation, layout_system, pbr,
        profiling, query, render, text, transition, univis_node, visual,
    };

    /// Common layout-facing imports such as roots, nodes, and geometry helpers.
    pub mod prelude {
        pub use luma_engine::layout::prelude::*;
    }
}

/// The recommended import surface for most applications.
///
/// This bundles the facade plugin plus the core public types from engine,
/// style, and interaction.
pub mod prelude {
    pub use crate::{LumaPlugin, UnivisUiPlugin};
    pub use luma_engine::prelude::*;
    pub use luma_interaction::prelude::*;
    pub use luma_style::prelude::*;
}

/// Registers the complete Luma UI stack: style, engine, and interaction.
///
/// # Example
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_luma::prelude::*;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(LumaPlugin);
/// ```
pub struct LumaPlugin;
pub type UnivisUiPlugin = LumaPlugin;

impl Plugin for LumaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LumaStylePlugin)
            .add_plugins(LumaEnginePlugin)
            .add_plugins(LumaInteractionPlugin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use bevy::{asset::AssetPlugin, text::TextPlugin};
    use luma_engine::schedule::{UiRolloutConfig, UiValidationState};

    fn public_plugin_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), TextPlugin));
        app
    }

    #[test]
    fn facade_prelude_exposes_canonical_surface() {
        let _root = URootUi::screen();
        let _node = UNode::default();
        let _padding = USides::all(12.0);
        let _text = UText::default();
        let _label = UTextLabel::default();
        let _interaction = UInteraction::default();
        let _colors = UInteractionColors::default();
    }

    #[test]
    fn facade_plugin_registers_engine_resources() {
        let mut app = public_plugin_test_app();
        app.add_plugins(LumaPlugin);

        assert!(app.world().contains_resource::<UiRolloutConfig>());
        assert!(app.world().contains_resource::<UiValidationState>());
    }

    #[test]
    fn direct_plugin_combo_registers_engine_resources() {
        let mut app = public_plugin_test_app();
        app.add_plugins((
            LumaStylePlugin,
            LumaEnginePlugin,
            LumaInteractionPlugin,
        ));

        assert!(app.world().contains_resource::<UiRolloutConfig>());
        assert!(app.world().contains_resource::<UiValidationState>());
    }
}
