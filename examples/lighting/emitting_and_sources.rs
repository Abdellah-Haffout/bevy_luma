//! Emitting Light & UI/Text Light Sources Showcase
//!
//! Demonstrates turning BOTH `UText` and `UNode` entities into physical 3D scene lights:
//! - Self-illuminated HDR lamp & neon materials (`UPbr::lamp`, `UPbr::neon`) with `Bloom`.
//! - `ULightSource::lamp(...)` (true `RectLight` area light synced with UI dimensions).
//! - `ULightSource::bulb(...)` (`PointLight` orb synced with a node badge).
//! - `ULightSource::spotlight(...)` (`SpotLight` projected straight from a `UText` signage element).
//! - Live dimmer & switches turning both physical scene emission AND material HDR glow on/off together.
//!
//! Controls:
//! - `[1]`: Toggle `UText` Neon Sign RectLight source
//! - `[2]`: Toggle `UNode` Studio Softbox Ceiling RectLight source
//! - `[3]`: Toggle `UText` Spot Projector light source
//! - `[4]`: Toggle `UNode` Warm Desk Point Bulb source
//! - `[Up / Down]`: Adjust master dimmer across all UI lamps

use std::f32::consts::FRAC_PI_2;

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct EmitterConsoleState {
    sign_on: bool,
    softbox_on: bool,
    spot_text_on: bool,
    bulb_node_on: bool,
    dimmer: f32,
}

impl Default for EmitterConsoleState {
    fn default() -> Self {
        Self {
            sign_on: true,
            softbox_on: true,
            spot_text_on: true,
            bulb_node_on: true,
            dimmer: 1.0,
        }
    }
}

#[derive(Component, Clone, Copy)]
enum LampChannel {
    TextSign,
    NodeSoftbox,
    TextSpotlight,
    NodeBulb,
}

#[derive(Component)]
struct SpinArtifact;

