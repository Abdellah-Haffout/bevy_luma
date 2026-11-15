//! Light-Dependent Interactive UI Showcase
//!
//! Demonstrates components that physically CANNOT operate unless illuminated:
//! - Interactive `World3d` solar buttons equipped with `URequiresLight`, `ULightSensor`,
//!   `ULightReactiveVisual`, `UInteraction`, `UInteractionColors`, `UInteractionScale`, and `UFocusable`.
//! - When illuminated, clicking or Tab-focusing & pressing `Enter`/`Space` increments the terminal counters.
//! - When the overhead `ULightSource` lamp is turned OFF (`[L]`) OR an opaque `ULightOccluder`
//!   shutter slides in front of the beam (`[S]`), picking hover/click, interaction state, and
//!   focus navigation automatically lock offline (`ULightAvailabilityChanged`).
//!
//! Controls:
//! - `Mouse Click` or `[Tab]` + `[Enter / Space]`: Operate solar buttons (only works when lit!)
//! - `[L]`: Toggle overhead `ULightSource` lamp ON/OFF
//! - `[S]`: Close/open opaque `ULightOccluder` beam shutter
//! - `[A]`: Toggle low ambient light vs ambient-inclusive solar cells

use bevy::camera::Hdr;
use bevy::light::GlobalAmbientLight;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_luma::prelude::*;

#[derive(Resource)]
struct SolarStationState {
    lamp_on: bool,
    shutter_closed: bool,
    ambient_backup: bool,
    ops_count: u32,
    last_action: String,
    shutter_y: f32,
}

impl Default for SolarStationState {
    fn default() -> Self {
        Self {
            lamp_on: true,
            shutter_closed: false,
            ambient_backup: false,
            ops_count: 0,
            last_action: "Station ready. Click solar buttons or press [Tab] + [Enter].".into(),
            shutter_y: 0.72,
        }
    }
}

#[derive(Component)]
struct OverheadLamp;

#[derive(Component)]
struct BeamShutterRig;

#[derive(Component)]
struct SolarButton {
    label: &'static str,
}

#[derive(Component)]
struct StatusBannerLabel;

#[derive(Component)]
struct PowerMeterLabel;

#[derive(Component)]
struct MainTelemetrySensor;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Lighting 05 - Light-Dependent UI (URequiresLight)".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.25, 0.30, 0.42),
            brightness: 12.0,
            affects_lightmapped_meshes: true,
        })
        .init_resource::<SolarStationState>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (handle_controls, sync_lamp_and_shutter, update_hud),
        )
        .run();
}

