//! Receiving Light Showcase (3D Nodes & Typography)
//!
//! Demonstrates physically based light reception across `UNode` panels and `UText`:
//! - Material presets (`matte`, `wood`, `stone`, `brushed_metal`, `chrome`, `gold`, `glass`).
//! - Procedural SDF edge bevel normals (`UPbrSurface::beveled`) catching specular highlights.
//! - Real-time shadow receiving on 3D UI cards & text (`receive_shadows: true`).
//! - Double-sided front/back normal flipping when orbiting around the display.
//!
//! Controls:
//! - `[Space]`: Pause/resume orbiting lights & shadow caster
//! - `[B]`: Toggle SDF bevel normals
//! - `[R]`: Toggle real-time shadow receiving on cards
//! - `[C]`: Swing camera to inspect the back side (double-sided lighting)

use std::f32::consts::PI;

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct ShowcaseState {
    animating: bool,
    bevel_enabled: bool,
    receive_shadows: bool,
    back_view: bool,
    time: f32,
}

impl Default for ShowcaseState {
    fn default() -> Self {
        Self {
            animating: true,
            bevel_enabled: true,
            receive_shadows: true,
            back_view: false,
            time: 0.0,
        }
    }
}

#[derive(Component)]
struct OrbitingOrb {
    phase: f32,
    radius_x: f32,
    radius_z: f32,
    height: f32,
}

#[derive(Component)]
struct ShadowCastingOrbit;

