//! Blocking & Occluding Light Showcase (SDF UI & Typography Shadows)
//!
//! Demonstrates real light occlusion from `UNode` and `UText` elements:
//! - Hardware SDF silhouette shadow casting via `ULightOccluder` + `prepass_fragment_shader`.
//! - Giant `UText` cutout letters (`"SHADOW"`) casting razor-sharp letter shadows onto a receiving board & wall.
//! - Chamfered (`UShapeMode::Cut`) movable `UNode` shutter blocking both real shadow rays AND `ULightSensor` beams.
//! - Opaque vs partially translucent blocker (`ULightOccluder::solid` vs `ULightOccluder::translucent`).
//!
//! Controls:
//! - `[W/A/S/D]` or Arrow keys: Move the UI shutter and `UText` occluder in front of the projector
//! - `[T]`: Toggle shutter between fully opaque and 40% translucent occlusion
//! - `[Space]`: Pause/resume gentle idle sweep

use std::f32::consts::FRAC_PI_2;

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct BlockerState {
    manual_offset: Vec2,
    translucent: bool,
    auto_sweep: bool,
    time: f32,
}

impl Default for BlockerState {
    fn default() -> Self {
        Self {
            manual_offset: Vec2::ZERO,
            translucent: false,
            auto_sweep: true,
            time: 0.0,
        }
    }
}

#[derive(Component)]
struct MovableShutterRig;

#[derive(Component)]
struct ToggleableShutterNode;

#[derive(Component)]
struct SensorReadoutLabel;