fn setup_scene(mut commands: Commands, theme: Res<Theme>) {
    let font = theme.text.font.inter_regular.clone();
    let serif = theme.text.font.free_serif.clone();

    commands.spawn((
        Camera3d::default(),
        Hdr,
        Bloom::NATURAL,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.015, 0.018, 0.025)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.10, 2.15).looking_at(Vec3::new(0.0, 0.02, -0.2), Vec3::Y),
    ));

    // Toggleable overhead Spotlight Lamp angled downward across the shutter toward the console.
    commands.spawn((
        OverheadLamp,
        ULightSource {
            enabled: true,
            kind: ULightSourceKind::Spot {
                radius: 0.08,
                inner_angle: 0.35,
                outer_angle: 0.65,
            },
            color: Color::srgb(1.0, 0.94, 0.78),
            intensity: 480_000.0,
            range: 9.0,
            shadow_maps_enabled: true,
            // Shine along child -Z matching parent Transform::looking_at.
            direction_sign: -1.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.52, 1.55).looking_at(Vec3::new(0.0, -0.05, -0.35), Vec3::Y),
    ));

    // Movable opaque UI Shutter in the beam path (z = +0.42).
    let shutter = commands
        .spawn((
            BeamShutterRig,
            URootUi::world_3d(Vec2::new(680.0, 220.0)),
            UNode {
                width: UVal::Px(680.0),
                height: UVal::Px(220.0),
                background_color: Color::srgb(0.14, 0.16, 0.20),
                border_radius: UCornerRadius::all(18.0),
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: Color::srgb(0.85, 0.35, 0.22),
                width: 2.0,
                radius: UCornerRadius::all(18.0),
                ..default()
            },
            UPbr::brushed_metal(),
            UPbrSurface::beveled(6.0, 0.7),
            ULightOccluder::solid(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            Transform::from_xyz(0.0, 0.72, 0.42),
        ))
        .id();

    commands.spawn((
        ChildOf(shutter),
        UTextLabel {
            text: "OPAQUE LIGHT SHUTTER (Blocks Solar Power)".into(),
            font: font.clone(),
            font_size: 16.0,
            color: Color::srgb(0.95, 0.55, 0.45),
            ..default()
        },
        UNode::default(),
    ));

    // Main Solar Workstation Panel at z = -0.35.
    let console = commands
        .spawn((
            URootUi::world_3d(Vec2::new(920.0, 540.0)),
            UNode {
                width: UVal::Px(920.0),
                height: UVal::Px(540.0),
                background_color: Color::srgb(0.16, 0.19, 0.24),
                border_radius: UCornerRadius::all(22.0),
                padding: USides::all(28.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.35, 0.45, 0.58, 0.55),
                width: 2.0,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UPbr::matte(),
            UPbrSurface::beveled(6.0, 0.6),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 20.0,
                ..default()
            },
            Transform::from_xyz(0.0, -0.04, -0.35),
        ))
        .id();

    // Header + live Solar Sensor Telemetry card.
    let header = commands
        .spawn((
            ChildOf(console),
            MainTelemetrySensor,
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(135.0),
                background_color: Color::srgb(0.12, 0.15, 0.19),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(18.0),
                ..default()
            },
            UPbr::matte(),
            ULightSensor::default().with_threshold(0.8),
            ULightReactiveVisual {
                dark_color: Color::srgb(0.09, 0.06, 0.07),
                lit_color: Color::srgb(0.10, 0.24, 0.20),
                lit_emissive_boost: 0.35,
                adopt_incident_color: false,
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
        ChildOf(header),
        UTextLabel {
            text: "SOLAR-POWERED INTERACTIVE CONSOLE".into(),
            font: serif,
            font_size: 24.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(header),
        PowerMeterLabel,
        UTextLabel {
            text: "SOLAR IRRADIANCE: CHECKING...".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.75, 0.96, 0.85),
            ..default()
        },
        UNode::default(),
    ));

    // Middle Row: Three Light-Locked Solar Action Buttons!
    let button_row = commands
        .spawn((
            ChildOf(console),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(190.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Stretch,
                gap: 20.0,
                ..default()
            },
        ))
        .id();

    for (label, caption) in [
        ("CHARGE CELL", "Harvest +1 Unit"),
        ("FIRE BEACON", "Emit Signal"),
        ("SYNC CORE", "Calibrate Bus"),
    ] {
        spawn_solar_button(&mut commands, button_row, &font, label, caption);
    }

    // Bottom status bar (remains subtly readable on screen so user always sees controls).
    let footer = commands
        .spawn((
            ChildOf(console),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(110.0),
                background_color: Color::srgb(0.08, 0.10, 0.14),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::all(16.0),
                ..default()
            },
            UPbr::matte(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(footer),
        StatusBannerLabel,
        UTextLabel {
            text: "Ready".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.95, 0.84, 0.45),
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(footer),
        UTextLabel {
            text: "[Click / Tab+Enter] Trigger Solar Button   [L] Toggle Lamp   [S] Close/Open Shutter   [A] Toggle Ambient Power"
                .into(),
            font,
            font_size: 13.0,
            color: Color::srgb(0.72, 0.78, 0.88),
            ..default()
        },
        UNode::default(),
    ));
}