#[derive(Component)]
struct ConsoleHudLabel;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 02 - Emitting Light & UI/Text as Light Sources".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        // Very low ambient so illumination visibly comes from UText & UNode lamps.
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.3, 0.35, 0.45),
            brightness: 10.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<EmitterConsoleState>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (handle_input, sync_lamps, spin_artifacts))
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    theme: Res<Theme>,
) {
    let font = theme.text.font.inter_regular.clone();
    let serif = theme.text.font.free_serif.clone();

    commands.spawn((
        Camera3d::default(),
        Hdr,
        Bloom::NATURAL,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.01, 0.012, 0.018)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.45, 2.35).looking_at(Vec3::new(0.0, 0.05, 0.0), Vec3::Y),
    ));

    // Matte studio room: floor and rear wall (lit purely by our UI nodes & text).
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 8.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.24, 0.24, 0.28),
            perceptual_roughness: 0.72,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.58, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.18, 0.22),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.8, -0.9).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    ));

    // Metallic centerpiece sphere + wooden plinth below the UI emitters.
    commands.spawn((
        SpinArtifact,
        Mesh3d(meshes.add(Cuboid::new(0.28, 0.28, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.88, 0.94),
            metallic: 0.92,
            perceptual_roughness: 0.16,
            ..default()
        })),
        Transform::from_xyz(-0.35, -0.18, 0.22),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.18))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.76, 0.62),
            metallic: 0.45,
            perceptual_roughness: 0.30,
            ..default()
        })),
        Transform::from_xyz(0.38, -0.30, 0.20),
    ));

    // 1. Floating World3d UI Board acting as light fixture rack + wooden receiver strip.
    let fixture_root = commands
        .spawn((
            URootUi::world_3d(Vec2::new(980.0, 560.0)),
            UNode {
                width: UVal::Px(980.0),
                height: UVal::Px(560.0),
                background_color: Color::srgba(0.04, 0.05, 0.08, 0.72),
                border_radius: UCornerRadius::all(20.0),
                padding: USides::all(24.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.35, 0.45, 0.65, 0.35),
                width: 1.5,
                radius: UCornerRadius::all(20.0),
                ..default()
            },
            UPbr::matte(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 18.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.22, -0.25),
        ))
        .id();

    // Top row: UNode Studio Softbox (RectLight) + Console status.
    let top_row = commands
        .spawn((
            ChildOf(fixture_root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(110.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                gap: 20.0,
                ..default()
            },
        ))
        .id();

    // A pure UNode panel behaving as a warm daylight softbox RectLight.
    let softbox = commands
        .spawn((
            ChildOf(top_row),
            LampChannel::NodeSoftbox,
            UNode {
                width: UVal::Px(340.0),
                height: UVal::Px(96.0),
                background_color: Color::srgb(1.0, 0.95, 0.86),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::all(14.0),
                ..default()
            },
            UPbr::lamp(Color::srgb(1.0, 0.92, 0.80), 4.5),
            ULightSource::lamp(Color::srgb(1.0, 0.92, 0.80), 22_000.0)
                .with_range(6.0)
                .with_offset(Vec3::new(0.0, 0.0, 0.06)),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(softbox),
        UTextLabel {
            text: "UNODE SOFTBOX LAMP (RectLight)".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.18, 0.14, 0.08),
            ..default()
        },
        UNode::default(),
    ));

    // Console info box.
    let info_box = commands
        .spawn((
            ChildOf(top_row),
            UNode {
                width: UVal::Px(540.0),
                height: UVal::Px(96.0),
                background_color: Color::srgba(0.14, 0.11, 0.08, 0.95),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::all(14.0),
                ..default()
            },
            UPbr::wood(),
            UPbrSurface::wood_plank(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::Center,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(info_box),
        ConsoleHudLabel,
        UTextLabel {
            text: "[1] Text Neon ON  [2] Node Softbox ON  [3] Text Spot ON  [4] Node Bulb ON  (Dimmer 100%)".into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.95, 0.90, 0.82),
            ..default()
        },
        UNode::default(),
        UPbr::matte(),
    ));

    // Middle row: UText ITSELF as a glowing Cyan Neon RectLight AND UText as an Amber SpotLight!
    let mid_row = commands
        .spawn((
            ChildOf(fixture_root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(190.0),
                background_color: Color::srgba(0.03, 0.04, 0.06, 0.8),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 14.0),
                ..default()
            },
            UPbr::matte(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceAround,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    // UText Sign casting real Cyan RectLight out of its letters!
    commands.spawn((
        ChildOf(mid_row),
        LampChannel::TextSign,
        UTextLabel {
            text: "NEON TEXT LAMP".into(),
            font: serif.clone(),
            font_size: 46.0,
            color: Color::srgb(0.20, 0.92, 1.0),
            ..default()
        },
        UNode::default(),
        UPbr::neon(Color::srgb(0.20, 0.92, 1.0), 6.0),
        ULightSource::lamp(Color::srgb(0.20, 0.92, 1.0), 32_000.0)
            .with_range(7.0)
            .with_offset(Vec3::new(0.0, 0.0, 0.08)),
    ));

    // UText Emitter acting as a focused Magenta/Amber SpotLight onto the front metallic cube!
    commands.spawn((
        ChildOf(mid_row),
        LampChannel::TextSpotlight,
        UTextLabel {
            text: "BEAM TEXT".into(),
            font: serif,
            font_size: 40.0,
            color: Color::srgb(1.0, 0.35, 0.72),
            ..default()
        },
        UNode::default(),
        UPbr::lamp(Color::srgb(1.0, 0.35, 0.72), 6.5),
        ULightSource::spotlight(Color::srgb(1.0, 0.35, 0.72), 180_000.0, 7.0, 0.48)
            .with_shadows(true)
            .with_offset(Vec3::new(0.0, 0.0, 0.08)),
    ));

    // Bottom row: Warm UNode Point Bulb Orb + Wooden shelf receiving light from all sources.
    let bot_row = commands
        .spawn((
            ChildOf(fixture_root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(130.0),
                background_color: Color::srgb(0.45, 0.27, 0.14),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 14.0),
                ..default()
            },
            UPbr::wood(),
            UPbrSurface::wood_plank(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(bot_row),
        UTextLabel {
            text: "Non-emissive Oak Wood Shelf lit exclusively by the UNode & UText lamps above."
                .into(),
            font: font.clone(),
            font_size: 16.0,
            color: Color::srgb(0.95, 0.88, 0.78),
            ..default()
        },
        UNode::default(),
        UPbr::matte(),
    ));

    let bulb = commands
        .spawn((
            ChildOf(bot_row),
            LampChannel::NodeBulb,
            UNode {
                width: UVal::Px(82.0),
                height: UVal::Px(82.0),
                background_color: Color::srgb(1.0, 0.72, 0.25),
                border_radius: UCornerRadius::all(41.0),
                ..default()
            },
            UPbr::lamp(Color::srgb(1.0, 0.72, 0.25), 7.5),
            ULightSource::bulb(Color::srgb(1.0, 0.72, 0.25), 55_000.0, 6.0)
                .with_offset(Vec3::new(0.0, 0.0, 0.06)),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(bulb),
        UTextLabel {
            text: "BULB".into(),
            font,
            font_size: 14.0,
            color: Color::srgb(0.25, 0.12, 0.02),
            ..default()
        },
        UNode::default(),
    ));
}

fn handle_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<EmitterConsoleState>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        state.sign_on = !state.sign_on;
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        state.softbox_on = !state.softbox_on;
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        state.spot_text_on = !state.spot_text_on;
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        state.bulb_node_on = !state.bulb_node_on;
    }
    let dt = time.delta_secs();
    if keyboard.pressed(KeyCode::ArrowUp) {
        state.dimmer = (state.dimmer + 0.75 * dt).clamp(0.0, 2.0);
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        state.dimmer = (state.dimmer - 0.75 * dt).clamp(0.0, 2.0);
    }
}

fn sync_lamps(
    state: Res<EmitterConsoleState>,
    mut sources: Query<(&LampChannel, &mut ULightSource)>,
    mut hud: Query<&mut UTextLabel, With<ConsoleHudLabel>>,
) {
    if !state.is_changed() {
        return;
    }

    for (channel, mut src) in sources.iter_mut() {
        let on = match channel {
            LampChannel::TextSign => state.sign_on,
            LampChannel::NodeSoftbox => state.softbox_on,
            LampChannel::TextSpotlight => state.spot_text_on,
            LampChannel::NodeBulb => state.bulb_node_on,
        };
        src.enabled = on;
        src.dimmer = state.dimmer;
    }

    if let Ok(mut label) = hud.single_mut() {
        let s = |b: bool| if b { "ON" } else { "OFF" };
        label.text = format!(
            "[1] Text Neon {}  [2] Node Softbox {}  [3] Text Spot {}  [4] Node Bulb {}  (Dimmer {:.0}%)",
            s(state.sign_on),
            s(state.softbox_on),
            s(state.spot_text_on),
            s(state.bulb_node_on),
            state.dimmer * 100.0
        );
    }
}

fn spin_artifacts(time: Res<Time>, mut query: Query<&mut Transform, With<SpinArtifact>>) {
    let dt = time.delta_secs();
    for mut tf in query.iter_mut() {
        tf.rotate_y(0.65 * dt);
        tf.rotate_x(0.32 * dt);
    }
}
