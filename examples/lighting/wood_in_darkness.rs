//! Non-Emissive Wood in Total Darkness Showcase
//!
//! Demonstrates physically realistic zero-ambient darkness (`GlobalAmbientLight::NONE`) where
//! pure dielectric / wood UI surfaces (`UPbr::wood()`, `UPbrSurface::wood_plank()`) and carved
//! wooden 3D typography (`UText`) emit ZERO light on their own and stay completely invisible until
//! struck by a real scene light beam:
//! - Sweep the flashlight across the pitch-black wall to uncover oak panels, cedar inlays, carved
//!   wood lettering (`"TIMBER & OAK"`), beveled edges, and cast shadows!
//! - Toggle the warm workshop lantern (`[L]`) to illuminate the full wooden structure, or switch
//!   back to total zero-ambient darkness where anything outside the cone vanishes completely.
//!
//! Controls:
//! - `Mouse Move` or `[W/A/S/D]` / Arrow keys: Aim the handheld flashlight beam across the wall
//! - `[Space]`: Toggle automatic searchlight sweep when hands-free
//! - `[L]`: Toggle overhead wooden workbench lantern

use std::f32::consts::FRAC_PI_2;

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct DarknessLabState {
    auto_sweep: bool,
    lantern_on: bool,
    aim_target: Vec2,
    time: f32,
}

impl Default for DarknessLabState {
    fn default() -> Self {
        Self {
            auto_sweep: true,
            lantern_on: false,
            aim_target: Vec2::ZERO,
            time: 0.0,
        }
    }
}

#[derive(Component)]
struct FlashlightSpot;

#[derive(Component)]
struct WorkshopLantern;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 06 - Wood in Darkness (Zero Ambient) | [Mouse/WASD] Aim  [Space] Sweep  [L] Lantern"
                    .into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        // Literal zero ambient light: non-emissive wood is pitch black until struck by light.
        .insert_resource(GlobalAmbientLight::NONE)
        .init_resource::<DarknessLabState>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (handle_controls, update_lights))
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
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_xyz(0.0, 0.06, 2.15).looking_at(Vec3::new(0.0, 0.0, -0.2), Vec3::Y),
    ));

    // Handheld Flashlight Spotlight (cast_shadows enabled so carved wood lettering casts shadows).
    commands.spawn((
        FlashlightSpot,
        SpotLight {
            color: Color::srgb(1.0, 0.90, 0.72),
            intensity: 420_000.0,
            range: 8.5,
            inner_angle: 0.16,
            outer_angle: 0.34,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, -0.10, 1.55).looking_at(Vec3::new(0.0, 0.0, -0.2), Vec3::Y),
    ));

    // Optional Workbench Lantern light (starts OFF so scene opens in pure darkness).
    commands.spawn((
        WorkshopLantern,
        ULightSource {
            enabled: false,
            kind: ULightSourceKind::Point { radius: 0.08 },
            color: Color::srgb(1.0, 0.76, 0.42),
            intensity: 140_000.0,
            range: 8.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.55, 1.25),
    ));

    // Matte dark timber workshop rear wall (also completely black in darkness until light strikes it).
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(9.0, 5.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.24, 0.16, 0.10),
            metallic: 0.0,
            perceptual_roughness: 0.88,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -0.45).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    ));

    // Wooden Showcase Frame — ZERO emissive glow everywhere; purely dielectric UPbr::wood()!
    let wood_wall = commands
        .spawn((
            URootUi::world_3d(Vec2::new(980.0, 560.0)),
            UNode {
                width: UVal::Px(980.0),
                height: UVal::Px(560.0),
                background_color: Color::srgb(0.40, 0.24, 0.12),
                border_radius: UCornerRadius::all(18.0),
                padding: USides::all(28.0),
                ..default()
            },
            UBorder {
                color: Color::srgb(0.24, 0.14, 0.07),
                width: 4.0,
                radius: UCornerRadius::all(18.0),
                ..default()
            },
            UPbr::wood(),
            UPbrSurface::wood_plank().with_bevel(8.0, 0.75),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 20.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.25),
        ))
        .id();

    // Raised carved wooden title in front of the wall so letters catch highlights & cast shadows!
    let foreground_text = commands
        .spawn((
            URootUi::world_3d(Vec2::new(820.0, 140.0)),
            UNode {
                width: UVal::Px(820.0),
                height: UVal::Px(140.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            Transform::from_xyz(0.0, 0.18, -0.05),
        ))
        .id();

    commands.spawn((
        ChildOf(foreground_text),
        UTextLabel {
            text: "TIMBER & OAK".into(),
            font: serif,
            font_size: 86.0,
            color: Color::srgb(0.68, 0.45, 0.24),
            ..default()
        },
        UNode::default(),
        UPbr::wood(),
        UPbrSurface::wood_plank().with_bevel(4.5, 0.75),
        ULightOccluder::solid(),
    ));

    // Top spacer on the back board corresponding to the carved title depth offset.
    commands.spawn((
        ChildOf(wood_wall),
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(110.0),
            background_color: Color::NONE,
            ..default()
        },
    ));

    // Row of 4 non-emissive timber planks (Walnut, Cedar, Birch, Slate Stone) with zero glow.
    let plank_row = commands
        .spawn((
            ChildOf(wood_wall),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(220.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Stretch,
                gap: 18.0,
                ..default()
            },
        ))
        .id();

    for (species, albedo, pbr) in [
        ("WALNUT", Color::srgb(0.28, 0.16, 0.09), UPbr::wood()),
        ("CEDAR", Color::srgb(0.56, 0.30, 0.15), UPbr::wood()),
        ("WHITE OAK", Color::srgb(0.76, 0.60, 0.40), UPbr::wood()),
        ("BASALT STONE", Color::srgb(0.22, 0.23, 0.25), UPbr::stone()),
    ] {
        let plank = commands
            .spawn((
                ChildOf(plank_row),
                UNode {
                    width: UVal::Px(215.0),
                    height: UVal::Percent(1.0),
                    background_color: albedo,
                    border_radius: UCornerRadius::all(12.0),
                    padding: USides::all(16.0),
                    ..default()
                },
                pbr,
                UPbrSurface::wood_plank().with_bevel(7.0, 0.7),
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    justify_content: UJustifyContent::SpaceBetween,
                    ..default()
                },
            ))
            .id();

        commands.spawn((
            ChildOf(plank),
            UTextLabel {
                text: species.into(),
                font: font.clone(),
                font_size: 16.0,
                color: Color::srgb(0.92, 0.86, 0.78),
                ..default()
            },
            UNode::default(),
            UPbr::wood(),
        ));

        commands.spawn((
            ChildOf(plank),
            UTextLabel {
                text: "Zero Emissive\nInvisible Without Light".into(),
                font: font.clone(),
                font_size: 12.0,
                color: Color::srgb(0.78, 0.70, 0.60),
                ..default()
            },
            UNode::default(),
            UPbr::wood(),
        ));
    }

    // Bottom plaque (non-emissive brushed brass inlay + instructions, also invisible in unlit shadow).
    let plaque = commands
        .spawn((
            ChildOf(wood_wall),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(96.0),
                background_color: Color::srgb(0.32, 0.20, 0.10),
                border_radius: UCornerRadius::all(12.0),
                padding: USides::all(16.0),
                ..default()
            },
            UPbr::wood(),
            UPbrSurface::wood_plank().with_bevel(5.0, 0.6),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(plaque),
        UTextLabel {
            text: "Sweep Beam ([Mouse] or [WASD/Arrows])  |  [Space] Auto Sweep  |  [L] Toggle Workbench Lantern"
                .into(),
            font,
            font_size: 14.0,
            color: Color::srgb(0.95, 0.88, 0.78),
            ..default()
        },
        UNode::default(),
        UPbr::matte(),
    ));
}