#[derive(Component)]
struct MaterialSwatchCard;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct StatusHudText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 01 - Receiving Light (Materials, Bevels & Shadows)".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.55, 0.65, 0.85),
            brightness: 45.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<ShowcaseState>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                handle_controls,
                animate_orbits,
                update_camera_and_surfaces,
            ),
        )
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
        MainCamera,
        Camera3d::default(),
        Hdr,
        Bloom::NATURAL,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.025, 0.03, 0.045)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.28, 1.85).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // Warm key spotlight casting real shadows onto the 3D UI gallery board.
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 0.92, 0.80),
            intensity: 260_000.0,
            range: 8.0,
            outer_angle: 0.68,
            inner_angle: 0.35,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-0.95, 1.05, 1.45).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Orbiting cyan and amber point lights with glowing cores.
    for (phase, color, height) in [
        (0.0, Color::srgb(0.25, 0.88, 1.0), 0.18),
        (PI, Color::srgb(1.0, 0.62, 0.22), -0.15),
    ] {
        commands.spawn((
            OrbitingOrb {
                phase,
                radius_x: 1.05,
                radius_z: 0.72,
                height,
            },
            PointLight {
                color,
                intensity: 110_000.0,
                range: 5.0,
                radius: 0.04,
                ..default()
            },
            Mesh3d(meshes.add(Sphere::new(0.038))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::from(color) * 9.0,
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.8, height, 0.5),
        ));
    }

    // Orbiting 3D torus knot casting moving real-time shadows onto the beveled UI cards.
    commands.spawn((
        ShadowCastingOrbit,
        Mesh3d(meshes.add(Torus::new(0.055, 0.13))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.78, 0.85),
            metallic: 0.8,
            perceptual_roughness: 0.25,
            ..default()
        })),
        Transform::from_xyz(-0.28, 0.24, 0.48),
    ));

    // Studio floor receiving shadows behind/below the panel.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 8.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.09, 0.12),
            perceptual_roughness: 0.65,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.52, 0.0),
    ));

    // Main 3D UI Gallery Board.
    let board = commands
        .spawn((
            URootUi::world_3d(Vec2::new(980.0, 580.0)),
            UNode {
                width: UVal::Px(980.0),
                height: UVal::Px(580.0),
                background_color: Color::srgba(0.07, 0.085, 0.12, 0.92),
                border_radius: UCornerRadius::all(22.0),
                padding: USides::all(26.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.5, 0.7, 0.95, 0.35),
                width: 2.0,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UPbr::stone(),
            UPbrSurface::beveled(8.0, 0.55),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 18.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.04, 0.0),
        ))
        .id();

    // Header row with metallic lit 3D UText.
    let header = commands
        .spawn((
            ChildOf(board),
            UNode {
                width: UVal::Percent(1.0),
                padding: USides::axes(16.0, 10.0),
                background_color: Color::srgba(0.11, 0.13, 0.19, 0.85),
                border_radius: UCornerRadius::all(12.0),
                ..default()
            },
            UPbr::brushed_metal(),
            UPbrSurface::beveled(5.0, 0.6),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(header),
        UTextLabel {
            text: "PHYSICALLY LIT 3D UI & TYPOGRAPHY".into(),
            font: serif.clone(),
            font_size: 24.0,
            color: Color::srgb(0.95, 0.82, 0.45),
            ..default()
        },
        UNode::default(),
        UPbr::gold(),
        UPbrSurface::beveled(4.5, 0.85),
    ));

    commands.spawn((
        ChildOf(header),
        StatusHudText,
        UTextLabel {
            text: "BEVEL: ON  |  SHADOWS: ON  |  VIEW: FRONT".into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.75, 0.86, 0.98),
            ..default()
        },
        UNode::default(),
    ));

    // Material Swatch Grid (6 physically distinct materials + lit UText labels inside each).
    let grid = commands
        .spawn((
            ChildOf(board),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(370.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                    UTrackSize::Fr(1.0),
                ],
                gap: 16.0,
                ..default()
            },
        ))
        .id();

    let swatches = [
        (
            "MATTE",
            "Roughness 0.92  Metallic 0.0",
            Color::srgb(0.76, 0.30, 0.28),
            UPbr::matte(),
            UPbrSurface::beveled(7.0, 0.8),
        ),
        (
            "OAK WOOD",
            "Warm diffuse grain, zero emit",
            Color::srgb(0.55, 0.34, 0.16),
            UPbr::wood(),
            UPbrSurface::wood_plank(),
        ),
        (
            "BRUSHED STEEL",
            "Metallic 0.85  Roughness 0.34",
            Color::srgb(0.62, 0.68, 0.76),
            UPbr::brushed_metal(),
            UPbrSurface::beveled(8.0, 0.85),
        ),
        (
            "MIRROR CHROME",
            "Metallic 1.0  Roughness 0.08",
            Color::srgb(0.92, 0.95, 1.0),
            UPbr::chrome(),
            UPbrSurface::beveled(8.0, 0.95),
        ),
        (
            "POLISHED GOLD",
            "Metallic 1.0  Roughness 0.16",
            Color::srgb(0.96, 0.74, 0.24),
            UPbr::gold(),
            UPbrSurface::beveled(8.0, 0.9),
        ),
        (
            "OPTICAL GLASS",
            "Translucent dielectric Fresnel",
            Color::srgba(0.60, 0.86, 0.98, 0.28),
            UPbr::glass(),
            UPbrSurface::glass().with_bevel(6.0, 0.8),
        ),
    ];

    for (title, desc, color, pbr, surface) in swatches {
        let is_glass = color.alpha() < 0.9;
        let card = commands
            .spawn((
                ChildOf(grid),
                MaterialSwatchCard,
                UNode {
                    height: UVal::Percent(1.0),
                    background_color: color,
                    border_radius: UCornerRadius::all(16.0),
                    padding: USides::all(16.0),
                    ..default()
                },
                UBorder {
                    color: if is_glass {
                        Color::srgba(0.85, 0.96, 1.0, 0.75)
                    } else {
                        Color::srgba(1.0, 1.0, 1.0, 0.18)
                    },
                    width: 1.6,
                    radius: UCornerRadius::all(16.0),
                    ..default()
                },
                pbr.clone(),
                surface,
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    justify_content: UJustifyContent::SpaceBetween,
                    ..default()
                },
            ))
            .id();

        commands.spawn((
            ChildOf(card),
            UTextLabel {
                text: title.into(),
                font: serif.clone(),
                font_size: 20.0,
                color: Color::WHITE,
                ..default()
            },
            UNode::default(),
            pbr,
            UPbrSurface::beveled(3.5, 0.7),
        ));

        commands.spawn((
            ChildOf(card),
            UTextLabel {
                text: desc.into(),
                font: font.clone(),
                font_size: 13.0,
                color: Color::srgba(0.95, 0.97, 1.0, 0.88),
                ..default()
            },
            UNode::default(),
            UPbr::matte(),
        ));
    }

    // Footer legend.
    commands.spawn((
        ChildOf(board),
        UTextLabel {
            text: "[Space] Orbit Lights   [B] Toggle SDF Bevels   [R] Toggle Shadow Reception   [C] Flip Front/Back Camera"
                .into(),
            font,
            font_size: 13.0,
            color: Color::srgb(0.65, 0.78, 0.92),
            ..default()
        },
        UNode::default(),
    ));
}

