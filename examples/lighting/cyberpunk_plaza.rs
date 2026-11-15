//! Cyberpunk Night Plaza - Free-Roam 3D Illuminated UI Showcase
//!
//! A pitch-black cybernetic plaza (`GlobalAmbientLight::NONE`) with ZERO external lights.
//! Every beam, neon reflection on the wet metallic basalt floor, volumetric-feeling glow,
//! and shadow across the 3D props comes exclusively from illuminated `World3d` UI screens
//! and glowing 3D `UTextLabel` signage positioned in EVERY direction (North, East, South,
//! West, Overhead Skyway, and Floor Holopads).
//!
//! Showcases the full power of `bevy_luma`:
//! - **Free 6-DoF Camera**: Fly in any direction above or across the dark plaza floor.
//! - **North (`NEON COMMAND`)**: GPU `UShader::cyber_matrix`, `UDisplay::Grid`, Arabic + Latin
//!   RTL typography, `UTransition::bouncy`, `UInteractionScale`, `UFocusable`, `UInnerGlow`.
//! - **East (`PHOTONICS LAB`)**: GPU `UShader::aurora_beams`, cycling `ULightModulator`
//!   (`ColorCycle`, `Pulse`, `Flicker`, `Steady`), toggleable `ULightFilter` optical gel beam
//!   changer, and beveled PBR swatches (`wood`, `chrome`, `gold`, `glass`, `stone`).
//! - **South (`SOLAR VAULT`)**: GPU `UShader::plasma_energy`, `UScrollContainer::vertical()`
//!   + `UScrollKineticDrag`, `ULightOccluder` armored shutter casting real SDF shadows onto
//!   light-gated `URequiresLight` + `ULightSensor` + `ULightReactiveVisual` switches.
//! - **West (`NEURAL UPLINK`)**: GPU `UShader::liquid_chrome`, interactive 3D `UTextInput`
//!   command terminal broadcasting live text across the plaza, `UDisplay::Radial` reactor core,
//!   and multi-stop `UGradient`.
//!
//! Controls:
//! - `[W/A/S/D]` or Arrow keys: Fly forward / strafe / back in camera direction
//! - `[E / Space]` and `[Q / Ctrl]`: Fly up / down (`Shift` = sprint boost)
//! - `[Right-Click Drag]` or `[V]` toggle: Look around freely in any direction (`[J/L/I/K]` also turns)
//! - `[Left-Click]`: Interact directly with any 3D button, shader card, scroll feed, or text input
//! - `[1 / 2 / 3 / 4]`: Snap aim toward North / East / South / West screens (`[R]` reset center)
//! - `[B]`: Toggle City Power Blackout (watch the entire plaza go dark & vault controls lock!)

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::asset::RenderAssetUsages;
use bevy::camera::Hdr;
use bevy::camera::visibility::RenderLayers;
use bevy::image::{ImageAddressMode, ImageFilterMode};
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::GlobalAmbientLight;
use bevy::math::Affine2;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_luma::prelude::*;

// ---------------------------------------------------------------------------
// Palette & Constants
// ---------------------------------------------------------------------------

const CYAN: Color = Color::srgb(0.08, 0.92, 1.0);
const MAGENTA: Color = Color::srgb(1.0, 0.16, 0.68);
const AMBER: Color = Color::srgb(1.0, 0.72, 0.14);
const VIOLET: Color = Color::srgb(0.68, 0.34, 1.0);
const EMERALD: Color = Color::srgb(0.14, 0.98, 0.62);
const SCREEN_RADIUS: f32 = 3.85;
const SCREEN_MPU: f32 = 0.0024;

const FLOOR_PRESETS: [(&str, Color); 4] = [
    ("Teal Slate", Color::srgb(0.24, 0.44, 0.52)),
    ("Sandstone", Color::srgb(0.58, 0.42, 0.28)),
    ("Jade Stone", Color::srgb(0.22, 0.48, 0.36)),
    ("Royal Indigo", Color::srgb(0.32, 0.28, 0.58)),
];

fn hdr_glow(color: Color, intensity: f32, alpha: f32) -> Color {
    let lin = LinearRgba::from(color);
    Color::LinearRgba(LinearRgba::new(
        lin.red * intensity,
        lin.green * intensity,
        lin.blue * intensity,
        alpha,
    ))
}

fn hdr_emissive(color: Color, strength: f32) -> LinearRgba {
    let lin = LinearRgba::from(color);
    let gain = strength.max(0.0);
    LinearRgba::rgb(lin.red * gain, lin.green * gain, lin.blue * gain)
}

fn screen_panel_pbr(tint: Color, emissive_strength: f32) -> UPbr {
    UPbr::new(0.16, 0.28).with_emissive(hdr_emissive(tint, emissive_strength))
}

fn screen_rect_light(
    color: Color,
    intensity: f32,
    range: f32,
    panel_emissive: f32,
) -> (ULightSource, PlazaLightFixture) {
    (
        ULightSource::lamp(color, intensity)
            .with_range(range)
            .with_emissive(panel_emissive)
            .with_offset(Vec3::new(0.0, 0.0, 0.08)),
        PlazaLightFixture {
            base_intensity: intensity,
        },
    )
}

// ---------------------------------------------------------------------------
// State & Marker Components
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct FloorMaterialHandle(Handle<StandardMaterial>);

#[derive(Resource)]
struct PlazaState {
    overdrive: bool,
    blackout: bool,
    east_gel_enabled: bool,
    east_mode_idx: usize,
    south_shutter_closed: bool,
    free_look_toggle: bool,
    radial_speed: f32,
    shutter_offset_y: f32,
    broadcast: String,
    logs: Vec<String>,
    ops_count: usize,
    floor_preset_idx: usize,
    floor_roughness: f32,
    light_scale: f32,
    bloom_mode_idx: usize,
    soft_ambient: bool,
    hud_expanded: bool,
    /// Pending one-shot HUD camera snap (`0..=3` = N/E/S/W, `4` = Reset).
    pending_snap: Option<usize>,
}

impl Default for PlazaState {
    fn default() -> Self {
        Self {
            overdrive: false,
            blackout: false,
            east_gel_enabled: false,
            east_mode_idx: 0,
            south_shutter_closed: false,
            free_look_toggle: false,
            radial_speed: 1.0,
            shutter_offset_y: 0.78,
            floor_preset_idx: 0,
            floor_roughness: 0.58,
            light_scale: 1.0,
            bloom_mode_idx: 1,
            soft_ambient: false,
            hud_expanded: true,
            broadcast: "شبكة المراقبة نشطة — LUMACORE GRID ONLINE".into(),
            logs: vec![
                "[00:01] BOOT: Zero-ambient cybernetic plaza initialized".into(),
                "[00:02] NORTH: RectLight cyan deck + shader matrix online".into(),
                "[00:03] EAST: Chromatic photonics SpotLight calibrated".into(),
                "[00:04] SOUTH: Solar vault switches armed & illuminated".into(),
                "[00:05] WEST: Neural 3D UTextInput uplink ready".into(),
                "[00:06] HINT: Try typing 'overdrive', 'blackout', or 'prism' in West".into(),
            ],
            ops_count: 0,
            pending_snap: None,
        }
    }
}

impl PlazaState {
    fn push_log(&mut self, msg: impl Into<String>) {
        self.logs.push(msg.into());
        if self.logs.len() > 14 {
            self.logs.remove(0);
        }
    }
}

#[derive(Component)]
struct FlyCamera {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

#[derive(Component)]
struct PlazaLightFixture {
    base_intensity: f32,
}

#[derive(Component)]
struct EastPrismLight;

#[derive(Component)]
struct EastGelLens;

#[derive(Component)]
struct EastSculpture;

#[derive(Component)]
struct SouthShutterRig;

#[derive(Component)]
struct BroadcastBannerLabel;

#[derive(Component)]
struct TelemetryHudLabel;

#[derive(Component)]
struct ScrollLogLabel;

#[derive(Component)]
struct EastModeLabel;

#[derive(Component)]
struct ReactorCoreSpinner;

#[derive(Component)]
struct HudDrawerBody;

#[derive(Component)]
struct HudStatusText;

#[derive(Component)]
struct HudToggleLabel;

#[derive(Component)]
struct HudFloorReadout;

#[derive(Component)]
struct HudRoughnessReadout;

#[derive(Component)]
struct HudLightReadout;

#[derive(Component)]
struct HudBloomReadout;

#[derive(Component)]
struct HudAmbientReadout;

#[derive(Component)]
struct HudFloorPresetChip(usize);

fn world_root(size: Vec2, mpu: f32) -> URootUi {
    let mut root = URootUi::world_3d(size);
    root.meters_per_unit = mpu;
    root
}

// ---------------------------------------------------------------------------
// App Entry
// ---------------------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma Cyberpunk Night Plaza | [WASD/QE] Fly  [RMB Drag / V] Look  [1-4] Snap  [Left-Click] Interact"
                    .into(),
                resolution: (1360, 860).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        // Pitch black: zero ambient fill light anywhere in the scene.
        .insert_resource(GlobalAmbientLight::NONE)
        .init_resource::<PlazaState>()
        .add_systems(Startup, setup_cyberpunk_plaza)
        .add_systems(
            Update,
            (
                control_free_camera,
                animate_plaza_elements,
                sync_plaza_ui_and_lights,
                sync_hud_settings,
            ),
        )
        .add_observer(on_cyber_text_submitted)
        .run();
}

// ---------------------------------------------------------------------------
// Scene Construction
// ---------------------------------------------------------------------------

fn setup_cyberpunk_plaza(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    theme: Res<Theme>,
) {
    let font = theme.text.font.inter_regular.clone();
    let serif = theme.text.font.free_serif.clone();
    let icon_font = theme.icon.font.clone();

    // Free-roam 3D Camera (Layer 0, order 0) starting at human eye height in the plaza center.
    commands.spawn((
        FlyCamera {
            yaw: 0.0,
            pitch: 0.04,
            speed: 3.2,
        },
        Camera3d::default(),
        RenderLayers::layer(0),
        Hdr,
        Bloom {
            intensity: 0.28,
            low_frequency_boost: 0.85,
            ..Bloom::NATURAL
        },
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.002, 0.003, 0.007)),
            ..default()
        },
        Transform::from_xyz(0.0, 1.30, 0.45),
    ));

    // Transparent 2D Screen HUD Camera (Layer 1, order 1) composited cleanly atop the 3D scene.
    commands.spawn((
        Camera2d,
        RenderLayers::layer(1),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));

    spawn_plaza_environment(&mut commands, &mut meshes, &mut materials, &mut images);
    spawn_floor_holo_compass(&mut commands, &font);
    spawn_sky_holo_crown(&mut commands, &font, &serif);

    spawn_north_deck(&mut commands, &font, &serif, &icon_font);
    spawn_east_lab(&mut commands, &font, &serif, &icon_font);
    spawn_south_vault(&mut commands, &font, &serif, &icon_font);
    spawn_west_console(&mut commands, &font, &serif, &icon_font);
    spawn_screen_settings_hud(&mut commands, &font, &icon_font);
}

