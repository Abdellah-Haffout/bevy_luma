//! Complete Multi-Camera, Multi-Window, RenderLayers & Parley Text Input Showcase.
//!
//! Demonstrates all major multi-camera and rich text-input capabilities in one interactive scene:
//! 1. **Smart `UiCameraRef::Auto` disambiguation**: `Camera3d` (Layer 0) and `Camera2d` (Layer 1) coexist
//!    on the primary window while `URootUi::world_3d` and `URootUi::screen` resolve automatically.
//! 2. **Offscreen `RenderTarget::Image` isolation**: An offscreen `Camera2d` (Layer 2) renders a live
//!    animated HUD card into a texture (`UImage`) without causing `AutoAmbiguousCamera` conflicts or
//!    stealing pointer picking.
//! 3. **Hierarchical `RenderLayers` propagation**: Camera layers automatically propagate to roots,
//!    descendant `UNode`s, `TextChildMarker`, `UTextInput` viewports/carets, and highlight quads.
//! 4. **Multi-Window picking & IME isolation**: Toggle a secondary OS window with its own `Camera2d`
//!    (`Layer 3`) and isolated `UTextInput`, pointer picking, and `Window.ime_enabled`/`ime_position`.
//! 5. **Parley-powered `UTextInput`**:
//!    - Single-line horizontal viewport scrolling (`UTextInputViewportMarker` / `UClip`)
//!    - Variable-width double-click word selection & drag selection
//!    - Multiline editing (`Enter` newline, `Ctrl/Cmd+Enter` submit, `ArrowUp`/`ArrowDown`, pooled highlights)
//!    - Arabic RTL click/caret accuracy (`noto_sans_arabic_regular`) + `IME` status telemetry
//!    - Interactive `UTextInput` inside a 3D world-space panel (`URootUi::world_3d`)

use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::window::{PrimaryWindow, WindowRef};
use bevy_luma::layout::layout_system::{ResolvedRootUi, UiCanvasSize};
use bevy_luma::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Luma UI — Complete Multi-Camera, RenderLayers & Parley Text Showcase".into(),
                resolution: (1280, 820).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LumaPlugin)
        .init_resource::<ShowcaseState>()
        .init_resource::<SecondaryWindowState>()
        .add_systems(Startup, setup_showcase)
        .add_systems(
            Update,
            (
                configure_demo_inputs,
                animate_3d_sculpture_and_offscreen_card,
                reconcile_secondary_window,
                sync_telemetry_labels,
            ),
        )
        .add_observer(on_demo_text_changed)
        .add_observer(on_demo_text_submit)
        .run();
}

// ---------------------------------------------------------------------------
// Resources & Component Markers
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct ShowcaseState {
    last_event: String,
    submit_count: u32,
    rotation_boost: f32,
    world_clicks: u32,
    focused_name: String,
    camera_entities: Option<CameraSummary>,
}

impl Default for ShowcaseState {
    fn default() -> Self {
        Self {
            last_event: "Ready — click any field (2D, 3D, or 2nd Window) and start typing".into(),
            submit_count: 0,
            rotation_boost: 0.0,
            world_clicks: 0,
            focused_name: "None".into(),
            camera_entities: None,
        }
    }
}

#[derive(Clone, Copy)]
struct CameraSummary {
    cam_3d: Entity,
    cam_2d: Entity,
    cam_offscreen: Entity,
    root_3d: Entity,
    root_2d: Entity,
    root_offscreen: Entity,
}

#[derive(Resource, Default)]
struct SecondaryWindowState {
    window: Option<Entity>,
    camera: Option<Entity>,
    root: Option<Entity>,
}

#[derive(Component)]
struct DemoInputSpec {
    name: &'static str,
    font: Handle<Font>,
    width: UVal,
    height: f32,
}

#[derive(Component)]
struct DemoInputConfigured;

