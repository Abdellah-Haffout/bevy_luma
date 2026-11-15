//! Changing & Modulating Light Showcase (Optical Gels + Waveforms)
//!
//! Demonstrates two complementary ways UI elements alter scene lighting:
//! 1. **Optical Gels (`ULightFilter`)**: Translucent `UPbr::glass()` UI cards carrying
//!    `ULightFilter::gel(...)` slide in front of a white `SpotLight` projector beam.
//!    Passing through Ruby, Emerald, or Cobalt glass tints the real 3D scene beam,
//!    highlights metallic spheres, and drives a downstream `ULightSensor` (`adopt_incident_color`).
//! 2. **Live Modulators (`ULightModulator`)**: Real `ULightSource` `UNode` and `UText` emitters
//!    animated with `Steady`, `Pulse`, `Flicker`, and `ColorCycle` waveforms.
//!
//! Controls:
//! - `[Space]`: Pause/resume automatic optical gel carousel sweep
//! - `[A / D]` or `[Left / Right]`: Manually slide the optical gel tray through the beam
//! - `[G]`: Enable/disable optical gel filtering
//! - `[M]`: Toggle waveform modulation on the lower light bank

use std::f32::consts::FRAC_PI_2;

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct ChangingLightState {
    auto_sweep: bool,
    filters_enabled: bool,
    modulators_enabled: bool,
    carousel_x: f32,
    time: f32,
}

impl Default for ChangingLightState {
    fn default() -> Self {
        Self {
            auto_sweep: true,
            filters_enabled: true,
            modulators_enabled: true,
            carousel_x: 0.0,
            time: 0.0,
        }
    }
}

#[derive(Component)]
struct GelCarouselRig;

#[derive(Component)]
struct OpticalGelCard;

#[derive(Component)]
struct ModulatedEmitter;

#[derive(Component)]
struct BeamSensorPanel;