fn hash_u32(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(374_761_393).wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn value_noise_2d(x: u32, y: u32, period: u32, cell_px: u32) -> f32 {
    let gx = x / cell_px;
    let gy = y / cell_px;
    let grid_cells = period / cell_px;
    let fx = (x % cell_px) as f32 / cell_px as f32;
    let fy = (y % cell_px) as f32 / cell_px as f32;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);

    let sample = |cx: u32, cy: u32| -> f32 {
        (hash_u32(cx % grid_cells, cy % grid_cells) & 0xFFFF) as f32 / 65535.0
    };
    let n00 = sample(gx, gy);
    let n10 = sample(gx + 1, gy);
    let n01 = sample(gx, gy + 1);
    let n11 = sample(gx + 1, gy + 1);
    let nx0 = n00 + (n10 - n00) * u;
    let nx1 = n01 + (n11 - n01) * u;
    nx0 + (nx1 - nx0) * v
}

fn make_tiled_image(width: u32, height: u32, data: Vec<u8>, format: TextureFormat) -> Image {
    let mut img = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::default(),
    );
    img.sampler
        .get_or_init_descriptor()
        .set_filter(ImageFilterMode::Linear)
        .set_address_mode(ImageAddressMode::Repeat);
    img
}

/// Generates seamless tiled architectural stone albedo, normal, and metallic-roughness maps.
fn create_floor_pbr_maps(images: &mut Assets<Image>) -> (Handle<Image>, Handle<Image>, Handle<Image>) {
    const N: u32 = 256;
    const SLAB: u32 = 128;

    let mut heights = vec![0.0f32; (N * N) as usize];
    let mut albedo = Vec::with_capacity((N * N * 4) as usize);
    let mut mr = Vec::with_capacity((N * N * 4) as usize);

    for y in 0..N {
        for x in 0..N {
            let tx = (x % SLAB) as f32 / SLAB as f32;
            let ty = (y % SLAB) as f32 / SLAB as f32;
            let edge = tx.min(1.0 - tx).min(ty.min(1.0 - ty));
            let s = (edge / 0.032).clamp(0.0, 1.0);
            let seam = s * s * (3.0 - 2.0 * s);

            let checker = (((x / SLAB) + (y / SLAB)) % 2) as f32;
            let macro_n = value_noise_2d(x, y, N, 32);
            let med_n = value_noise_2d(x, y, N, 8);
            let fine_n = (hash_u32(x, y) & 0xFFFF) as f32 / 65535.0;

            let height = seam * (0.72 + 0.14 * macro_n + 0.10 * med_n + 0.04 * fine_n);
            heights[(y * N + x) as usize] = height;

            // Tonal architectural slate variation so base_color shines cleanly with subtle stone grain.
            let tone = (0.76 + 0.06 * checker + 0.10 * macro_n + 0.06 * med_n + 0.04 * fine_n)
                * (0.62 + 0.38 * seam);
            let lum = (tone.clamp(0.35, 1.0) * 255.0) as u8;
            let warm = (lum as f32 * 0.98) as u8;
            let cool = (lum as f32 * 1.02).min(255.0) as u8;
            albedo.extend_from_slice(&[warm, lum, cool, 255]);

            // glTF metallic-roughness packing: G = perceptual roughness factor, B = metallic factor.
            let rough_var = if seam < 0.55 {
                0.98
            } else {
                0.78 + 0.14 * med_n + 0.08 * fine_n
            };
            let rough_byte = (rough_var.clamp(0.72, 1.0) * 255.0) as u8;
            mr.extend_from_slice(&[255, rough_byte, 255, 255]);
        }
    }

    let mut normals = Vec::with_capacity((N * N * 4) as usize);
    let bump_strength = 3.2;
    for y in 0..N {
        for x in 0..N {
            let xm = (x + N - 1) % N;
            let xp = (x + 1) % N;
            let ym = (y + N - 1) % N;
            let yp = (y + 1) % N;
            let dx = (heights[(y * N + xp) as usize] - heights[(y * N + xm) as usize]) * bump_strength;
            let dy = (heights[(yp * N + x) as usize] - heights[(ym * N + x) as usize]) * bump_strength;
            let n = Vec3::new(-dx, -dy, 1.0).normalize();
            normals.extend_from_slice(&[
                ((n.x * 0.5 + 0.5) * 255.0) as u8,
                ((n.y * 0.5 + 0.5) * 255.0) as u8,
                ((n.z * 0.5 + 0.5) * 255.0) as u8,
                255,
            ]);
        }
    }

    (
        images.add(make_tiled_image(N, N, albedo, TextureFormat::Rgba8UnormSrgb)),
        images.add(make_tiled_image(N, N, normals, TextureFormat::Rgba8Unorm)),
        images.add(make_tiled_image(N, N, mr, TextureFormat::Rgba8Unorm)),
    )
}

fn spawn_plaza_environment(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut Assets<Image>,
) {
    let (floor_albedo, floor_normal, floor_mr) = create_floor_pbr_maps(images);
    let mut floor_mesh = Plane3d::default().mesh().size(22.0, 22.0).build();
    let _ = floor_mesh.generate_tangents();

    // Colored, textured stone-slate floor with subtle tactile micro-roughness (adjustable in HUD).
    let floor_mat = materials.add(StandardMaterial {
        base_color: FLOOR_PRESETS[0].1,
        base_color_texture: Some(floor_albedo),
        normal_map_texture: Some(floor_normal),
        metallic_roughness_texture: Some(floor_mr),
        metallic: 0.06,
        perceptual_roughness: 0.58,
        reflectance: 0.52,
        uv_transform: Affine2::from_scale(Vec2::splat(11.0)),
        ..default()
    });
    commands.insert_resource(FloorMaterialHandle(floor_mat.clone()));
    commands.spawn((
        Mesh3d(meshes.add(floor_mesh)),
        MeshMaterial3d(floor_mat),
        Transform::IDENTITY,
    ));

    // Four non-emissive dark metallic corner monoliths framing the cyber plaza.
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.09, 0.12),
        metallic: 0.85,
        perceptual_roughness: 0.28,
        ..default()
    });
    let pillar_mesh = meshes.add(Cuboid::new(0.42, 3.6, 0.42));
    for (x, z) in [(-3.4, -3.4), (3.4, -3.4), (-3.4, 3.4), (3.4, 3.4)] {
        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(x, 1.8, z),
        ));
    }

    // Specular chrome/gold sculpture pedestal in the East Photonics zone catching SpotLight beams.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.45, 0.40, 0.45))),
        MeshMaterial3d(pillar_mat),
        Transform::from_xyz(2.15, 0.20, 0.0),
    ));
    commands.spawn((
        EastSculpture,
        Mesh3d(meshes.add(Cuboid::new(0.28, 0.28, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.82, 0.48),
            metallic: 0.96,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(2.15, 0.68, 0.0),
    ));

    // Matte timber + chrome blocks near North & South kiosks catching cast shadows.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.22))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.88, 0.92, 0.96),
            metallic: 0.95,
            perceptual_roughness: 0.10,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.22, -2.1),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.65, 0.30, 0.35))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.20, 0.11),
            metallic: 0.0,
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.15, 2.2),
    ));
}

fn spawn_floor_holo_compass(commands: &mut Commands, font: &Handle<Font>) {
    let dirs = [
        ("N // NEON COMMAND", Vec3::new(0.0, 0.015, -1.75), 0.0, CYAN),
        (
            "E // PHOTONICS LAB",
            Vec3::new(1.75, 0.015, 0.0),
            -FRAC_PI_2,
            MAGENTA,
        ),
        ("S // SOLAR VAULT", Vec3::new(0.0, 0.015, 1.75), PI, AMBER),
        (
            "W // NEURAL UPLINK",
            Vec3::new(-1.75, 0.015, 0.0),
            FRAC_PI_2,
            VIOLET,
        ),
    ];

    for (title, pos, yaw, accent) in dirs {
        let mut root_comp = world_root(Vec2::new(540.0, 110.0), 0.0020);
        root_comp.resolution_scale = 1.0;

        let rot = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-FRAC_PI_2);
        let pad = commands
            .spawn((
                root_comp,
                UNode {
                    width: UVal::Px(540.0),
                    height: UVal::Px(110.0),
                    background_color: Color::srgba(0.04, 0.06, 0.10, 0.90),
                    border_radius: UCornerRadius::all(16.0),
                    shape_mode: UShapeMode::Cut,
                    padding: USides::all(12.0),
                    ..default()
                },
                UBorder {
                    color: accent,
                    width: 2.0,
                    radius: UCornerRadius::all(16.0),
                    ..default()
                },
                UGradient::linear(
                    Color::srgba(0.06, 0.11, 0.18, 0.94),
                    accent.with_alpha(0.55),
                    0.0,
                ),
                UInnerGlow::new(accent.with_alpha(0.75), 12.0),
                UShadow::glow(hdr_glow(accent, 2.2, 0.65), 20.0),
                UPbr::satin(),
                ULightSource::lamp(accent, 14_000.0)
                    .with_range(4.2)
                    .with_emissive(0.55),
                PlazaLightFixture {
                    base_intensity: 14_000.0,
                },
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                    ..default()
                },
                Transform::from_translation(pos).with_rotation(rot),
            ))
            .id();

        commands.spawn((
            ChildOf(pad),
            UTextLabel {
                text: title.into(),
                font: font.clone(),
                font_size: 26.0,
                color: Color::WHITE,
                ..default()
            },
            UNode::default(),
            UPbr::neon(accent, 6.0),
        ));
    }
}