#[derive(Component)]
struct RotatingSculpture {
    base_y: f32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum TelemetryRole {
    CameraRouting,
    Status,
    WindowButton,
    WorldCounter,
}

#[derive(Component)]
struct OffscreenPulseDot;

#[derive(Component)]
struct OffscreenClockText;

// ---------------------------------------------------------------------------
// Startup Setup: 3 Cameras + 3D Sculpture + Offscreen RTT + 2D HUD + 3D Panel
// ---------------------------------------------------------------------------

fn setup_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<ShowcaseState>,
    theme: Res<Theme>,
) {
    let body_font = theme.text.font.inter_regular.clone();
    let arabic_font = theme.text.font.noto_sans_arabic_regular.clone();

    // 1. Primary 3D Camera (Layer 0, order 0) — renders 3D meshes + World3d UI panel.
    // Offset slightly toward -X so the 3D scene frames neatly in the open right 55% beside the HUD.
    let cam_3d = commands
        .spawn((
            Camera3d::default(),
            RenderLayers::layer(0),
            AmbientLight {
                color: Color::srgb(0.85, 0.90, 1.0),
                brightness: 900.0,
                ..default()
            },
            Camera {
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.03, 0.04, 0.07)),
                ..default()
            },
            Transform::from_xyz(-0.42, 0.14, 1.65).looking_at(Vec3::new(-0.42, 0.02, 0.0), Vec3::Y),
        ))
        .id();

    // 2. Primary 2D HUD Camera (Layer 1, order 1) — transparent pass overlaid atop 3D.
    // Even with Camera3d + Offscreen Camera present, `URootUi::screen()` auto-resolves here.
    let cam_2d = commands
        .spawn((
            Camera2d,
            RenderLayers::layer(1),
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id();

    // 3. Offscreen RTT Camera (Layer 2, order -1) — excluded from Auto camera resolution
    // and primary-window pointer picking, explicitly targeted via `UiCameraRef::Entity`.
    let offscreen_image = images.add(Image::new_target_texture(
        480,
        84,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let cam_offscreen = commands
        .spawn((
            Camera2d,
            RenderLayers::layer(2),
            Camera {
                order: -1,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.05, 0.07, 0.12)),
                ..default()
            },
            RenderTarget::Image(offscreen_image.clone().into()),
        ))
        .id();

    // 3D Lighting & Rotating Metallic Torus (Layer 0)
    commands.spawn((
        DirectionalLight {
            illuminance: 28_000.0,
            color: Color::srgb(0.95, 0.97, 1.0),
            ..default()
        },
        Transform::from_xyz(1.5, 3.0, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(0),
    ));
    commands.spawn((
        PointLight {
            intensity: 140_000.0,
            range: 5.0,
            color: Color::srgb(0.15, 0.85, 1.0),
            ..default()
        },
        Transform::from_xyz(-0.4, 0.8, 0.9),
        RenderLayers::layer(0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.06, 0.09),
            perceptual_roughness: 0.35,
            metallic: 0.15,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.60, 0.0),
        RenderLayers::layer(0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.24, 0.12))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.13, 0.19),
            perceptual_roughness: 0.28,
            metallic: 0.4,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.54, 0.0),
        RenderLayers::layer(0),
    ));
    commands.spawn((
        RotatingSculpture { base_y: -0.34 },
        Mesh3d(meshes.add(Torus::new(0.052, 0.135))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.88, 0.72),
            metallic: 0.90,
            perceptual_roughness: 0.20,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.34, 0.0)
            .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.5, 0.3, 0.1)),
        RenderLayers::layer(0),
    ));

    // Spawn Offscreen RTT Mini HUD (inherits Layer 2 automatically from cam_offscreen)
    let root_offscreen = spawn_offscreen_card(&mut commands, cam_offscreen, body_font.clone());

    // Spawn Interactive 3D World Panel (Auto-resolves to cam_3d -> inherits Layer 0)
    let root_3d = spawn_world_3d_panel(&mut commands, body_font.clone());

    // Spawn Main 2D HUD Studio Sidebar (Auto-resolves to cam_2d -> inherits Layer 1)
    let root_2d = spawn_screen_studio_hud(
        &mut commands,
        body_font,
        arabic_font,
        offscreen_image,
    );

    state.camera_entities = Some(CameraSummary {
        cam_3d,
        cam_2d,
        cam_offscreen,
        root_3d,
        root_2d,
        root_offscreen,
    });
}