#[derive(Component)]
struct BeamSensorReadout;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 04 - Changing Light (ULightFilter & ULightModulator)".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.32, 0.36, 0.44),
            brightness: 18.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<ChangingLightState>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                handle_controls,
                animate_gel_carousel,
                update_beam_readout,
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
        Camera3d::default(),
        Hdr,
        Bloom::NATURAL,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.015, 0.018, 0.028)),
            ..default()
        },
        Transform::from_xyz(0.55, 0.35, 2.40).looking_at(Vec3::new(0.0, 0.05, -0.1), Vec3::Y),
    ));

    // White upper projector beam passing through the sliding ULightFilter optical gel tray at z = +0.42.
    commands.spawn((
        SpotLight {
            color: Color::WHITE,
            intensity: 440_000.0,
            range: 9.0,
            outer_angle: 0.56,
            inner_angle: 0.28,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.26, 1.55).looking_at(Vec3::new(0.0, 0.22, -0.35), Vec3::Y),
    ));

    // Backdrop stage wall + metallic target spheres catching the filtered & modulated light.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(8.5, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.45, 0.48),
            perceptual_roughness: 0.65,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -0.58).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
    ));

    for (x, roughness, metallic) in [(-0.85, 0.14, 0.92), (0.85, 0.22, 0.85)] {
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.16))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.92, 0.92, 0.94),
                metallic,
                perceptual_roughness: roughness,
                ..default()
            })),
            Transform::from_xyz(x, 0.24, -0.15),
        ));
    }

    // Rear Receiving Display + Downstream Light Sensor (left/top) & Modulator Bank (bottom).
    let board = commands
        .spawn((
            URootUi::world_3d(Vec2::new(1020.0, 620.0)),
            UNode {
                width: UVal::Px(1020.0),
                height: UVal::Px(620.0),
                background_color: Color::srgb(0.68, 0.70, 0.74),
                border_radius: UCornerRadius::all(22.0),
                padding: USides::all(24.0),
                ..default()
            },
            UPbr::matte(),
            UPbrSurface::solid().with_bevel(6.0, 0.55),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 18.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.35),
        ))
        .id();

    // Top Sensor Panel showing the real-time beam tint intercepted downstream.
    let sensor_panel = commands
        .spawn((
            ChildOf(board),
            BeamSensorPanel,
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(165.0),
                background_color: Color::WHITE,
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(18.0),
                ..default()
            },
            UPbr::matte(),
            UPbrSurface::solid().with_bevel(4.0, 0.45),
            ULightSensor::default().with_threshold(0.8),
            ULightReactiveVisual {
                dark_color: Color::srgb(0.14, 0.15, 0.18),
                lit_color: Color::srgb(0.92, 0.92, 0.95),
                lit_emissive_boost: 0.25,
                adopt_incident_color: true,
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(sensor_panel),
        UTextLabel {
            text: "OPTICAL BEAM SENSOR (Receives Filtered White Spotlight)".into(),
            font: serif.clone(),
            font_size: 20.0,
            color: Color::srgb(0.10, 0.12, 0.16),
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(sensor_panel),
        BeamSensorReadout,
        UTextLabel {
            text: "DETECTING BEAM COLOR...".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.12, 0.14, 0.18),
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(sensor_panel),
        UTextLabel {
            text: "[Space] Pause Carousel   [A / D or Arrows] Slide Gels   [G] Filters ON   [M] Modulators ON"
                .into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.18, 0.20, 0.26),
            ..default()
        },
        UNode::default(),
    ));

    // Bottom bank: 4 live ULightModulator archetypes on both UNode & UText light sources.
    let mod_bank = commands
        .spawn((
            ChildOf(board),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(260.0),
                background_color: Color::srgba(0.06, 0.07, 0.10, 0.92),
                border_radius: UCornerRadius::all(18.0),
                padding: USides::all(18.0),
                ..default()
            },
            UPbr::matte(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Stretch,
                gap: 14.0,
                ..default()
            },
        ))
        .id();

    spawn_modulator_card(
        &mut commands,
        mod_bank,
        &font,
        &serif,
        "STEADY",
        "Constant Lamp",
        Color::srgb(1.0, 0.85, 0.55),
        ULightModulator {
            enabled: true,
            mode: ULightModulationMode::Steady,
            ..default()
        },
    );
    spawn_modulator_card(
        &mut commands,
        mod_bank,
        &font,
        &serif,
        "PULSE",
        "Breathing Sine",
        Color::srgb(0.25, 0.82, 1.0),
        ULightModulator::pulse(2.2, 0.18, 1.4),
    );
    spawn_modulator_card(
        &mut commands,
        mod_bank,
        &font,
        &serif,
        "FLICKER",
        "Spark / Candle",
        Color::srgb(1.0, 0.52, 0.18),
        ULightModulator::flicker(15.0, 0.62),
    );
    spawn_modulator_card(
        &mut commands,
        mod_bank,
        &font,
        &serif,
        "PRISM",
        "HSL ColorCycle",
        Color::srgb(0.85, 0.35, 1.0),
        ULightModulator::color_cycle(0.42, 0.92, 0.55),
    );

    // Foreground sliding Optical Gel Tray at z = +0.42 right across the white SpotLight ray.
    let gel_tray = commands
        .spawn((
            GelCarouselRig,
            URootUi::world_3d(Vec2::new(820.0, 170.0)),
            UNode {
                width: UVal::Px(820.0),
                height: UVal::Px(170.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                gap: 48.0,
                ..default()
            },
            Transform::from_xyz(0.0, 0.24, 0.42),
        ))
        .id();

    for (title, gel_color) in [
        ("RUBY GEL", Color::srgba(1.0, 0.16, 0.20, 0.42)),
        ("EMERALD GEL", Color::srgba(0.14, 1.0, 0.42, 0.42)),
        ("COBALT GEL", Color::srgba(0.18, 0.48, 1.0, 0.42)),
    ] {
        let gel = commands
            .spawn((
                ChildOf(gel_tray),
                OpticalGelCard,
                UNode {
                    width: UVal::Px(220.0),
                    height: UVal::Px(140.0),
                    background_color: gel_color,
                    border_radius: UCornerRadius::all(18.0),
                    ..default()
                },
                UBorder {
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                    width: 2.0,
                    radius: UCornerRadius::all(18.0),
                    ..default()
                },
                UPbr::glass(),
                UPbrSurface::glass().with_bevel(6.0, 0.8),
                ULightFilter::gel(Color::srgb(
                    gel_color.to_srgba().red,
                    gel_color.to_srgba().green,
                    gel_color.to_srgba().blue,
                )),
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                    ..default()
                },
            ))
            .id();

        commands.spawn((
            ChildOf(gel),
            UTextLabel {
                text: title.into(),
                font: font.clone(),
                font_size: 16.0,
                color: Color::WHITE,
                ..default()
            },
            UNode::default(),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_modulator_card(
    commands: &mut Commands,
    parent: Entity,
    font: &Handle<Font>,
    serif: &Handle<Font>,
    title: &str,
    subtitle: &str,
    color: Color,
    modulator: ULightModulator,
) {
    let card = commands
        .spawn((
            ChildOf(parent),
            UNode {
                width: UVal::Px(220.0),
                height: UVal::Percent(1.0),
                background_color: Color::srgb(0.11, 0.13, 0.17),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::all(16.0),
                ..default()
            },
            UPbr::matte(),
            UPbrSurface::beveled(4.0, 0.5),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    // UText itself acting as a modulated emissive light source!
    commands.spawn((
        ChildOf(card),
        ModulatedEmitter,
        UTextLabel {
            text: title.into(),
            font: serif.clone(),
            font_size: 32.0,
            color,
            ..default()
        },
        UNode::default(),
        UPbr::neon(color, 4.5),
        ULightSource::bulb(color, 18_000.0, 3.2).with_offset(Vec3::new(0.0, 0.0, 0.10)),
        modulator,
    ));

    commands.spawn((
        ChildOf(card),
        UTextLabel {
            text: subtitle.into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.76, 0.80, 0.88),
            ..default()
        },
        UNode::default(),
    ));
}

fn handle_controls(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ChangingLightState>,
    mut filters: Query<&mut ULightFilter, With<OpticalGelCard>>,
    mut modulators: Query<&mut ULightModulator, With<ModulatedEmitter>>,
) {
    let dt = time.delta_secs();
    let mut manual = false;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        state.carousel_x -= 0.72 * dt;
        manual = true;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        state.carousel_x += 0.72 * dt;
        manual = true;
    }
    if manual {
        state.auto_sweep = false;
        state.carousel_x = state.carousel_x.clamp(-0.95, 0.95);
    }

    if keyboard.just_pressed(KeyCode::Space) {
        state.auto_sweep = !state.auto_sweep;
    }
    if keyboard.just_pressed(KeyCode::KeyG) {
        state.filters_enabled = !state.filters_enabled;
        for mut f in filters.iter_mut() {
            f.enabled = state.filters_enabled;
        }
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        state.modulators_enabled = !state.modulators_enabled;
        for mut m in modulators.iter_mut() {
            m.enabled = state.modulators_enabled;
        }
    }
}

fn animate_gel_carousel(
    time: Res<Time>,
    mut state: ResMut<ChangingLightState>,
    mut rig: Query<&mut Transform, With<GelCarouselRig>>,
) {
    if state.auto_sweep {
        state.time += time.delta_secs();
        state.carousel_x = (state.time * 0.75).sin() * 0.80;
    }

    if let Ok(mut tf) = rig.single_mut() {
        tf.translation.x = state.carousel_x;
    }
}

fn update_beam_readout(
    state: Res<ChangingLightState>,
    sensor_q: Query<&ULightSensor, With<BeamSensorPanel>>,
    mut label_q: Query<&mut UTextLabel, With<BeamSensorReadout>>,
) {
    let Ok(sensor) = sensor_q.single() else {
        return;
    };
    let Ok(mut label) = label_q.single_mut() else {
        return;
    };

    let c = Srgba::from(sensor.dominant_color);
    let filter_tag = if let Some(filter_ent) = sensor.filtered_by {
        format!("FILTERED ({:?})", filter_ent)
    } else {
        "UNFILTERED WHITE BEAM".into()
    };

    label.text = format!(
        "{} | RGB: ({:.2}, {:.2}, {:.2}) | Direct: {:.1} | Gels: {} | Modulators: {}",
        filter_tag,
        c.red,
        c.green,
        c.blue,
        sensor.direct_illuminance,
        if state.filters_enabled { "ON" } else { "OFF" },
        if state.modulators_enabled { "ON" } else { "OFF" },
    );
}