fn spawn_sky_holo_crown(commands: &mut Commands, font: &Handle<Font>, serif: &Handle<Font>) {
    // High overhead floating holographic canopy sign visible when looking up anywhere.
    let sky_root = commands
        .spawn((
            world_root(Vec2::new(980.0, 220.0), 0.0028),
            UNode {
                width: UVal::Px(980.0),
                height: UVal::Px(220.0),
                background_color: Color::srgba(0.08, 0.22, 0.36, 0.94),
                border_radius: UCornerRadius::all(26.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(20.0),
                ..default()
            },
            UBorder {
                color: CYAN,
                width: 2.5,
                radius: UCornerRadius::all(26.0),
                ..default()
            },
            UGradient::linear(
                Color::srgba(0.07, 0.24, 0.38, 0.94),
                Color::srgba(0.24, 0.08, 0.34, 0.94),
                0.25,
            ),
            UInnerGlow::new(MAGENTA.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(CYAN, 2.4, 0.72), 32.0),
            UPbr::new(0.16, 0.26),
            UPbrSurface::beveled(5.0, 0.55),
            screen_rect_light(Color::srgb(0.42, 0.86, 1.0), 170_000.0, 9.0, 0.36),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                gap: 8.0,
                ..default()
            },
            Transform::from_xyz(0.0, 3.45, 0.0).with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
        ))
        .id();

    commands.spawn((
        ChildOf(sky_root),
        UTextLabel {
            text: "CYBER LUMA // NIGHT SKYWAY".into(),
            font: serif.clone(),
            font_size: 56.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
        UPbr::neon(CYAN, 7.8),
    ));

    commands.spawn((
        ChildOf(sky_root),
        UTextLabel {
            text: "ساحة الليل الرقمية — كل الإضاءة صادرة من الواجهة والنصوص".into(),
            font: font.clone(),
            font_size: 22.0,
            color: MAGENTA,
            ..default()
        },
        UNode::default(),
        UPbr::neon(MAGENTA, 5.8),
    ));
}

// ---------------------------------------------------------------------------
// 1. NORTH MEGA-SCREEN: NEON COMMAND DECK (Grid + CyberShader + Bilingual)
// ---------------------------------------------------------------------------

fn spawn_north_deck(
    commands: &mut Commands,
    font: &Handle<Font>,
    serif: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    let root = commands
        .spawn((
            world_root(Vec2::new(1140.0, 680.0), SCREEN_MPU),
            UNode {
                width: UVal::Px(1140.0),
                height: UVal::Px(680.0),
                background_color: Color::srgba(0.05, 0.16, 0.24, 0.96),
                border_radius: UCornerRadius::all(22.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(24.0),
                ..default()
            },
            UBorder {
                color: CYAN,
                width: 2.5,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UGradient::radial(
                Color::srgba(0.08, 0.28, 0.38, 0.96),
                Color::srgba(0.03, 0.12, 0.20, 0.95),
            ),
            UInnerGlow::new(CYAN.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(CYAN, 1.8, 0.58), 32.0),
            UPbr::new(0.16, 0.28),
            UPbrSurface::beveled(6.0, 0.65),
            screen_rect_light(CYAN, 210_000.0, 10.0, 0.32),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 16.0,
                ..default()
            },
            Transform::from_xyz(0.0, 1.35, -SCREEN_RADIUS),
        ))
        .id();

    // Header powered by live GPU UShader::cyber_matrix + neon UTextLabel signage.
    let header = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(165.0),
                background_color: Color::srgb(0.05, 0.18, 0.28),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 16.0),
                ..default()
            },
            UShader::cyber_matrix(CYAN, MAGENTA, Color::srgb(0.05, 0.19, 0.29)),
            screen_panel_pbr(CYAN, 0.32),
            UInnerGlow::new(CYAN.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(CYAN, 2.2, 0.65), 24.0),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::Center,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(header),
        UTextLabel {
            text: "NORTH-01 // NIGHT COMMAND DECK".into(),
            font: serif.clone(),
            font_size: 46.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
        UPbr::neon(CYAN, 6.8),
    ));

    commands.spawn((
        ChildOf(header),
        BroadcastBannerLabel,
        UTextLabel {
            text: "شبكة المراقبة نشطة — LUMACORE GRID ONLINE".into(),
            font: font.clone(),
            font_size: 18.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
        UPbr::neon(Color::srgb(0.75, 0.96, 1.0), 4.2),
    ));

    // Middle: 3-Column CSS Grid of Interactive Control Modules.
    let grid = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(285.0),
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

    // Module 1: Overdrive Boost
    let btn_overdrive = spawn_cyber_tile(
        commands,
        grid,
        font,
        icon_font,
        Icon::ZAP,
        "NEON OVERDRIVE",
        "Boost bloom & lamp lumen output across the entire plaza",
        CYAN,
    );
    commands.entity(btn_overdrive).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.overdrive = !state.overdrive;
            state.ops_count += 1;
            let on = state.overdrive;
            state.push_log(format!(
                "[CMD] Neon Overdrive {}",
                if on { "ENGAGED (1.75x)" } else { "NORMAL" }
            ));
        },
    );

    // Module 2: City Grid Blackout Toggle
    let btn_blackout = spawn_cyber_tile(
        commands,
        grid,
        font,
        icon_font,
        Icon::POWER,
        "GRID BLACKOUT [B]",
        "Cut plaza emitter power to test darkness & light-locked UI",
        MAGENTA,
    );
    commands.entity(btn_blackout).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.blackout = !state.blackout;
            state.ops_count += 1;
            let off = state.blackout;
            state.push_log(format!(
                "[PWR] City Emitters {}",
                if off { "BLACKOUT" } else { "RESTORED" }
            ));
        },
    );

    // Module 3: Plaza Pulse Sync
    let btn_pulse = spawn_cyber_tile(
        commands,
        grid,
        font,
        icon_font,
        Icon::ACTIVITY,
        "SYNC SKYWAY",
        "Broadcast diagnostic ping to South log stream",
        EMERALD,
    );
    commands.entity(btn_pulse).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.ops_count += 1;
            let n = state.ops_count;
            state.broadcast = format!("PING #{n} // قطاع الشمال يرسل نبضة ضوئية للساحة");
            state.push_log(format!("[PING #{n}] North Command -> All Districts"));
        },
    );

    // Footer Telemetry Strip
    let footer = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(128.0),
                background_color: Color::srgb(0.06, 0.16, 0.25),
                border_radius: UCornerRadius::all(14.0),
                padding: USides::axes(20.0, 14.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.06, 0.18, 0.28),
                Color::srgb(0.04, 0.11, 0.20),
                0.1,
            ),
            screen_panel_pbr(CYAN, 0.24),
            UInnerGlow::new(CYAN.with_alpha(0.50), 8.0),
            UBorder {
                color: CYAN.with_alpha(0.65),
                width: 1.5,
                radius: UCornerRadius::all(14.0),
                ..default()
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
        ChildOf(footer),
        TelemetryHudLabel,
        UTextLabel {
            text: "CAM: (0.0, 1.3, 0.4) | ZERO AMBIENT ACTIVE".into(),
            font: font.clone(),
            font_size: 16.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(footer),
        UTextLabel {
            text: "[WASD] Fly   [Space/E & Ctrl/Q] Up/Down   [RMB Drag or V] Free Look   [1-4] Snap N/E/S/W   [B] Blackout"
                .into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.76, 0.85, 0.94),
            ..default()
        },
        UNode::default(),
    ));
}

// ---------------------------------------------------------------------------
// 2. EAST MEGA-SCREEN: PHOTONICS & OPTICAL LAB (Aurora + PBR + Modulator + Gel)
// ---------------------------------------------------------------------------