fn spawn_offscreen_card(commands: &mut Commands, cam_offscreen: Entity, font: Handle<Font>) -> Entity {
    let root = commands
        .spawn((
            URootUi {
                camera: UiCameraRef::Entity(cam_offscreen),
                canvas: UiCanvasSize::Fixed(Vec2::new(480.0, 84.0)),
                ..URootUi::screen()
            },
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                padding: USides::axes(16.0, 12.0),
                background_color: Color::srgba(0.07, 0.11, 0.18, 1.0),
                border_radius: UCornerRadius::all(10.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.20, 0.75, 0.95, 0.65),
                width: 1.5,
                radius: UCornerRadius::all(10.0),
                offset: 0.0,
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                gap: 12.0,
                ..default()
            },
        ))
        .id();

    let left_col = commands
        .spawn((
            ChildOf(root),
            UNode::default(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 4.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(left_col),
        UTextLabel {
            text: "OFFSCREEN RENDER TARGET (Layer 2 via UiCameraRef::Entity)".into(),
            font_size: 11.5,
            font: font.clone(),
            color: Color::srgb(0.42, 0.90, 1.0),
            ..default()
        },
    ));
    commands.spawn((
        ChildOf(left_col),
        OffscreenClockText,
        UTextLabel {
            text: "Live offscreen pass • 0.0s • isolated from Auto & picking".into(),
            font_size: 12.5,
            font,
            color: Color::srgb(0.86, 0.91, 0.98),
            ..default()
        },
    ));

    commands.spawn((
        ChildOf(root),
        OffscreenPulseDot,
        UNode {
            width: UVal::Px(24.0),
            height: UVal::Px(24.0),
            border_radius: UCornerRadius::all(12.0),
            background_color: Color::srgb(0.18, 0.88, 0.72),
            ..default()
        },
    ));

    root
}

fn spawn_world_3d_panel(commands: &mut Commands, font: Handle<Font>) -> Entity {
    let mut world_root = URootUi::world_3d(Vec2::new(460.0, 320.0));
    world_root.meters_per_unit = 0.0017;

    let root = commands
        .spawn((
            world_root,
            Transform::from_xyz(0.0, 0.16, 0.0).with_rotation(Quat::from_rotation_y(-0.16)),
            UNode {
                width: UVal::Px(460.0),
                height: UVal::Px(320.0),
                padding: USides::all(22.0),
                background_color: Color::srgba(0.06, 0.09, 0.15, 0.93),
                border_radius: UCornerRadius::all(16.0),
                ..default()
            },
            UBorder {
                color: Color::srgb(0.22, 0.78, 1.0),
                width: 2.0,
                radius: UCornerRadius::all(16.0),
                offset: 0.0,
            },
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
        ChildOf(root),
        UTextLabel {
            text: "3D Spatial UI (UiCameraRef::Auto -> Camera3d • Layer 0)".into(),
            font_size: 14.5,
            font: font.clone(),
            color: Color::srgb(0.45, 0.88, 1.0),
            ..default()
        },
    ));
    commands.spawn((
        ChildOf(root),
        UTextLabel {
            text: "Click directly inside 3D world space below to type, drag-select, or scroll:"
                .into(),
            font_size: 12.0,
            font: font.clone(),
            color: Color::srgb(0.75, 0.82, 0.92),
            wrap_mode: UTextWrapMode::WrapToParent,
            linebreak: LineBreak::WordBoundary,
            ..default()
        },
    ));

    spawn_demo_input(
        commands,
        root,
        "3D World Panel Input",
        UTextInput::new("Type inside 3D world space...")
            .with_value("3D perspective Parley caret & viewport clipping work identically!")
            .with_font_size(14.0),
        font.clone(),
        UVal::Percent(1.0),
        44.0,
    );

    let action_row = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Percent(1.0),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::SpaceBetween,
                align_items: UAlignItems::Center,
                gap: 12.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(action_row),
        TelemetryRole::WorldCounter,
        UTextLabel {
            text: "3D Clicks: 0".into(),
            font_size: 13.0,
            font: font.clone(),
            color: Color::srgb(0.85, 0.92, 0.98),
            ..default()
        },
    ));

    let boost_btn = commands
        .spawn((
            ChildOf(action_row),
            UNode {
                width: UVal::Px(150.0),
                height: UVal::Px(38.0),
                border_radius: UCornerRadius::all(8.0),
                background_color: Color::srgb(0.14, 0.56, 0.84),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            UFocusable::new(),
            UFocusVisual::border(Color::srgb(0.4, 0.95, 1.0), 2.0),
            UInteraction::default(),
            UInteractionColors {
                normal: Color::srgb(0.14, 0.56, 0.84),
                hovered: Color::srgb(0.22, 0.68, 0.96),
                pressed: Color::srgb(0.10, 0.42, 0.68),
            },
        ))
        .id();

    commands.spawn((
        ChildOf(boost_btn),
        UTextLabel {
            text: "Boost 3D Spin".into(),
            font_size: 13.5,
            font,
            color: Color::WHITE,
            ..default()
        },
    ));
    commands.entity(boost_btn).observe(
        |_click: On<PointerClick>, mut state: ResMut<ShowcaseState>| {
            state.world_clicks += 1;
            state.rotation_boost += 4.5;
            state.last_event = format!("3D button clicked! Total 3D clicks: {}", state.world_clicks);
        },
    );

    root
}