fn spawn_solar_button(
    commands: &mut Commands,
    parent: Entity,
    font: &Handle<Font>,
    label: &'static str,
    caption: &str,
) {
    let mut ec = commands.spawn((
        ChildOf(parent),
        SolarButton { label },
        UNode {
            width: UVal::Px(268.0),
            height: UVal::Percent(1.0),
            background_color: Color::srgb(0.16, 0.46, 0.34),
            border_radius: UCornerRadius::all(16.0),
            padding: USides::all(18.0),
            ..default()
        },
        UBorder {
            color: Color::srgb(0.45, 0.92, 0.70),
            width: 2.0,
            radius: UCornerRadius::all(16.0),
            ..default()
        },
        UPbr::satin(),
        UPbrSurface::beveled(5.0, 0.65),
        UInteraction::default(),
        UInteractionColors {
            normal: Color::srgb(0.18, 0.50, 0.38),
            hovered: Color::srgb(0.24, 0.68, 0.50),
            pressed: Color::srgb(0.12, 0.34, 0.25),
        },
        UInteractionScale::default(),
        UFocusable::new(),
        UFocusVisual::ring(Color::srgb(0.98, 0.80, 0.25))
            .with_contract(6.0, 0.0)
            .with_as_hover(true),
        ULightSensor::default().with_threshold(0.8),
        URequiresLight::with_min_illuminance(0.8),
        ULightReactiveVisual {
            dark_color: Color::srgb(0.08, 0.08, 0.10),
            lit_color: Color::srgb(0.18, 0.50, 0.38),
            lit_emissive_boost: 0.45,
            adopt_incident_color: false,
        },
        ULayout {
            display: UDisplay::Flex,
            flex_direction: UFlexDirection::Column,
            justify_content: UJustifyContent::Center,
            align_items: UAlignItems::Center,
            gap: 8.0,
            ..default()
        },
    ));

    ec.observe(
        |trigger: On<PointerClick>,
         buttons: Query<(&SolarButton, &URequiresLight)>,
         mut state: ResMut<SolarStationState>| {
            if let Ok((btn, req)) = buttons.get(trigger.event_target())
                && req.allows_interaction()
            {
                state.ops_count += 1;
                state.last_action =
                    format!("Clicked [{}] -> Ops: {}", btn.label, state.ops_count);
            }
        },
    );

    ec.observe(
        |trigger: On<UFocusActivate>,
         buttons: Query<(&SolarButton, &URequiresLight)>,
         mut state: ResMut<SolarStationState>| {
            if let Ok((btn, req)) = buttons.get(trigger.event_target())
                && req.allows_interaction()
            {
                state.ops_count += 1;
                state.last_action = format!(
                    "Activated [{}] via Keyboard Focus -> Ops: {}",
                    btn.label, state.ops_count
                );
            }
        },
    );

    ec.observe(
        |trigger: On<ULightAvailabilityChanged>,
         buttons: Query<&SolarButton>,
         mut state: ResMut<SolarStationState>| {
            let ev = trigger.event();
            if let Ok(btn) = buttons.get(ev.entity) {
                state.last_action = if ev.is_active {
                    format!(
                        "[{}] ONLINE (light {:.2} lux) — controls unlocked!",
                        btn.label, ev.illuminance
                    )
                } else {
                    format!(
                        "[{}] LOCKED OFFLINE (light {:.2} lux) — insufficient light!",
                        btn.label, ev.illuminance
                    )
                };
            }
        },
    );

    let button_id = ec.id();

    commands.spawn((
        ChildOf(button_id),
        UTextLabel {
            text: label.into(),
            font: font.clone(),
            font_size: 18.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(button_id),
        UTextLabel {
            text: caption.into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.84, 0.92, 0.88),
            ..default()
        },
        UNode::default(),
    ));
}

fn handle_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SolarStationState>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut req_q: Query<(&mut URequiresLight, &mut ULightSensor)>,
) {
    if keyboard.just_pressed(KeyCode::KeyL) {
        state.lamp_on = !state.lamp_on;
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        state.shutter_closed = !state.shutter_closed;
    }
    if keyboard.just_pressed(KeyCode::KeyA) {
        state.ambient_backup = !state.ambient_backup;
        ambient.brightness = if state.ambient_backup { 120.0 } else { 12.0 };
        for (mut req, mut sensor) in req_q.iter_mut() {
            req.include_ambient = state.ambient_backup;
            sensor.include_ambient = state.ambient_backup;
        }
    }
}

fn sync_lamp_and_shutter(
    time: Res<Time>,
    mut state: ResMut<SolarStationState>,
    mut lamp_q: Query<&mut ULightSource, With<OverheadLamp>>,
    mut shutter_q: Query<&mut Transform, With<BeamShutterRig>>,
) {
    if let Ok(mut src) = lamp_q.single_mut() {
        src.enabled = state.lamp_on;
    }

    let target_y = if state.shutter_closed { 0.0 } else { 0.72 };
    let t = (time.delta_secs() * 8.0).min(1.0);
    state.shutter_y = state.shutter_y + (target_y - state.shutter_y) * t;

    if let Ok(mut tf) = shutter_q.single_mut() {
        tf.translation.y = state.shutter_y;
    }
}

fn update_hud(
    state: Res<SolarStationState>,
    sensor_q: Query<&ULightSensor, With<MainTelemetrySensor>>,
    buttons_q: Query<&URequiresLight, With<SolarButton>>,
    mut meter_q: Query<&mut UTextLabel, With<PowerMeterLabel>>,
    mut status_q: Query<&mut UTextLabel, (With<StatusBannerLabel>, Without<PowerMeterLabel>)>,
) {
    if let (Ok(sensor), Ok(mut meter)) = (sensor_q.single(), meter_q.single_mut()) {
        let online = buttons_q.iter().any(|r| r.allows_interaction());
        meter.text = format!(
            "STATE: {} | Direct: {:.1} | Ambient: {:.1} | Lamp: {} | Shutter: {} | Total Ops: {}",
            if online {
                "ONLINE (INTERACTION ACTIVE)"
            } else {
                "LOCKED (REQUIRES LIGHT)"
            },
            sensor.direct_illuminance,
            sensor.ambient_illuminance,
            if state.lamp_on { "ON" } else { "OFF" },
            if state.shutter_closed {
                "CLOSED"
            } else {
                "OPEN"
            },
            state.ops_count,
        );
    }

    if let Ok(mut status) = status_q.single_mut() {
        status.text = state.last_action.clone();
    }
}