fn spawn_east_lab(
    commands: &mut Commands,
    font: &Handle<Font>,
    serif: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    let root = commands
        .spawn((
            world_root(Vec2::new(1140.0, 680.0), SCREEN_MPU),
            UNode {
                width: UVal::Px(1140.0),
                height: UVal::Px(680.0),
                background_color: Color::srgba(0.18, 0.06, 0.24, 0.96),
                border_radius: UCornerRadius::all(22.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(24.0),
                ..default()
            },
            UBorder {
                color: MAGENTA,
                width: 2.5,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UGradient::radial(
                Color::srgba(0.24, 0.08, 0.32, 0.96),
                Color::srgba(0.10, 0.04, 0.18, 0.95),
            ),
            UInnerGlow::new(MAGENTA.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(MAGENTA, 1.8, 0.58), 32.0),
            UPbr::new(0.16, 0.28),
            UPbrSurface::beveled(6.0, 0.7),
            screen_rect_light(MAGENTA, 195_000.0, 10.0, 0.30),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 16.0,
                ..default()
            },
            Transform::from_xyz(SCREEN_RADIUS, 1.35, 0.0)
                .with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
        ))
        .id();

    let header = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(150.0),
                background_color: Color::srgb(0.20, 0.07, 0.28),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 14.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.24, 0.08, 0.34),
                Color::srgb(0.08, 0.16, 0.34),
                0.35,
            ),
            UShader::aurora_beams(MAGENTA, CYAN, AMBER).with_blend_mode(UShaderBlendMode::Add),
            screen_panel_pbr(MAGENTA, 0.28),
            UInnerGlow::new(MAGENTA.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(MAGENTA, 2.2, 0.65), 24.0),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::Center,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    // UText casting a real shadow-enabled cycling SpotLight onto the gold sculpture & PBR cards!
    commands.spawn((
        ChildOf(header),
        EastPrismLight,
        UTextLabel {
            text: "EAST-02 // PHOTONICS & OPTICS".into(),
            font: serif.clone(),
            font_size: 44.0,
            color: MAGENTA,
            ..default()
        },
        UNode::default(),
        UPbr::neon(MAGENTA, 6.8),
        ULightSource::spotlight(MAGENTA, 380_000.0, 11.0, 0.65)
            .with_shadows(true)
            .with_emissive(6.8)
            .with_offset(Vec3::new(0.0, 0.0, 0.16)),
        ULightModulator::color_cycle(0.32, 0.92, 0.56),
        PlazaLightFixture {
            base_intensity: 380_000.0,
        },
    ));

    commands.spawn((
        ChildOf(header),
        EastModeLabel,
        UTextLabel {
            text: "WAVEFORM: PRISM COLOR-CYCLE | OPTICAL GEL: OFF".into(),
            font: font.clone(),
            font_size: 16.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    // Middle Row: Non-emissive Beveled PBR Material Swatches reacting purely to light.
    let swatch_row = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(265.0),
                background_color: Color::srgba(0.11, 0.09, 0.16, 0.94),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: MAGENTA.with_alpha(0.55),
                width: 1.5,
                radius: UCornerRadius::all(16.0),
                ..default()
            },
            screen_panel_pbr(MAGENTA, 0.14),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Stretch,
                gap: 14.0,
                ..default()
            },
        ))
        .id();

    for (name, color, pbr, surf) in [
        (
            "OAK WOOD",
            Color::srgb(0.55, 0.34, 0.16),
            UPbr::wood(),
            UPbrSurface::wood_plank(),
        ),
        (
            "CHROME",
            Color::srgb(0.86, 0.90, 0.96),
            UPbr::chrome(),
            UPbrSurface::beveled(6.0, 0.8),
        ),
        (
            "24K GOLD",
            Color::srgb(0.98, 0.78, 0.24),
            UPbr::gold(),
            UPbrSurface::beveled(6.0, 0.8),
        ),
        (
            "CYBER GLASS",
            Color::srgba(0.35, 0.88, 1.0, 0.48),
            UPbr::glass(),
            UPbrSurface::glass(),
        ),
        (
            "BASALT STONE",
            Color::srgb(0.26, 0.28, 0.32),
            UPbr::stone(),
            UPbrSurface::beveled(5.0, 0.6),
        ),
    ] {
        let swatch = commands
            .spawn((
                ChildOf(swatch_row),
                UNode {
                    width: UVal::Px(195.0),
                    height: UVal::Percent(1.0),
                    background_color: color,
                    border_radius: UCornerRadius::all(12.0),
                    shape_mode: UShapeMode::Cut,
                    padding: USides::all(14.0),
                    ..default()
                },
                pbr,
                surf,
                ULayout {
                    display: UDisplay::Flex,
                    flex_direction: UFlexDirection::Column,
                    justify_content: UJustifyContent::SpaceBetween,
                    ..default()
                },
            ))
            .id();

        commands.spawn((
            ChildOf(swatch),
            UTextLabel {
                text: name.into(),
                font: font.clone(),
                font_size: 16.0,
                color: Color::WHITE,
                ..default()
            },
            UNode::default(),
            UPbr::matte(),
        ));

        commands.spawn((
            ChildOf(swatch),
            UTextLabel {
                text: "Zero Emissive\nPBR Specular".into(),
                font: font.clone(),
                font_size: 13.0,
                color: Color::srgb(0.90, 0.92, 0.96),
                ..default()
            },
            UNode::default(),
            UPbr::matte(),
        ));
    }

    // Bottom Interactive Optics Controls.
    let controls = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(180.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![UTrackSize::Fr(1.0), UTrackSize::Fr(1.0)],
                gap: 16.0,
                ..default()
            },
        ))
        .id();

    let btn_wave = spawn_cyber_tile(
        commands,
        controls,
        font,
        icon_font,
        Icon::RADIO,
        "CYCLE MODULATOR",
        "Switch East SpotLight: Prism -> Flicker -> Pulse -> Steady",
        MAGENTA,
    );
    commands.entity(btn_wave).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.east_mode_idx = (state.east_mode_idx + 1) % 4;
            state.ops_count += 1;
            let idx = state.east_mode_idx;
            state.push_log(format!("[OPTIC] Modulator mode -> #{idx}"));
        },
    );

    let btn_gel = spawn_cyber_tile(
        commands,
        controls,
        font,
        icon_font,
        Icon::SUN,
        "TOGGLE OPTICAL GEL",
        "Slide cyan/amber ULightFilter stained lens into the East beam",
        CYAN,
    );
    commands.entity(btn_gel).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.east_gel_enabled = !state.east_gel_enabled;
            state.ops_count += 1;
            let on = state.east_gel_enabled;
            state.push_log(format!(
                "[GEL] East Optical Filter {}",
                if on { "INSERTED" } else { "CLEARED" }
            ));
        },
    );

    // Floating movable ULightFilter optical gel plate hovering in front of the East SpotLight!
    let gel_plate = commands
        .spawn((
            EastGelLens,
            world_root(Vec2::new(420.0, 260.0), 0.0022),
            UNode {
                width: UVal::Px(420.0),
                height: UVal::Px(260.0),
                background_color: Color::srgba(0.10, 0.95, 0.60, 0.36),
                border_radius: UCornerRadius::all(20.0),
                ..default()
            },
            UBorder {
                color: EMERALD,
                width: 2.0,
                radius: UCornerRadius::all(20.0),
                ..default()
            },
            UPbr::glass(),
            UPbrSurface::glass().with_bevel(5.0, 0.8),
            ULightFilter::gel(Color::srgb(0.15, 1.0, 0.55)),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            Transform::from_xyz(2.95, 2.55, 0.0).with_rotation(Quat::from_rotation_y(-FRAC_PI_2)),
        ))
        .id();

    commands.spawn((
        ChildOf(gel_plate),
        UTextLabel {
            text: "OPTICAL FILTER GEL".into(),
            font: font.clone(),
            font_size: 20.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));
}

// ---------------------------------------------------------------------------
// 3. SOUTH MEGA-SCREEN: SOLAR VAULT (Plasma + Occluder + URequiresLight + Scroll)
// ---------------------------------------------------------------------------