fn handle_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ShowcaseState>,
    mut hud_text: Query<&mut UTextLabel, With<StatusHudText>>,
) {
    let mut dirty = false;
    if keyboard.just_pressed(KeyCode::Space) {
        state.animating = !state.animating;
        dirty = true;
    }
    if keyboard.just_pressed(KeyCode::KeyB) {
        state.bevel_enabled = !state.bevel_enabled;
        dirty = true;
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        state.receive_shadows = !state.receive_shadows;
        dirty = true;
    }
    if keyboard.just_pressed(KeyCode::KeyC) {
        state.back_view = !state.back_view;
        dirty = true;
    }

    if dirty && let Ok(mut label) = hud_text.single_mut() {
        label.text = format!(
            "BEVEL: {}  |  SHADOWS: {}  |  VIEW: {}",
            if state.bevel_enabled { "ON" } else { "OFF" },
            if state.receive_shadows { "ON" } else { "OFF" },
            if state.back_view { "BACK (2-SIDED)" } else { "FRONT" },
        );
    }
}

fn animate_orbits(
    time: Res<Time>,
    mut state: ResMut<ShowcaseState>,
    mut orbs: Query<(&OrbitingOrb, &mut Transform), Without<ShadowCastingOrbit>>,
    mut caster: Query<&mut Transform, (With<ShadowCastingOrbit>, Without<OrbitingOrb>)>,
) {
    if state.animating {
        state.time += time.delta_secs();
    }
    let t = state.time;

    for (orb, mut tf) in orbs.iter_mut() {
        let a = t * 1.15 + orb.phase;
        tf.translation = Vec3::new(
            a.cos() * orb.radius_x,
            orb.height + (a * 2.0).sin() * 0.12,
            a.sin() * orb.radius_z,
        );
    }

    if let Ok(mut tf) = caster.single_mut() {
        let a = t * 0.75;
        tf.translation = Vec3::new(a.sin() * 0.55, 0.24 + (a * 1.5).cos() * 0.14, 0.45);
        tf.rotation = Quat::from_euler(EulerRot::XYZ, t * 0.9, t * 1.2, 0.3);
    }
}

fn update_camera_and_surfaces(
    time: Res<Time>,
    state: Res<ShowcaseState>,
    mut cam_q: Query<&mut Transform, With<MainCamera>>,
    mut cards_q: Query<&mut UPbrSurface, With<MaterialSwatchCard>>,
) {
    if let Ok(mut cam_tf) = cam_q.single_mut() {
        let desired_pos = if state.back_view {
            Vec3::new(0.25, 0.28, -1.85)
        } else {
            Vec3::new(0.0, 0.28, 1.85)
        };
        let alpha = (time.delta_secs() * 5.0).min(1.0);
        cam_tf.translation = cam_tf.translation.lerp(desired_pos, alpha);
        cam_tf.look_at(Vec3::new(0.0, 0.04, 0.0), Vec3::Y);
    }

    if state.is_changed() {
        for mut surface in cards_q.iter_mut() {
            surface.bevel_strength = if state.bevel_enabled { 0.85 } else { 0.0 };
            surface.receive_shadows = state.receive_shadows;
        }
    }
}