#[derive(Component)]
struct PrimarySensorCard;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 03 - Blocking Light (UI & Text SDF Occluders)".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.35, 0.42, 0.55),
            brightness: 25.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<BlockerState>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (handle_controls, move_shutter, update_sensor_readout),
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
        Camera3d::default(),
        Hdr,
        Bloom::NATURAL,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.02, 0.024, 0.035)),
            ..default()
        },
        Transform::from_xyz(0.55, 0.35, 2.25).looking_at(Vec3::new(0.0, 0.0, -0.1), Vec3::Y),
    ));

    // High-powered shadow-casting SpotLight shining through the occluders onto the receiver screen.
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            intensity: 480_000.0,
            range: 10.0,
            outer_angle: 0.72,
            inner_angle: 0.42,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.25, 1.85).looking_at(Vec3::new(0.0, 0.0, -0.5), Vec3::Y),
    ));

    // Light receiving wall behind the receiver board so shadows across the edges read dramatically.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.44, 0.48),
            perceptual_roughness: 0.75,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -0.65).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    ));

    // Receiving UI board + Light Sensor card at z = -0.35.
    let receiver_board = commands
        .spawn((
            URootUi::world_3d(Vec2::new(960.0, 560.0)),
            UNode {
                width: UVal::Px(960.0),
                height: UVal::Px(560.0),
                background_color: Color::srgb(0.74, 0.76, 0.80),
                border_radius: UCornerRadius::all(20.0),
                padding: USides::all(24.0),
                ..default()
            },
            UPbr::matte(),
            UPbrSurface::solid().with_bevel(6.0, 0.5),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.35),
        ))
        .id();

    let sensor_panel = commands
        .spawn((
            ChildOf(receiver_board),
            PrimarySensorCard,
            UNode {
                width: UVal::Px(430.0),
                height: UVal::Px(130.0),
                background_color: Color::srgb(0.14, 0.17, 0.22),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.4, 0.85, 1.0, 0.5),
                width: 1.5,
                radius: UCornerRadius::all(14.0),
                ..default()
            },
            UPbr::matte(),
            ULightSensor::default().with_threshold(1.2),
            ULightReactiveVisual {
                dark_color: Color::srgb(0.16, 0.08, 0.08),
                lit_color: Color::srgb(0.14, 0.42, 0.28),
                lit_emissive_boost: 0.4,
                adopt_incident_color: false,
            },
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
        ChildOf(sensor_panel),
        SensorReadoutLabel,
        UTextLabel {
            text: "SENSOR: CHECKING RAY OCCLUSION...".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(receiver_board),
        UTextLabel {
            text: "[W/A/S/D or Arrows] Slide Occluders   [T] Toggle Opaque / 40% Translucent   [Space] Pause Sweep"
                .into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.14, 0.16, 0.22),
            ..default()
        },
        UNode::default(),
    ));

    // Movable Occluder Rig at z = +0.35 in front of the receiver board.
    let shutter_rig = commands
        .spawn((
            MovableShutterRig,
            URootUi::world_3d(Vec2::new(660.0, 320.0)),
            UNode {
                width: UVal::Px(660.0),
                height: UVal::Px(320.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceAround,
                align_items: UAlignItems::Center,
                gap: 20.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.06, 0.35),
        ))
        .id();

    // Chamfered UI panel acting as an SDF light occluder.
    let card_occluder = commands
        .spawn((
            ChildOf(shutter_rig),
            ToggleableShutterNode,
            UNode {
                width: UVal::Px(320.0),
                height: UVal::Px(110.0),
                background_color: Color::srgb(0.18, 0.22, 0.30),
                border_radius: UCornerRadius::all(26.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(12.0),
                ..default()
            },
            UPbr::stone(),
            UPbrSurface::beveled(7.0, 0.7),
            ULightOccluder::solid(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(card_occluder),
        UTextLabel {
            text: "CHAMFERED UI BLOCKER".into(),
            font,
            font_size: 15.0,
            color: Color::srgb(0.92, 0.95, 1.0),
            ..default()
        },
        UNode::default(),
    ));

    // Giant UText acting directly as an SDF glyph silhouette shadow caster!
    commands.spawn((
        ChildOf(shutter_rig),
        UTextLabel {
            text: "SHADOW".into(),
            font: serif,
            font_size: 92.0,
            color: Color::srgb(0.92, 0.72, 0.28),
            ..default()
        },
        UNode::default(),
        UPbr::gold(),
        UPbrSurface::beveled(4.0, 0.8),
        ULightOccluder::solid(),
    ));
}

fn handle_controls(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BlockerState>,
    mut shutter_q: Query<(&mut ULightOccluder, &mut UNode), With<ToggleableShutterNode>>,
) {
    let dt = time.delta_secs();
    let speed = 0.75;
    let mut moved = false;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        state.manual_offset.x -= speed * dt;
        moved = true;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        state.manual_offset.x += speed * dt;
        moved = true;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        state.manual_offset.y += speed * dt;
        moved = true;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        state.manual_offset.y -= speed * dt;
        moved = true;
    }
    if moved {
        state.manual_offset = state.manual_offset.clamp(Vec2::splat(-0.75), Vec2::splat(0.75));
        state.auto_sweep = false;
    }

    if keyboard.just_pressed(KeyCode::Space) {
        state.auto_sweep = !state.auto_sweep;
    }

    if keyboard.just_pressed(KeyCode::KeyT) {
        state.translucent = !state.translucent;
        if let Ok((mut occ, mut node)) = shutter_q.single_mut() {
            if state.translucent {
                *occ = ULightOccluder::translucent(0.40);
                node.background_color = Color::srgba(0.35, 0.55, 0.80, 0.45);
            } else {
                *occ = ULightOccluder::solid();
                node.background_color = Color::srgb(0.18, 0.22, 0.30);
            }
        }
    }
}

fn move_shutter(
    time: Res<Time>,
    mut state: ResMut<BlockerState>,
    mut rig_q: Query<&mut Transform, With<MovableShutterRig>>,
) {
    if state.auto_sweep {
        state.time += time.delta_secs();
    }
    if let Ok(mut tf) = rig_q.single_mut() {
        let sweep = if state.auto_sweep {
            Vec2::new((state.time * 0.9).sin() * 0.45, (state.time * 0.45).cos() * 0.14)
        } else {
            Vec2::ZERO
        };
        let target = state.manual_offset + sweep;
        tf.translation.x = target.x;
        tf.translation.y = 0.06 + target.y;
    }
}

fn update_sensor_readout(
    state: Res<BlockerState>,
    sensor_q: Query<&ULightSensor, With<PrimarySensorCard>>,
    mut readout_q: Query<&mut UTextLabel, With<SensorReadoutLabel>>,
) {
    let (Ok(sensor), Ok(mut label)) = (sensor_q.single(), readout_q.single_mut()) else {
        return;
    };
    let status = if sensor.occluded_by.is_some() {
        if state.translucent {
            "PARTIALLY BLOCKED (40% pass)"
        } else {
            "OCCLUDED BY UI BLOCKER"
        }
    } else if sensor.is_lit {
        "CLEAR BEAM (LIT)"
    } else {
        "LOW LIGHT"
    };
    label.text = format!(
        "SENSOR: {}  |  ILLUMINANCE: {:.2}  |  MODE: {}",
        status,
        sensor.direct_illuminance,
        if state.translucent { "TRANSLUCENT" } else { "OPAQUE" }
    );
}