fn spawn_south_vault(
    commands: &mut Commands,
    font: &Handle<Font>,
    serif: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    let root = commands
        .spawn((
            world_root(Vec2::new(1140.0, 680.0), SCREEN_MPU),
            UNode {
                width: UVal::Px(1140.0),
                height: UVal::Px(680.0),
                background_color: Color::srgba(0.20, 0.10, 0.04, 0.96),
                border_radius: UCornerRadius::all(22.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(24.0),
                ..default()
            },
            UBorder {
                color: AMBER,
                width: 2.5,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UGradient::radial(
                Color::srgba(0.26, 0.14, 0.04, 0.96),
                Color::srgba(0.12, 0.06, 0.02, 0.95),
            ),
            UInnerGlow::new(AMBER.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(AMBER, 1.8, 0.58), 32.0),
            UPbr::new(0.16, 0.28),
            UPbrSurface::beveled(6.0, 0.65),
            screen_rect_light(AMBER, 195_000.0, 10.0, 0.30),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 16.0,
                ..default()
            },
            Transform::from_xyz(0.0, 1.35, SCREEN_RADIUS).with_rotation(Quat::from_rotation_y(PI)),
        ))
        .id();

    let header = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(140.0),
                background_color: Color::srgb(0.22, 0.10, 0.04),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 14.0),
                ..default()
            },
            UShader::plasma_energy(AMBER, MAGENTA, Color::srgb(0.24, 0.10, 0.04)),
            screen_panel_pbr(AMBER, 0.30),
            UInnerGlow::new(AMBER.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(AMBER, 2.2, 0.65), 24.0),
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
            text: "SOUTH-03 // SOLAR VAULT".into(),
            font: serif.clone(),
            font_size: 44.0,
            color: AMBER,
            ..default()
        },
        UNode::default(),
        UPbr::neon(AMBER, 6.5),
        ULightSource::spotlight(AMBER, 340_000.0, 11.0, 0.72)
            .with_shadows(true)
            .with_emissive(6.5)
            .with_offset(Vec3::new(0.0, 0.0, 0.16)),
        PlazaLightFixture {
            base_intensity: 340_000.0,
        },
    ));

    // Shutter toggle control
    let shutter_btn = spawn_cyber_tile(
        commands,
        header,
        font,
        icon_font,
        Icon::SHIELD,
        "ARMOR SHUTTER",
        "Drop SDF Occluder to block South beam",
        AMBER,
    );
    commands.entity(shutter_btn).insert(UNode {
        width: UVal::Px(320.0),
        height: UVal::Px(105.0),
        background_color: Color::srgba(0.22, 0.14, 0.06, 0.95),
        border_radius: UCornerRadius::all(12.0),
        padding: USides::all(12.0),
        ..default()
    });
    commands.entity(shutter_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.south_shutter_closed = !state.south_shutter_closed;
            state.ops_count += 1;
            let closed = state.south_shutter_closed;
            state.push_log(format!(
                "[VAULT] Armor Shutter {}",
                if closed { "CLOSED" } else { "OPENED" }
            ));
        },
    );

    // Split Body: Left = Light-Dependent URequiresLight Switches; Right = Scrollable Event Feed.
    let split = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(460.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![UTrackSize::Fr(1.0), UTrackSize::Fr(1.15)],
                gap: 18.0,
                ..default()
            },
        ))
        .id();

    // Left: Light-Locked Security Controls (only work when illuminated!)
    let left_col = commands
        .spawn((
            ChildOf(split),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::srgba(0.12, 0.07, 0.04, 0.94),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: AMBER.with_alpha(0.55),
                width: 1.5,
                radius: UCornerRadius::all(16.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.14, 0.08, 0.04),
                Color::srgb(0.08, 0.04, 0.03),
                0.3,
            ),
            screen_panel_pbr(AMBER, 0.12),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 12.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(left_col),
        UTextLabel {
            text: "SOLAR-GATED OVERRIDES (Requires Light)".into(),
            font: font.clone(),
            font_size: 16.0,
            color: AMBER,
            ..default()
        },
        UNode::default(),
    ));

    for (label, code) in [
        ("OVERRIDE GATE A", "AUTH-A"),
        ("DECRYPT ARCHIVE", "DEC-09"),
        ("FLUSH CAPACITOR", "CAP-X"),
    ] {
        let mut sw = commands.spawn((
            ChildOf(left_col),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(104.0),
                background_color: Color::srgb(0.14, 0.36, 0.24),
                border_radius: UCornerRadius::all(12.0),
                padding: USides::all(14.0),
                ..default()
            },
            UBorder {
                color: EMERALD,
                width: 2.0,
                radius: UCornerRadius::all(12.0),
                ..default()
            },
            UPbr::satin(),
            UPbrSurface::beveled(5.0, 0.65),
            UInteraction::default(),
            UInteractionColors {
                normal: Color::srgb(0.14, 0.36, 0.24),
                hovered: Color::srgb(0.20, 0.56, 0.38),
                pressed: Color::srgb(0.10, 0.26, 0.16),
            },
            UInteractionScale::default(),
            UFocusable::new(),
            UFocusVisual::ring(AMBER).with_contract(4.0, 0.0),
            ULightSensor::default().with_threshold(60.0),
            URequiresLight::with_min_illuminance(60.0),
            ULightReactiveVisual {
                dark_color: Color::srgb(0.08, 0.05, 0.05),
                lit_color: Color::srgb(0.16, 0.42, 0.28),
                lit_emissive_boost: 0.95,
                adopt_incident_color: false,
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ));
        sw.observe(
            move |click: On<PointerClick>,
                  reqs: Query<&URequiresLight>,
                  mut state: ResMut<PlazaState>| {
                if reqs
                    .get(click.event_target())
                    .is_ok_and(URequiresLight::allows_interaction)
                {
                    state.ops_count += 1;
                    state.push_log(format!("[VAULT] Executed {label} ({code})"));
                }
            },
        );
        sw.observe(
            move |trig: On<ULightAvailabilityChanged>, mut state: ResMut<PlazaState>| {
                let active = trig.event().is_active;
                state.push_log(format!(
                    "[SENSOR] {code} {}",
                    if active { "ONLINE" } else { "LOCKED (DARK)" }
                ));
            },
        );
        let sw_id = sw.id();
        commands.spawn((
            ChildOf(sw_id),
            UTextLabel {
                text: label.into(),
                font: font.clone(),
                font_size: 16.0,
                color: Color::WHITE,
                ..default()
            },
            UNode::default(),
        ));
        commands.spawn((
            ChildOf(sw_id),
            UTextLabel {
                text: code.into(),
                font: font.clone(),
                font_size: 14.0,
                color: EMERALD,
                ..default()
            },
            UNode::default(),
        ));
    }

    commands.spawn((
        ULightSource {
            direction_sign: -1.0,
            ..ULightSource::spotlight(AMBER, 420_000.0, 4.5, 0.72).with_shadows(true)
        },
        PlazaLightFixture {
            base_intensity: 420_000.0,
        },
        Transform::from_xyz(0.55, 1.35, 2.25).looking_at(Vec3::new(0.55, 1.10, 3.85), Vec3::Y),
    ));

    // Movable Armored SDF Occluder plate sitting in front of that South Solar beam!
    let shutter = commands
        .spawn((
            SouthShutterRig,
            world_root(Vec2::new(560.0, 320.0), 0.0022),
            UNode {
                width: UVal::Px(560.0),
                height: UVal::Px(320.0),
                background_color: Color::srgb(0.14, 0.14, 0.16),
                border_radius: UCornerRadius::all(18.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(18.0),
                ..default()
            },
            UBorder {
                color: AMBER,
                width: 2.0,
                radius: UCornerRadius::all(18.0),
                ..default()
            },
            UPbr::stone(),
            UPbrSurface::beveled(6.0, 0.7),
            ULightOccluder::solid(),
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            Transform::from_xyz(0.55, 2.65, 3.05).with_rotation(Quat::from_rotation_y(PI)),
        ))
        .id();

    commands.spawn((
        ChildOf(shutter),
        UTextLabel {
            text: "SDF ARMOR SHUTTER".into(),
            font: font.clone(),
            font_size: 24.0,
            color: AMBER,
            ..default()
        },
        UNode::default(),
    ));

    // Right Column: Scrollable Cyber Log Stream (`UScrollContainer` + `UScrollKineticDrag`)!
    let scroll_wrap = commands
        .spawn((
            ChildOf(split),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::srgb(0.12, 0.07, 0.04),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(16.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.14, 0.09, 0.04),
                Color::srgb(0.07, 0.04, 0.03),
                0.2,
            ),
            screen_panel_pbr(AMBER, 0.22),
            UInnerGlow::new(AMBER.with_alpha(0.45), 10.0),
            UBorder {
                color: AMBER.with_alpha(0.65),
                width: 1.5,
                radius: UCornerRadius::all(16.0),
                ..default()
            },
            UScrollContainer::vertical(),
            UScrollKineticDrag::default(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(scroll_wrap),
        UTextLabel {
            text: "KINETIC LOG STREAM (Scroll / Drag in 3D)".into(),
            font: font.clone(),
            font_size: 16.0,
            color: AMBER,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(scroll_wrap),
        ScrollLogLabel,
        UTextLabel {
            text: "Initializing log stream...".into(),
            font: font.clone(),
            font_size: 14.0,
            color: Color::srgb(0.92, 0.90, 0.85),
            wrap_mode: UTextWrapMode::WrapToParent,
            ..default()
        },
        UNode {
            width: UVal::Percent(1.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// 4. WEST MEGA-SCREEN: NEURAL UPLINK (TextInput + Radial Reactor + Gradients)
// ---------------------------------------------------------------------------

fn spawn_west_console(
    commands: &mut Commands,
    font: &Handle<Font>,
    serif: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    let root = commands
        .spawn((
            world_root(Vec2::new(1140.0, 680.0), SCREEN_MPU),
            UNode {
                width: UVal::Px(1140.0),
                height: UVal::Px(680.0),
                background_color: Color::srgba(0.14, 0.07, 0.28, 0.96),
                border_radius: UCornerRadius::all(22.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(24.0),
                ..default()
            },
            UBorder {
                color: VIOLET,
                width: 2.5,
                radius: UCornerRadius::all(22.0),
                ..default()
            },
            UGradient::radial(
                Color::srgba(0.18, 0.09, 0.34, 0.96),
                Color::srgba(0.08, 0.04, 0.18, 0.95),
            ),
            UInnerGlow::new(VIOLET.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(VIOLET, 1.8, 0.58), 32.0),
            UPbr::new(0.16, 0.28),
            UPbrSurface::beveled(6.0, 0.65),
            screen_rect_light(VIOLET, 210_000.0, 10.5, 0.32),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 16.0,
                ..default()
            },
            Transform::from_xyz(-SCREEN_RADIUS, 1.35, 0.0)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2)),
        ))
        .id();

    let header = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(140.0),
                background_color: Color::srgb(0.14, 0.07, 0.26),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::axes(24.0, 14.0),
                ..default()
            },
            UShader::liquid_chrome(VIOLET, CYAN, Color::WHITE),
            screen_panel_pbr(VIOLET, 0.28),
            UInnerGlow::new(VIOLET.with_alpha(0.75), 14.0),
            UShadow::glow(hdr_glow(VIOLET, 2.2, 0.65), 24.0),
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
        ChildOf(header),
        UTextLabel {
            text: "WEST-04 // NEURAL UPLINK".into(),
            font: serif.clone(),
            font_size: 44.0,
            color: VIOLET,
            ..default()
        },
        UNode::default(),
        UPbr::neon(VIOLET, 6.8),
        ULightSource::bulb(VIOLET, 78_000.0, 7.0)
            .with_emissive(6.8)
            .with_offset(Vec3::new(0.0, 0.0, 0.16)),
        PlazaLightFixture {
            base_intensity: 78_000.0,
        },
    ));

    commands.spawn((
        ChildOf(header),
        UTextLabel {
            text: "Interactive 3D Terminal: Click field, type ('overdrive', 'blackout', 'prism', 'pulse', or any message) + Enter"
                .into(),
            font: font.clone(),
            font_size: 15.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    // Split body: Left = Live 3D TextInput & Presets; Right = UDisplay::Radial Reactor + Conic UGradient.
    let body = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(460.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Grid,
                grid_template_columns: vec![UTrackSize::Fr(1.2), UTrackSize::Fr(1.0)],
                gap: 18.0,
                ..default()
            },
        ))
        .id();

    // Left panel: UTextInput + Quick Gradient Spectrum Strip.
    let left = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::srgba(0.10, 0.06, 0.22, 0.94),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(20.0),
                ..default()
            },
            UBorder {
                color: VIOLET.with_alpha(0.55),
                width: 1.5,
                radius: UCornerRadius::all(16.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.11, 0.06, 0.24),
                Color::srgb(0.06, 0.03, 0.14),
                0.25,
            ),
            screen_panel_pbr(VIOLET, 0.22),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 16.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(left),
        UTextLabel {
            text: "LIVE COMMAND TERMINAL (3D UTextInput)".into(),
            font: font.clone(),
            font_size: 16.0,
            color: VIOLET,
            ..default()
        },
        UNode::default(),
    ));

    spawn_cyber_text_input(commands, left, font);

    // Multi-stop Gradient Bar demonstrating linear_stops + neon edge
    commands.spawn((
        ChildOf(left),
        UNode {
            width: UVal::Percent(1.0),
            height: UVal::Px(56.0),
            background_color: Color::WHITE,
            border_radius: UCornerRadius::all(12.0),
            ..default()
        },
        UGradient::linear_stops(
            0.0,
            [
                UGradientStop::new(0.0, CYAN),
                UGradientStop::new(0.35, VIOLET),
                UGradientStop::new(0.7, MAGENTA),
                UGradientStop::new(1.0, AMBER),
            ],
        ),
        UInnerGlow::new(CYAN, 10.0),
        UShadow::glow(hdr_glow(VIOLET, 2.2, 0.65), 18.0),
    ));

    let quick_btn = spawn_cyber_tile(
        commands,
        left,
        font,
        icon_font,
        Icon::TERMINAL,
        "BOOST REACTOR SPIN",
        "Accelerate radial core frequency",
        VIOLET,
    );
    commands.entity(quick_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.radial_speed = if state.radial_speed > 2.0 { 1.0 } else { 3.5 };
            state.ops_count += 1;
            let spd = state.radial_speed;
            state.push_log(format!("[NEURAL] Radial reactor spin -> {spd:.1}x"));
        },
    );

    // Right panel: UDisplay::Radial Core + Conic UGradient orb.
    let right = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: Color::srgba(0.10, 0.06, 0.22, 0.94),
                border_radius: UCornerRadius::all(16.0),
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: CYAN.with_alpha(0.55),
                width: 1.5,
                radius: UCornerRadius::all(16.0),
                ..default()
            },
            UGradient::linear(
                Color::srgb(0.11, 0.06, 0.24),
                Color::srgb(0.05, 0.08, 0.22),
                0.35,
            ),
            screen_panel_pbr(VIOLET, 0.22),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(right),
        UTextLabel {
            text: "RADIAL SYNAPSE RING (UDisplay::Radial)".into(),
            font: font.clone(),
            font_size: 16.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
    ));

    let radial_ring = commands
        .spawn((
            ChildOf(right),
            ReactorCoreSpinner,
            UNode {
                width: UVal::Px(330.0),
                height: UVal::Px(330.0),
                background_color: Color::srgba(0.16, 0.09, 0.34, 0.90),
                border_radius: UCornerRadius::all(165.0),
                ..default()
            },
            UBorder {
                color: CYAN.with_alpha(0.75),
                width: 2.0,
                radius: UCornerRadius::all(165.0),
                ..default()
            },
            UGradient::conic_stops([
                UGradientStop::new(0.0, CYAN.with_alpha(0.68)),
                UGradientStop::new(0.5, MAGENTA.with_alpha(0.78)),
                UGradientStop::new(1.0, VIOLET.with_alpha(0.68)),
            ]),
            UInnerGlow::new(CYAN.with_alpha(0.85), 18.0),
            screen_panel_pbr(VIOLET, 0.34),
            ULayout {
                display: UDisplay::Radial,
                ..default()
            },
        ))
        .id();

    for (idx, col) in [CYAN, MAGENTA, VIOLET, EMERALD, AMBER, CYAN]
        .into_iter()
        .enumerate()
    {
        let pip = commands
            .spawn((
                ChildOf(radial_ring),
                UNode {
                    width: UVal::Px(64.0),
                    height: UVal::Px(64.0),
                    background_color: col,
                    border_radius: UCornerRadius::all(18.0),
                    shape_mode: UShapeMode::Cut,
                    ..default()
                },
                UBorder {
                    color: Color::WHITE,
                    width: 1.5,
                    radius: UCornerRadius::all(18.0),
                    ..default()
                },
                UPbr::neon(col, 6.2),
                ULayout {
                    display: UDisplay::Flex,
                    justify_content: UJustifyContent::Center,
                    align_items: UAlignItems::Center,
                    ..default()
                },
            ))
            .id();

        commands.spawn((
            ChildOf(pip),
            UTextLabel {
                text: format!("N{idx}"),
                font: font.clone(),
                font_size: 16.0,
                color: Color::BLACK,
                ..default()
            },
            UNode::default(),
        ));
    }
}