fn handle_controls(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cursor_events: MessageReader<CursorMoved>,
    mut state: ResMut<DarknessLabState>,
) {
    let dt = time.delta_secs();
    let speed = 1.15;
    let mut manual = false;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        state.aim_target.x -= speed * dt;
        manual = true;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        state.aim_target.x += speed * dt;
        manual = true;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        state.aim_target.y += speed * dt;
        manual = true;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        state.aim_target.y -= speed * dt;
        manual = true;
    }

    if let Some(moved) = cursor_events.read().last()
        && let Ok(win) = windows.single()
    {
        let w = win.width().max(1.0);
        let h = win.height().max(1.0);
        let nx = (moved.position.x / w) * 2.0 - 1.0;
        let ny = 1.0 - (moved.position.y / h) * 2.0;
        state.aim_target = Vec2::new(nx * 1.15, ny * 0.65);
        state.auto_sweep = false;
    }

    if manual {
        state.auto_sweep = false;
    }

    state.aim_target.x = state.aim_target.x.clamp(-1.25, 1.25);
    state.aim_target.y = state.aim_target.y.clamp(-0.70, 0.70);

    if keyboard.just_pressed(KeyCode::Space) {
        state.auto_sweep = !state.auto_sweep;
    }
    if keyboard.just_pressed(KeyCode::KeyL) {
        state.lantern_on = !state.lantern_on;
    }
}

fn update_lights(
    time: Res<Time>,
    mut state: ResMut<DarknessLabState>,
    mut spot_q: Query<&mut Transform, With<FlashlightSpot>>,
    mut lantern_q: Query<&mut ULightSource, With<WorkshopLantern>>,
) {
    if state.auto_sweep {
        state.time += time.delta_secs();
        state.aim_target.x = (state.time * 0.75).sin() * 0.92;
        state.aim_target.y = (state.time * 1.35).cos() * 0.36;
    }

    if let Ok(mut tf) = spot_q.single_mut() {
        let look = Vec3::new(state.aim_target.x, state.aim_target.y, -0.25);
        *tf = Transform::from_xyz(state.aim_target.x * 0.15, -0.10, 1.55).looking_at(look, Vec3::Y);
    }

    if let Ok(mut lantern) = lantern_q.single_mut() {
        lantern.enabled = state.lantern_on;
    }
}