fn spawn_screen_studio_hud(
    commands: &mut Commands,
    body_font: Handle<Font>,
    arabic_font: Handle<Font>,
    offscreen_image: Handle<Image>,
) -> Entity {
    // Transparent full-viewport Screen Root (lets pointer hits pass through open right side to 3D UI!)
    let root = commands
        .spawn((
            URootUi::screen(),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                padding: USides::all(18.0),
                background_color: Color::NONE,
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Row,
                justify_content: UJustifyContent::Start,
                align_items: UAlignItems::Stretch,
                ..default()
            },
        ))
        .id();

    // Left Studio Dock Panel
    let sidebar = commands
        .spawn((
            ChildOf(root),
            UNode {
                width: UVal::Px(530.0),
                height: UVal::Percent(1.0),
                padding: USides::all(18.0),
                background_color: Color::srgba(0.055, 0.070, 0.110, 0.92),
                border_radius: UCornerRadius::all(14.0),
                ..default()
            },
            UBorder {
                color: Color::srgba(0.28, 0.42, 0.68, 0.45),
                width: 1.5,
                radius: UCornerRadius::all(14.0),
                offset: 0.0,
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::SpaceBetween,
                gap: 10.0,
                ..default()
            },
        ))
        .id();

    // Header
    let header = commands
        .spawn((
            ChildOf(sidebar),
            UNode::default(),
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 4.0,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        ChildOf(header),
        UTextLabel {
            text: "Luma UI — Multi-Camera, RenderLayers & Parley Editor".into(),
            font_size: 16.5,
            font: body_font.clone(),
            color: Color::WHITE,
            ..default()
        },
    ));
    commands.spawn((
        ChildOf(header),
        TelemetryRole::CameraRouting,
        UTextLabel {
            text: "Resolving camera routing...".into(),
            font_size: 11.5,
            font: body_font.clone(),
            color: Color::srgb(0.48, 0.85, 1.0),
            wrap_mode: UTextWrapMode::WrapToParent,
            linebreak: LineBreak::WordBoundary,
            ..default()
        },
    ));

    // 1. Horizontal Overflow Single-Line Input
    spawn_labeled_input_block(
        commands,
        sidebar,
        "1) Single-Line Clipped Viewport + Horizontal Caret Scroll",
        "Narrow viewport: Parley click/drag/double-click keeps caret visible horizontally even with iiii vs WWWW variable glyphs!",
        "Single-Line Scroll",
        false,
        UVal::Px(340.0),
        40.0,
        body_font.clone(),
    );

    // 2. Multiline Editor Input
    spawn_labeled_input_block(
        commands,
        sidebar,
        "2) Multiline Editor (Enter = newline • Ctrl+Enter = submit • Up/Down)",
        "Line 1: Parley vertical caret geometry & pooled selection quads.\nLine 2: Press Enter to insert a new line.\nLine 3: Drag vertically to select across multiple lines!\nLine 4: Scroll automatically follows caret vertically.\nLine 5: Press Ctrl+Enter to trigger UTextInputSubmit.",
        "Multiline Parley Editor",
        true,
        UVal::Percent(1.0),
        120.0,
        body_font.clone(),
    );

    // 3. Arabic RTL + IME Input
    spawn_labeled_input_block(
        commands,
        sidebar,
        "3) Arabic RTL (Parley Origin Normalized) & Live Platform IME",
        "مرحبا بالعالم في محرر لومة — جرّب النقر والتحديد المزدوج",
        "Arabic RTL & IME",
        false,
        UVal::Percent(1.0),
        42.0,
        arabic_font,
    );

    // 4. Live Offscreen RTT Preview via UImage
    let rtt_wrapper = commands
        .spawn((
            ChildOf(sidebar),
            UNode {
                width: UVal::Percent(1.0),
                padding: USides::all(6.0),
                background_color: Color::srgba(0.03, 0.04, 0.07, 0.9),
                border_radius: UCornerRadius::all(10.0),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
        ))
        .id();
    commands.spawn((
        ChildOf(rtt_wrapper),
        UImage::new(offscreen_image)
            .with_size(UVal::Percent(1.0), UVal::Px(84.0))
            .with_radius(UCornerRadius::all(8.0)),
    ));

    // 5. Controls + Telemetry Footer
    let footer = commands
        .spawn((
            ChildOf(sidebar),
            UNode {
                width: UVal::Percent(1.0),
                padding: USides::all(10.0),
                background_color: Color::srgba(0.03, 0.045, 0.08, 0.9),
                border_radius: UCornerRadius::all(10.0),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 8.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(footer),
        TelemetryRole::Status,
        UTextLabel {
            text: "Telemetry initializing...".into(),
            font_size: 11.5,
            font: body_font.clone(),
            color: Color::srgb(0.84, 0.90, 0.97),
            wrap_mode: UTextWrapMode::WrapToParent,
            linebreak: LineBreak::WordBoundary,
            ..default()
        },
    ));

    let win_btn = commands
        .spawn((
            ChildOf(footer),
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Px(36.0),
                border_radius: UCornerRadius::all(8.0),
                background_color: Color::srgb(0.22, 0.36, 0.72),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                justify_content: UJustifyContent::Center,
                align_items: UAlignItems::Center,
                ..default()
            },
            UFocusable::new(),
            UFocusVisual::border(Color::srgb(0.45, 0.85, 1.0), 2.0),
            UInteraction::default(),
            UInteractionColors {
                normal: Color::srgb(0.22, 0.36, 0.72),
                hovered: Color::srgb(0.30, 0.46, 0.86),
                pressed: Color::srgb(0.16, 0.28, 0.58),
            },
        ))
        .id();
    commands.spawn((
        ChildOf(win_btn),
        TelemetryRole::WindowButton,
        UTextLabel {
            text: "Open 2nd Window (Test Multi-Window Picking & IME)".into(),
            font_size: 12.5,
            font: body_font,
            color: Color::WHITE,
            ..default()
        },
    ));
    commands.entity(win_btn).observe(toggle_secondary_window);

    root
}