// ---------------------------------------------------------------------------
// Reusable Cyber Tile & 3D TextInput Helpers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_cyber_tile(
    commands: &mut Commands,
    parent: Entity,
    font: &Handle<Font>,
    icon_font: &Handle<Font>,
    icon: &str,
    title: &str,
    subtitle: &str,
    accent: Color,
) -> Entity {
    let acc = accent.to_srgba();
    let base_bg = Color::srgba(
        0.06 + acc.red * 0.16,
        0.07 + acc.green * 0.16,
        0.11 + acc.blue * 0.16,
        0.95,
    );
    let hover_bg = Color::srgba(
        0.11 + acc.red * 0.26,
        0.13 + acc.green * 0.26,
        0.18 + acc.blue * 0.26,
        0.98,
    );
    let press_bg = Color::srgba(
        0.04 + acc.red * 0.10,
        0.05 + acc.green * 0.10,
        0.08 + acc.blue * 0.10,
        0.98,
    );

    let tile = commands
        .spawn((
            ChildOf(parent),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                background_color: base_bg,
                border_radius: UCornerRadius::all(14.0),
                shape_mode: UShapeMode::Cut,
                padding: USides::all(16.0),
                ..default()
            },
            UBorder {
                color: accent.with_alpha(0.85),
                width: 2.0,
                radius: UCornerRadius::all(14.0),
                ..default()
            },
            UInnerGlow::new(accent.with_alpha(0.65), 8.0),
            UTransition::bouncy(220.0, 0.85),
            UInteraction::default(),
            UInteractionColors {
                normal: base_bg,
                hovered: hover_bg,
                pressed: press_bg,
            },
            UInteractionScale::default(),
            UFocusable::new(),
            UFocusVisual::ring(accent).with_contract(4.0, 0.0),
            screen_panel_pbr(accent, 0.26),
            UPbrSurface::beveled(4.5, 0.55),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    let top = commands
        .spawn((
            ChildOf(tile),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                align_items: UAlignItems::Center,
                gap: 10.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(top),
        UTextLabel {
            text: icon.into(),
            font: icon_font.clone(),
            font_size: 24.0,
            color: accent,
            ..default()
        },
        UNode::default(),
        UPbr::neon(accent, 5.2),
    ));

    commands.spawn((
        ChildOf(top),
        UTextLabel {
            text: title.into(),
            font: font.clone(),
            font_size: 18.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(tile),
        UTextLabel {
            text: subtitle.into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::srgb(0.75, 0.84, 0.94),
            wrap_mode: UTextWrapMode::WrapToParent,
            ..default()
        },
        UNode {
            width: UVal::Percent(1.0),
            ..default()
        },
    ));

    tile
}

fn spawn_cyber_text_input(commands: &mut Commands, parent: Entity, font: &Handle<Font>) {
    let font_size = 18.0f32;
    let field = commands
        .spawn((
            ChildOf(parent),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(54.0),
                background_color: Color::srgba(0.11, 0.08, 0.22, 0.96),
                border_radius: UCornerRadius::all(10.0),
                padding: USides::axes(16.0, 10.0),
                ..default()
            },
            UBorder {
                color: VIOLET,
                width: 2.0,
                radius: UCornerRadius::all(10.0),
                ..default()
            },
            screen_panel_pbr(VIOLET, 0.22),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                ..default()
            },
            UFocusable::new(),
            UFocusVisual::border(CYAN, 2.5),
            UInteraction::default(),
            UTextInput {
                placeholder: "Type command & press Enter...".into(),
                value: String::new(),
                mode: TextInputMode::Normal,
                filter: TextInputFilter::SingleLine,
                font_size,
                clear_on_submit: true,
                ..default()
            },
            UTextInputCaret {
                color: CYAN,
                width: 2.5,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(field),
        UTextInputSelectionMarker,
        UNode {
            width: UVal::Px(0.0),
            height: UVal::Px(font_size * 1.25),
            background_color: Color::srgba(0.0, 0.72, 1.0, 0.35),
            border_radius: UCornerRadius::all(2.0),
            ..default()
        },
        USelf {
            position_type: UPositionType::Absolute,
            left: UVal::Px(16.0),
            ..default()
        },
        Visibility::Hidden,
    ));

    commands.spawn((
        ChildOf(field),
        UTextInputTextMarker,
        UTextLabel {
            text: "Type command & press Enter...".into(),
            font: font.clone(),
            font_size,
            color: Color::srgba(0.60, 0.70, 0.85, 0.75),
            ..default()
        },
        UNode::default(),
    ));

    commands.spawn((
        ChildOf(field),
        UTextInputCaretMarker,
        UNode {
            width: UVal::Px(2.5),
            height: UVal::Px(font_size * 1.25),
            background_color: CYAN,
            border_radius: UCornerRadius::all(1.0),
            ..default()
        },
        USelf {
            position_type: UPositionType::Absolute,
            left: UVal::Px(16.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

// ---------------------------------------------------------------------------
// Interactive & Animation Systems
// ---------------------------------------------------------------------------

fn on_cyber_text_submitted(trigger: On<UTextInputSubmit>, mut state: ResMut<PlazaState>) {
    let raw = trigger.event().value.trim().to_string();
    if raw.is_empty() {
        return;
    }
    let lower = raw.to_ascii_lowercase();
    match lower.as_str() {
        "overdrive" | "boost" => {
            state.overdrive = true;
            state.blackout = false;
            state.broadcast = "CMD: NEON OVERDRIVE ENGAGED".into();
        }
        "blackout" | "dark" => {
            state.blackout = !state.blackout;
            state.broadcast = if state.blackout {
                "CMD: CITY BLACKOUT ACTIVE".into()
            } else {
                "CMD: GRID POWER RESTORED".into()
            };
        }
        "prism" | "flicker" | "pulse" | "steady" => {
            state.east_mode_idx = match lower.as_str() {
                "prism" => 0,
                "flicker" => 1,
                "pulse" => 2,
                _ => 3,
            };
            state.broadcast = format!("CMD: EAST PHOTONICS -> {}", raw.to_uppercase());
        }
        "shutter" => {
            state.south_shutter_closed = !state.south_shutter_closed;
            state.broadcast = "CMD: SOUTH ARMOR SHUTTER TOGGLED".into();
        }
        _ => {
            state.broadcast = raw.clone();
        }
    }
    state.ops_count += 1;
    state.push_log(format!("[UPLINK] > {raw}"));
}

fn apply_camera_snap(idx: usize, fly: &mut FlyCamera, tf: &mut Transform) {
    match idx {
        0 => {
            fly.yaw = 0.0;
            fly.pitch = 0.0;
        }
        1 => {
            fly.yaw = -FRAC_PI_2;
            fly.pitch = 0.0;
        }
        2 => {
            fly.yaw = PI;
            fly.pitch = 0.0;
        }
        3 => {
            fly.yaw = FRAC_PI_2;
            fly.pitch = 0.0;
        }
        _ => {
            tf.translation = Vec3::new(0.0, 1.30, 0.45);
            fly.yaw = 0.0;
            fly.pitch = 0.04;
        }
    }
    tf.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
}

#[allow(clippy::too_many_arguments)]
fn control_free_camera(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mut state: ResMut<PlazaState>,
    focus_state: Res<UFocusState>,
    text_inputs: Query<Entity, With<UTextInput>>,
    mut cam_q: Query<(&mut FlyCamera, &mut Transform)>,
) {
    let Ok((mut fly, mut tf)) = cam_q.single_mut() else {
        return;
    };

    if let Some(snap_idx) = state.pending_snap.take() {
        apply_camera_snap(snap_idx, &mut fly, &mut tf);
    }

    let typing_active = focus_state.focused.is_some_and(|e| text_inputs.contains(e));
    if typing_active {
        return;
    }

    let dt = time.delta_secs();
    let look_active = mouse_buttons.pressed(MouseButton::Right) || state.free_look_toggle;
    if look_active && mouse_motion.delta != Vec2::ZERO {
        let sens = 0.0028;
        fly.yaw -= mouse_motion.delta.x * sens;
        fly.pitch = (fly.pitch - mouse_motion.delta.y * sens).clamp(-1.35, 1.35);
    }

    // Smooth keyboard yaw/pitch look keys (J / L / I / K) as an accessible alternative to mouse drag.
    let key_turn = 1.65 * dt;
    if keyboard.pressed(KeyCode::KeyJ) {
        fly.yaw += key_turn;
    }
    if keyboard.pressed(KeyCode::KeyL) {
        fly.yaw -= key_turn;
    }
    if keyboard.pressed(KeyCode::KeyI) {
        fly.pitch = (fly.pitch + key_turn).clamp(-1.35, 1.35);
    }
    if keyboard.pressed(KeyCode::KeyK) {
        fly.pitch = (fly.pitch - key_turn).clamp(-1.35, 1.35);
    }

    // Quick directional snaps [1 = North, 2 = East, 3 = South, 4 = West, R = Reset center].
    for (idx, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::KeyR,
    ]
    .into_iter()
    .enumerate()
    {
        if keyboard.just_pressed(key) {
            apply_camera_snap(idx, &mut fly, &mut tf);
            break;
        }
    }

    tf.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);

    // Full 6-DoF directional flight vector + vertical ascent/descent.
    let mut move_dir = Vec3::ZERO;
    let forward = tf.forward().as_vec3();
    let right = tf.right().as_vec3();

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        move_dir += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        move_dir -= forward;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_dir += right;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_dir -= right;
    }
    if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::Space) {
        move_dir += Vec3::Y;
    }
    if keyboard.pressed(KeyCode::KeyQ) || keyboard.pressed(KeyCode::ControlLeft) {
        move_dir -= Vec3::Y;
    }

    if move_dir.length_squared() > 1.0e-4 {
        let boost = if keyboard.pressed(KeyCode::ShiftLeft) {
            2.5
        } else {
            1.0
        };
        tf.translation += move_dir.normalize() * fly.speed * boost * dt;
        tf.translation.y = tf.translation.y.clamp(0.24, 9.5);
        tf.translation.x = tf.translation.x.clamp(-9.5, 9.5);
        tf.translation.z = tf.translation.z.clamp(-9.5, 9.5);
    }
}

#[allow(clippy::type_complexity)]
fn animate_plaza_elements(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    focus_state: Res<UFocusState>,
    text_inputs: Query<Entity, With<UTextInput>>,
    mut state: ResMut<PlazaState>,
    mut sculpture_q: Query<&mut Transform, With<EastSculpture>>,
    mut shutter_q: Query<
        &mut Transform,
        (
            With<SouthShutterRig>,
            Without<EastSculpture>,
            Without<EastGelLens>,
        ),
    >,
    mut gel_q: Query<
        (&mut Transform, &mut ULightFilter),
        (
            With<EastGelLens>,
            Without<EastSculpture>,
            Without<SouthShutterRig>,
        ),
    >,
    mut spinner_q: Query<&mut UGradient, With<ReactorCoreSpinner>>,
) {
    let dt = time.delta_secs();
    let typing = focus_state.focused.is_some_and(|e| text_inputs.contains(e));

    if !typing {
        if keyboard.just_pressed(KeyCode::KeyV) {
            state.free_look_toggle = !state.free_look_toggle;
        }
        if keyboard.just_pressed(KeyCode::KeyH) {
            state.hud_expanded = !state.hud_expanded;
        }
        if keyboard.just_pressed(KeyCode::KeyB) {
            state.blackout = !state.blackout;
            let off = state.blackout;
            state.push_log(format!(
                "[KEY B] Plaza Emitters {}",
                if off { "BLACKOUT" } else { "ONLINE" }
            ));
        }
    }

    if let Ok(mut tf) = sculpture_q.single_mut() {
        tf.rotate_y(0.7 * dt);
        tf.rotate_x(0.35 * dt);
    }

    let target_shutter_y = if state.south_shutter_closed { 1.15 } else { 2.65 };
    let lerp = (dt * 7.5).min(1.0);
    state.shutter_offset_y += (target_shutter_y - state.shutter_offset_y) * lerp;
    if let Ok(mut tf) = shutter_q.single_mut() {
        tf.translation.y = state.shutter_offset_y;
    }

    let target_gel_y = if state.east_gel_enabled { 1.98 } else { 2.95 };
    if let Ok((mut tf, mut filter)) = gel_q.single_mut() {
        tf.translation.y += (target_gel_y - tf.translation.y) * lerp;
        filter.enabled = state.east_gel_enabled;
    }

    if let Ok(mut grad) = spinner_q.single_mut() {
        grad.offset = (grad.offset + 0.16 * state.radial_speed * dt).fract();
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_plaza_ui_and_lights(
    state: Res<PlazaState>,
    cam_q: Query<&Transform, With<FlyCamera>>,
    mut fixtures: Query<(&PlazaLightFixture, &mut ULightSource)>,
    mut east_mod_q: Query<&mut ULightModulator, With<EastPrismLight>>,
    mut broadcast_q: Query<&mut UTextLabel, With<BroadcastBannerLabel>>,
    mut hud_q: Query<
        &mut UTextLabel,
        (With<TelemetryHudLabel>, Without<BroadcastBannerLabel>),
    >,
    mut log_q: Query<
        &mut UTextLabel,
        (
            With<ScrollLogLabel>,
            Without<BroadcastBannerLabel>,
            Without<TelemetryHudLabel>,
        ),
    >,
    mut east_label_q: Query<
        &mut UTextLabel,
        (
            With<EastModeLabel>,
            Without<BroadcastBannerLabel>,
            Without<TelemetryHudLabel>,
            Without<ScrollLogLabel>,
        ),
    >,
) {
    let dimmer = state.light_scale * if state.overdrive { 1.75 } else { 1.0 };
    for (fix, mut src) in fixtures.iter_mut() {
        src.enabled = !state.blackout;
        src.intensity = fix.base_intensity;
        src.dimmer = dimmer;
    }

    let mode_name = match state.east_mode_idx {
        0 => "PRISM COLOR-CYCLE",
        1 => "CYBER FLICKER",
        2 => "NEURAL PULSE",
        _ => "STEADY BEAM",
    };
    if let Ok(mut mod_c) = east_mod_q.single_mut() {
        mod_c.mode = match state.east_mode_idx {
            0 => ULightModulationMode::ColorCycle {
                speed: 0.32,
                saturation: 0.92,
                lightness: 0.56,
            },
            1 => ULightModulationMode::Flicker {
                speed: 14.0,
                depth: 0.60,
            },
            2 => ULightModulationMode::Pulse {
                speed: 2.2,
                min_factor: 0.25,
                max_factor: 1.5,
            },
            _ => ULightModulationMode::Steady,
        };
    }

    if let Ok(mut banner) = broadcast_q.single_mut() {
        banner.text = state.broadcast.clone();
    }

    if let (Ok(cam_tf), Ok(mut hud)) = (cam_q.single(), hud_q.single_mut()) {
        let p = cam_tf.translation;
        hud.text = format!(
            "POS: ({:.1}, {:.1}, {:.1}) | MODE: {} | OVERDRIVE: {} | FREE-LOOK[V]: {}",
            p.x,
            p.y,
            p.z,
            if state.blackout { "BLACKOUT" } else { "ONLINE" },
            if state.overdrive { "ON" } else { "OFF" },
            if state.free_look_toggle { "ON" } else { "RMB" },
        );
    }

    if let Ok(mut log_lbl) = log_q.single_mut() {
        log_lbl.text = state.logs.join("\n");
    }

    if let Ok(mut el) = east_label_q.single_mut() {
        el.text = format!(
            "WAVEFORM: {} | OPTICAL GEL: {}",
            mode_name,
            if state.east_gel_enabled {
                "ENGAGED (TINTED)"
            } else {
                "BYPASSED"
            }
        );
    }
}

// ---------------------------------------------------------------------------
// 2D Screen HUD Settings Overlay
// ---------------------------------------------------------------------------

fn hud_chip_button(
    commands: &mut Commands,
    parent: Entity,
    font: &Handle<Font>,
    label: &str,
    accent: Color,
    swatch_color: Option<Color>,
) -> Entity {
    let acc = accent.to_srgba();
    let normal_bg = Color::srgba(
        0.08 + acc.red * 0.14,
        0.10 + acc.green * 0.14,
        0.16 + acc.blue * 0.14,
        0.92,
    );
    let hover_bg = Color::srgba(
        0.14 + acc.red * 0.24,
        0.16 + acc.green * 0.24,
        0.24 + acc.blue * 0.24,
        0.98,
    );
    let press_bg = Color::srgba(0.05, 0.07, 0.11, 0.98);

    let chip = commands
        .spawn((
            ChildOf(parent),
            UNode {
                height: UVal::Px(28.0),
                padding: USides::axes(9.0, 4.0),
                background_color: normal_bg,
                border_radius: UCornerRadius::all(8.0),
                ..default()
            },
            UBorder {
                color: accent.with_alpha(0.55),
                width: 1.25,
                radius: UCornerRadius::all(8.0),
                ..default()
            },
            UTransition::bouncy(220.0, 0.85),
            UInteraction::default(),
            UInteractionColors {
                normal: normal_bg,
                hovered: hover_bg,
                pressed: press_bg,
            },
            UInteractionScale::default(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                justify_content: UJustifyContent::Center,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    if let Some(swatch) = swatch_color {
        commands.spawn((
            ChildOf(chip),
            UNode {
                width: UVal::Px(11.0),
                height: UVal::Px(11.0),
                background_color: swatch,
                border_radius: UCornerRadius::all(6.0),
                ..default()
            },
            UBorder {
                color: Color::WHITE.with_alpha(0.8),
                width: 1.0,
                radius: UCornerRadius::all(6.0),
                ..default()
            },
        ));
    }

    commands.spawn((
        ChildOf(chip),
        UTextLabel {
            text: label.into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    chip
}

fn spawn_hud_section_header(
    commands: &mut Commands,
    parent: Entity,
    font: &Handle<Font>,
    title: &str,
    color: Color,
) {
    commands.spawn((
        ChildOf(parent),
        UTextLabel {
            text: title.into(),
            font: font.clone(),
            font_size: 11.0,
            color,
            ..default()
        },
        UNode::default(),
    ));
}

fn spawn_screen_settings_hud(
    commands: &mut Commands,
    font: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    // Full-viewport transparent Screen Root (clicks outside dock cards pass through to 3D UI).
    let hud_root = commands
        .spawn((
            URootUi::screen(),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                padding: USides::all(16.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Start,
                ..default()
            },
        ))
        .id();

    // Top-Left compact status badge
    let status_pill = commands
        .spawn((
            ChildOf(hud_root),
            UNode {
                padding: USides::axes(14.0, 8.0),
                background_color: Color::srgba(0.05, 0.08, 0.14, 0.84),
                border_radius: UCornerRadius::all(12.0),
                ..default()
            },
            UBorder {
                color: CYAN.with_alpha(0.55),
                width: 1.25,
                radius: UCornerRadius::all(12.0),
                ..default()
            },
            UShadow::glow(Color::srgba(0.06, 0.72, 0.90, 0.25), 14.0),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(status_pill),
        UTextLabel {
            text: Icon::COMPASS.into(),
            font: icon_font.clone(),
            font_size: 15.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
    ));
    commands.spawn((
        ChildOf(status_pill),
        HudStatusText,
        UTextLabel {
            text: "CYBER PLAZA HUD | [WASD] Fly  [RMB/V] Look  [H] Settings".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::srgb(0.86, 0.94, 1.0),
            ..default()
        },
        UNode::default(),
    ));

    // Top-Right docked Settings HUD Panel
    let dock = commands
        .spawn((
            ChildOf(hud_root),
            UNode {
                width: UVal::Px(326.0),
                padding: USides::all(12.0),
                background_color: Color::srgba(0.05, 0.08, 0.14, 0.88),
                border_radius: UCornerRadius::all(14.0),
                ..default()
            },
            UBorder {
                color: CYAN.with_alpha(0.60),
                width: 1.5,
                radius: UCornerRadius::all(14.0),
                ..default()
            },
            UShadow::glow(Color::srgba(0.08, 0.78, 1.0, 0.28), 18.0),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 10.0,
                ..default()
            },
        ))
        .id();

    // Header Row with collapse toggle
    let dock_header = commands
        .spawn((
            ChildOf(dock),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    let title_grp = commands
        .spawn((
            ChildOf(dock_header),
            UNode {
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 7.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(title_grp),
        UTextLabel {
            text: Icon::SETTINGS.into(),
            font: icon_font.clone(),
            font_size: 15.0,
            color: CYAN,
            ..default()
        },
        UNode::default(),
    ));
    commands.spawn((
        ChildOf(title_grp),
        UTextLabel {
            text: "PLAZA SETTINGS HUD".into(),
            font: font.clone(),
            font_size: 13.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));

    let toggle_btn = hud_chip_button(commands, dock_header, font, "Hide [H]", CYAN, None);
    commands.entity(toggle_btn).despawn_children();
    commands.spawn((
        ChildOf(toggle_btn),
        HudToggleLabel,
        UTextLabel {
            text: "Hide [H]".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));
    commands.entity(toggle_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.hud_expanded = !state.hud_expanded;
        },
    );

    // Collapsible Settings Body
    let body = commands
        .spawn((
            ChildOf(dock),
            HudDrawerBody,
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 9.0,
                ..default()
            },
        ))
        .id();

    // --- Section 1: Floor Material & Roughness ---
    spawn_hud_section_header(commands, body, font, "FLOOR MATERIAL & ROUGHNESS", CYAN);

    let floor_row = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                flex_wrap: UFlexWrap::Wrap,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    for (idx, (name, swatch)) in FLOOR_PRESETS.into_iter().enumerate() {
        let chip = hud_chip_button(commands, floor_row, font, name, swatch, Some(swatch));
        commands.entity(chip).insert(HudFloorPresetChip(idx));
        commands.entity(chip).observe(
            move |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
                state.floor_preset_idx = idx;
                state.push_log(format!("[HUD] Floor -> {name}"));
            },
        );
    }

    // Roughness stepper row
    let rough_row = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(rough_row),
        HudFloorReadout,
        UTextLabel {
            text: "Teal Slate".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::srgb(0.75, 0.90, 0.96),
            ..default()
        },
        UNode::default(),
    ));

    let rough_ctrl = commands
        .spawn((
            ChildOf(rough_row),
            UNode {
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                align_items: UAlignItems::Center,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    let r_minus = hud_chip_button(commands, rough_ctrl, font, "Rough -", AMBER, None);
    commands.entity(r_minus).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.floor_roughness = (state.floor_roughness - 0.08).clamp(0.20, 0.92);
        },
    );
    commands.spawn((
        ChildOf(rough_ctrl),
        HudRoughnessReadout,
        UTextLabel {
            text: "0.58".into(),
            font: font.clone(),
            font_size: 12.0,
            color: AMBER,
            ..default()
        },
        UNode::default(),
    ));
    let r_plus = hud_chip_button(commands, rough_ctrl, font, "Rough +", AMBER, None);
    commands.entity(r_plus).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.floor_roughness = (state.floor_roughness + 0.08).clamp(0.20, 0.92);
        },
    );

    // --- Section 2: Lighting, RectLight Power, Bloom & Ambient ---
    spawn_hud_section_header(commands, body, font, "RECT LIGHTS, BLOOM & AMBIENT", MAGENTA);

    let light_row = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(light_row),
        HudLightReadout,
        UTextLabel {
            text: "RectLights: 1.00x".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::srgb(0.88, 0.92, 0.98),
            ..default()
        },
        UNode::default(),
    ));

    let light_btns = commands
        .spawn((
            ChildOf(light_row),
            UNode {
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    let l_minus = hud_chip_button(commands, light_btns, font, " - ", MAGENTA, None);
    commands.entity(l_minus).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.light_scale = (state.light_scale - 0.25).clamp(0.25, 2.25);
        },
    );
    let l_plus = hud_chip_button(commands, light_btns, font, " + ", MAGENTA, None);
    commands.entity(l_plus).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.light_scale = (state.light_scale + 0.25).clamp(0.25, 2.25);
        },
    );
    let blk_btn = hud_chip_button(commands, light_btns, font, "Blackout", AMBER, None);
    commands.entity(blk_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.blackout = !state.blackout;
            let off = state.blackout;
            state.push_log(format!(
                "[HUD] Plaza Power {}",
                if off { "BLACKOUT" } else { "ONLINE" }
            ));
        },
    );

    let fx_row = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 6.0,
                ..default()
            },
        ))
        .id();

    let bloom_btn = hud_chip_button(commands, fx_row, font, "Bloom: Normal", VIOLET, None);
    commands.entity(bloom_btn).despawn_children();
    commands.spawn((
        ChildOf(bloom_btn),
        HudBloomReadout,
        UTextLabel {
            text: "Bloom: Normal (0.28)".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));
    commands.entity(bloom_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.bloom_mode_idx = (state.bloom_mode_idx + 1) % 3;
        },
    );

    let amb_btn = hud_chip_button(commands, fx_row, font, "Ambient: Dark", EMERALD, None);
    commands.entity(amb_btn).despawn_children();
    commands.spawn((
        ChildOf(amb_btn),
        HudAmbientReadout,
        UTextLabel {
            text: "Ambient: Dark (0)".into(),
            font: font.clone(),
            font_size: 12.0,
            color: Color::WHITE,
            ..default()
        },
        UNode::default(),
    ));
    commands.entity(amb_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
            state.soft_ambient = !state.soft_ambient;
        },
    );

    // --- Section 3: Camera Quick Snap ---
    spawn_hud_section_header(commands, body, font, "CAMERA SNAP VIEWS", EMERALD);

    let snap_row = commands
        .spawn((
            ChildOf(body),
            UNode {
                width: UVal::Percent(1.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 5.0,
                ..default()
            },
        ))
        .id();

    for (idx, (lbl, col)) in [
        ("North [1]", CYAN),
        ("East [2]", MAGENTA),
        ("South [3]", AMBER),
        ("West [4]", VIOLET),
        ("Center [R]", EMERALD),
    ]
    .into_iter()
    .enumerate()
    {
        let btn = hud_chip_button(commands, snap_row, font, lbl, col, None);
        commands.entity(btn).observe(
            move |_click: On<PointerClick>, mut state: ResMut<PlazaState>| {
                state.pending_snap = Some(idx);
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_hud_settings(
    state: Res<PlazaState>,
    floor_handle: Option<Res<FloorMaterialHandle>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut cam_q: Query<(&Transform, &mut Bloom), With<FlyCamera>>,
    mut drawer_q: Query<&mut Visibility, With<HudDrawerBody>>,
    mut chips_q: Query<(&HudFloorPresetChip, &mut UBorder)>,
    mut hud_texts: ParamSet<(
        Query<&mut UTextLabel, With<HudStatusText>>,
        Query<&mut UTextLabel, With<HudToggleLabel>>,
        Query<&mut UTextLabel, With<HudFloorReadout>>,
        Query<&mut UTextLabel, With<HudRoughnessReadout>>,
        Query<&mut UTextLabel, With<HudLightReadout>>,
        Query<&mut UTextLabel, With<HudBloomReadout>>,
        Query<&mut UTextLabel, With<HudAmbientReadout>>,
    )>,
) {
    let (preset_name, preset_color) = FLOOR_PRESETS[state.floor_preset_idx % FLOOR_PRESETS.len()];

    if let Some(handle) = floor_handle
        && let Some(mut mat) = materials.get_mut(&handle.0)
        && (mat.base_color != preset_color
            || (mat.perceptual_roughness - state.floor_roughness).abs() > 1.0e-4)
    {
        mat.base_color = preset_color;
        mat.perceptual_roughness = state.floor_roughness;
    }

    let target_ambient = if state.soft_ambient { 140.0 } else { 0.0 };
    if (ambient.brightness - target_ambient).abs() > 1.0e-2 {
        ambient.color = Color::srgb(0.68, 0.80, 1.0);
        ambient.brightness = target_ambient;
    }

    let (bloom_name, bloom_val) = match state.bloom_mode_idx % 3 {
        0 => ("Off", 0.0),
        1 => ("Normal", 0.28),
        _ => ("High", 0.55),
    };

    let cam_pos = if let Ok((tf, mut bloom)) = cam_q.single_mut() {
        if (bloom.intensity - bloom_val).abs() > 1.0e-4 {
            bloom.intensity = bloom_val;
        }
        tf.translation
    } else {
        Vec3::ZERO
    };

    if let Ok(mut vis) = drawer_q.single_mut() {
        *vis = if state.hud_expanded {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    for (chip, mut border) in &mut chips_q {
        let active = chip.0 == state.floor_preset_idx;
        let base_col = FLOOR_PRESETS[chip.0].1;
        border.color = if active {
            Color::WHITE
        } else {
            base_col.with_alpha(0.55)
        };
        border.width = if active { 2.2 } else { 1.25 };
    }

    if let Ok(mut lbl) = hud_texts.p0().single_mut() {
        lbl.text = format!(
            "PLAZA HUD | POS ({:.1},{:.1},{:.1}) | {} | [H] Settings",
            cam_pos.x,
            cam_pos.y,
            cam_pos.z,
            if state.blackout { "BLACKOUT" } else { "ONLINE" },
        );
    }
    if let Ok(mut lbl) = hud_texts.p1().single_mut() {
        lbl.text = if state.hud_expanded {
            "Hide [H]".into()
        } else {
            "Open [H]".into()
        };
    }
    if let Ok(mut lbl) = hud_texts.p2().single_mut() {
        lbl.text = format!("Color: {preset_name}");
    }
    if let Ok(mut lbl) = hud_texts.p3().single_mut() {
        lbl.text = format!("{:.2}", state.floor_roughness);
    }
    if let Ok(mut lbl) = hud_texts.p4().single_mut() {
        let eff = state.light_scale * if state.overdrive { 1.75 } else { 1.0 };
        lbl.text = format!("Area Lights: {eff:.2}x");
    }
    if let Ok(mut lbl) = hud_texts.p5().single_mut() {
        lbl.text = format!("Bloom: {bloom_name} ({bloom_val:.2})");
    }
    if let Ok(mut lbl) = hud_texts.p6().single_mut() {
        lbl.text = if state.soft_ambient {
            "Ambient: Soft".into()
        } else {
            "Ambient: Dark".into()
        };
    }
}
