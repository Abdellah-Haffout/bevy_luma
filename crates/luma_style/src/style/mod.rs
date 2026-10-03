//! Embedded fonts, icons, and theme resources for Luma.

use bevy::{asset::embedded_asset, prelude::*};

/// Built-in icon font assets and helpers.
pub mod icons;

/// Common style imports, including [`Theme`] and the built-in icons.
pub mod prelude {
    pub use crate::style::Theme;
    pub use crate::style::icons::*;
    pub use crate::style::{LumaStylePlugin, UnivisUiStylePlugin};
}

/// Registers embedded style assets and initializes the shared [`Theme`] resource.
///
/// # Example
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use luma_style::style::LumaStylePlugin;
///
/// App::new()
///     .add_plugins(MinimalPlugins)
///     .add_plugins(LumaStylePlugin);
/// ```
pub struct LumaStylePlugin;
pub type UnivisUiStylePlugin = LumaStylePlugin;

impl Plugin for LumaStylePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets/fonts/Inter-Regular.ttf");
        embedded_asset!(app, "assets/fonts/AdwaitaSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FiraSans-Regular.ttf");
        embedded_asset!(app, "assets/fonts/NotoSansArabic-Regular.ttf");
        embedded_asset!(app, "assets/fonts/FreeSerif.otf");
        embedded_asset!(app, "assets/icons/Lucide.ttf");
        app.init_resource::<Theme>();
    }
}

/// Shared style resource exposed to widgets and application code.
///
/// # Example
///
/// ```rust,no_run
/// use bevy::prelude::*;
/// use luma_style::style::Theme;
///
/// fn read_theme(theme: Res<Theme>) {
///     let _body_font = theme.text.font.inter_regular.clone();
///     let _icon_font = theme.icon.font.clone();
/// }
/// ```
#[derive(Resource)]
pub struct Theme {
    /// Text-oriented style handles.
    pub text: TextStyles,
    /// Icon-oriented style handles.
    pub icon: IconStyles,
}

/// Group of text-related style handles.
pub struct TextStyles {
    /// Embedded body-font handles.
    pub font: Fonts,
}

/// Group of icon-related style handles.
pub struct IconStyles {
    /// Embedded icon font handle.
    pub font: Handle<Font>,
}

/// Built-in font handles embedded by [`LumaStylePlugin`].
pub struct Fonts {
    /// Embedded Inter Regular handle.
    pub inter_regular: Handle<Font>,
    /// Embedded Adwaita Sans Regular handle.
    pub adwaita_sans_regular: Handle<Font>,
    /// Embedded Fira Sans Regular handle.
    pub fira_sans_regular: Handle<Font>,
    /// Embedded Noto Sans Arabic Regular handle.
    pub noto_sans_arabic_regular: Handle<Font>,
    /// Embedded FreeSerif handle with broad Arabic + Latin coverage.
    pub free_serif: Handle<Font>,
}

impl FromWorld for Theme {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Theme {
            text: TextStyles {
                font: Fonts {
                    inter_regular: asset_server
                        .load("embedded://luma_style/style/assets/fonts/Inter-Regular.ttf"),
                    adwaita_sans_regular: asset_server.load(
                        "embedded://luma_style/style/assets/fonts/AdwaitaSans-Regular.ttf",
                    ),
                    fira_sans_regular: asset_server
                        .load("embedded://luma_style/style/assets/fonts/FiraSans-Regular.ttf"),
                    noto_sans_arabic_regular: asset_server.load(
                        "embedded://luma_style/style/assets/fonts/NotoSansArabic-Regular.ttf",
                    ),
                    free_serif: asset_server
                        .load("embedded://luma_style/style/assets/fonts/FreeSerif.otf"),
                },
            },
            icon: IconStyles {
                font: asset_server.load("embedded://luma_style/style/assets/icons/Lucide.ttf"),
            },
        }
    }
}