#[allow(clippy::too_many_arguments)]
fn spawn_labeled_input_block(
    commands: &mut Commands,
    parent: Entity,
    title: &'static str,
    initial_value: &'static str,
    spec_name: &'static str,
    multiline: bool,
    width: UVal,
    height: f32,
    input_font: Handle<Font>,
) {
    let block = commands
        .spawn((
            ChildOf(parent),
            UNode {
                width: UVal::Percent(1.0),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                gap: 5.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(block),
        UTextLabel {
            text: title.into(),
            font_size: 12.0,
            font: input_font.clone(),
            color: Color::srgb(0.70, 0.82, 0.96),
            ..default()
        },
    ));

    let input = UTextInput::new("Type here...")
        .with_value(initial_value)
        .with_font_size(14.0)
        .with_multiline(multiline);

    spawn_demo_input(commands, block, spec_name, input, input_font, width, height);
}

fn spawn_demo_input(
    commands: &mut Commands,
    parent: Entity,
    name: &'static str,
    input: UTextInput,
    font: Handle<Font>,
    width: UVal,
    height: f32,
) -> Entity {
    let entity = spawn_text_input(commands, input);
    commands.entity(entity).insert((
        ChildOf(parent),
        DemoInputSpec {
            name,
            font,
            width,
            height,
        },
    ));
    entity
}

// ---------------------------------------------------------------------------
// Update Systems & Observers
// ---------------------------------------------------------------------------

fn configure_demo_inputs(
    mut commands: Commands,
    mut query: Query<
        (Entity, &DemoInputSpec, &UTextInputCaret, &mut UNode),
        Without<DemoInputConfigured>,
    >,
    mut label_query: Query<&mut UTextLabel>,
) {
    for (entity, spec, caret, mut node) in query.iter_mut() {
        node.width = spec.width;
        node.height = UVal::Px(spec.height);

        if let Some(text_entity) = caret.text_entity
            && let Ok(mut label) = label_query.get_mut(text_entity)
        {
            label.font = spec.font.clone();
        }

        commands.entity(entity).insert(DemoInputConfigured);
    }
}

fn animate_3d_sculpture_and_offscreen_card(
    time: Res<Time>,
    mut state: ResMut<ShowcaseState>,
    mut sculptures: Query<(&RotatingSculpture, &mut Transform)>,
    mut pulse_dots: Query<&mut UNode, With<OffscreenPulseDot>>,
    mut clock_texts: Query<&mut UTextLabel, With<OffscreenClockText>>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    state.rotation_boost = (state.rotation_boost - dt * 2.5).max(0.0);

    for (sculpt, mut transform) in sculptures.iter_mut() {
        let speed = 0.75 + state.rotation_boost;
        transform.rotate_y(dt * speed);
        transform.rotate_x(dt * 0.35 * speed);
        transform.translation.y = sculpt.base_y + (elapsed * 1.4).sin() * 0.035;
    }

    let wave = 0.5 + 0.5 * (elapsed * 3.0).sin();
    for mut dot_node in pulse_dots.iter_mut() {
        dot_node.background_color = Color::srgb(0.15 + 0.65 * wave, 0.90 - 0.30 * wave, 0.75);
    }
    for mut label in clock_texts.iter_mut() {
        label.text = format!(
            "Live offscreen pass • {:.1}s • boost {:.1}x • Layer 2 isolated",
            elapsed,
            1.0 + state.rotation_boost
        );
    }
}

fn reconcile_secondary_window(
    mut commands: Commands,
    mut sec_state: ResMut<SecondaryWindowState>,
    windows: Query<(), With<Window>>,
) {
    // Reconcile if user closed the OS secondary window manually via its titlebar X.
    if let Some(sec_win) = sec_state.window
        && !windows.contains(sec_win)
    {
        if let Some(root) = sec_state.root.take() {
            commands.entity(root).despawn();
        }
        if let Some(cam) = sec_state.camera.take() {
            commands.entity(cam).despawn();
        }
        sec_state.window = None;
    }
}

fn sync_telemetry_labels(
    sec_state: Res<SecondaryWindowState>,
    mut state: ResMut<ShowcaseState>,
    focus_state: Res<UFocusState>,
    windows: Query<(&Window, Option<&PrimaryWindow>)>,
    resolved_roots: Query<(&ResolvedRootUi, Option<&RenderLayers>)>,
    specs: Query<(&DemoInputSpec, &UTextInput, &UTextInputCaret)>,
    mut labels: Query<(&TelemetryRole, &mut UTextLabel)>,
) {
    let routing_line = state.camera_entities.map(|cams| {
        let fmt_route = |root: Entity, expected_cam: Entity| {
            resolved_roots
                .get(root)
                .map(|(r, layers)| {
                    let matched = r.camera_entity == Some(expected_cam);
                    format!(
                        "{:?} ok={matched} layers={:?}",
                        r.space,
                        layers.cloned().unwrap_or_default()
                    )
                })
                .unwrap_or_else(|_| "pending".into())
        };
        format!(
            "Auto HUD: {} | Auto 3D: {} | Entity RTT: {}",
            fmt_route(cams.root_2d, cams.cam_2d),
            fmt_route(cams.root_3d, cams.cam_3d),
            fmt_route(cams.root_offscreen, cams.cam_offscreen),
        )
    });

    let focused_info = if let Some(focused) = focus_state.focused
        && let Ok((spec, input, caret)) = specs.get(focused)
    {
        state.focused_name = spec.name.into();
        format!(
            "{} | byte={}/{} | scroll=({:.0},{:.0}) | preedit={}",
            spec.name,
            input.cursor_position,
            input.value.len(),
            caret.scroll_offset.x,
            caret.scroll_offset.y,
            caret.ime_preedit.as_ref().map_or("none", |p| p.value.as_str())
        )
    } else {
        state.focused_name = "None".into();
        "None".into()
    };

    let primary_ime = windows
        .iter()
        .find(|(_, is_p)| is_p.is_some())
        .map(|(w, _)| {
            format!(
                "enabled={} pos=({:.0},{:.0})",
                w.ime_enabled, w.ime_position.x, w.ime_position.y
            )
        })
        .unwrap_or_else(|| "n/a".into());

    let telemetry = format!(
        "Focus: {focused_info}\nPrimary Window IME: {primary_ime}\nSubmits: {} | {}",
        state.submit_count, state.last_event
    );
    let btn_label = if sec_state.window.is_some() {
        "Close 2nd Window (Layer 3 + Isolated IME)"
    } else {
        "Open 2nd Window (Test Multi-Window Picking & IME)"
    };
    let clicks_str = format!("3D Clicks: {}", state.world_clicks);

    for (role, mut label) in labels.iter_mut() {
        let next = match role {
            TelemetryRole::CameraRouting => match routing_line.as_deref() {
                Some(line) => line,
                None => continue,
            },
            TelemetryRole::Status => telemetry.as_str(),
            TelemetryRole::WindowButton => btn_label,
            TelemetryRole::WorldCounter => clicks_str.as_str(),
        };
        if label.text != next {
            label.text = next.into();
        }
    }
}

fn toggle_secondary_window(
    _click: On<PointerClick>,
    mut commands: Commands,
    mut sec_state: ResMut<SecondaryWindowState>,
    theme: Res<Theme>,
) {
    if let Some(root) = sec_state.root.take() {
        commands.entity(root).despawn();
    }
    if let Some(cam) = sec_state.camera.take() {
        commands.entity(cam).despawn();
    }
    if let Some(win) = sec_state.window.take() {
        commands.entity(win).despawn();
        return;
    }

    let font = theme.text.font.inter_regular.clone();
    let sec_win = commands
        .spawn(Window {
            title: "Luma Secondary Window — RenderLayers(3) & Isolated IME/Picking".into(),
            resolution: (520, 340).into(),
            ..default()
        })
        .id();

    let sec_cam = commands
        .spawn((
            Camera2d,
            RenderLayers::layer(3),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.05, 0.06, 0.11)),
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(sec_win)),
        ))
        .id();

    let sec_root = commands
        .spawn((
            URootUi {
                camera: UiCameraRef::Entity(sec_cam),
                ..URootUi::screen()
            },
            UNode {
                width: UVal::Percent(1.0),
                height: UVal::Percent(1.0),
                padding: USides::all(24.0),
                background_color: Color::srgb(0.055, 0.075, 0.125),
                ..default()
            },
            ULayout {
                display: UDisplay::Flex,
                flex_direction: UFlexDirection::Column,
                justify_content: UJustifyContent::Center,
                gap: 14.0,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        ChildOf(sec_root),
        UTextLabel {
            text: "Secondary Window (Camera RenderTarget::Window • Inherited Layer 3)".into(),
            font_size: 14.0,
            font: font.clone(),
            color: Color::srgb(0.55, 0.90, 1.0),
            ..default()
        },
    ));
    commands.spawn((
        ChildOf(sec_root),
        UTextLabel {
            text: "Pointer picking & Window.ime_enabled are completely isolated from Window 1:"
                .into(),
            font_size: 12.0,
            font: font.clone(),
            color: Color::srgb(0.78, 0.84, 0.94),
            wrap_mode: UTextWrapMode::WrapToParent,
            linebreak: LineBreak::WordBoundary,
            ..default()
        },
    ));

    spawn_demo_input(
        &mut commands,
        sec_root,
        "2nd Window Multiline Input",
        UTextInput::new("Type in Window 2...")
            .with_value("Independent Window 2 editor.\nIME candidate coordinates stay bound to this window.")
            .with_font_size(14.0)
            .with_multiline(true),
        font,
        UVal::Percent(1.0),
        120.0,
    );

    sec_state.window = Some(sec_win);
    sec_state.camera = Some(sec_cam);
    sec_state.root = Some(sec_root);
}

fn on_demo_text_changed(
    trigger: On<UTextInputChanged>,
    specs: Query<&DemoInputSpec>,
    mut state: ResMut<ShowcaseState>,
) {
    let field = specs
        .get(trigger.entity)
        .map_or("Input", |s| s.name);
    let preview: String = trigger.event().value.chars().take(42).collect();
    state.last_event = format!("Changed [{field}]: \"{preview}\"");
}

fn on_demo_text_submit(
    trigger: On<UTextInputSubmit>,
    specs: Query<&DemoInputSpec>,
    mut state: ResMut<ShowcaseState>,
) {
    let field = specs
        .get(trigger.entity)
        .map_or("Input", |s| s.name);
    state.submit_count += 1;
    let preview: String = trigger.event().value.chars().take(42).collect();
    state.last_event = format!("Submitted [{field}]: \"{preview}\"");
}
