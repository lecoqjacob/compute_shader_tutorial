mod ca_simulator;
mod camera;
mod gui;
mod quad_pipeline;
#[allow(clippy::too_many_arguments)]
mod render;
mod utils;
mod vertex;

use bevy::{
    app::PluginGroupBuilder,
    input::mouse::MouseWheel,
    prelude::*,
    window::{close_on_esc, PrimaryWindow, WindowMode},
};
use bevy_vulkano::{
    BevyVulkanoContext, BevyVulkanoSettings, BevyVulkanoWindows, VulkanoWinitPlugin,
};

use crate::{
    ca_simulator::CASimulator,
    camera::OrthographicCamera,
    gui::user_interface,
    render::FillScreenRenderPass,
    utils::{cursor_to_world, get_canvas_line, MousePos},
};

pub const WIDTH: f32 = 1920.0;
pub const HEIGHT: f32 = 1080.0;
pub const CANVAS_SIZE_X: u32 = 512;
pub const CANVAS_SIZE_Y: u32 = 512;
pub const LOCAL_SIZE_X: u32 = 32;
pub const LOCAL_SIZE_Y: u32 = 32;
pub const NUM_WORK_GROUPS_X: u32 = CANVAS_SIZE_X / LOCAL_SIZE_X;
pub const NUM_WORK_GROUPS_Y: u32 = CANVAS_SIZE_Y / LOCAL_SIZE_Y;
pub const CLEAR_COLOR: [f32; 4] = [1.0; 4];
pub const CAMERA_MOVE_SPEED: f32 = 200.0;

#[derive(Resource)]
pub struct DynamicSettings {
    pub brush_radius: f32,
    pub draw_matter: u32,
}

impl Default for DynamicSettings {
    fn default() -> Self {
        Self {
            brush_radius: 4.0,
            draw_matter: 0xff0000ff,
        }
    }
}

pub struct PluginBundle;
impl PluginGroup for PluginBundle {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<PluginBundle>()
            // Minimum plugins for the demo
            .add(bevy::log::LogPlugin::default())
            .add(bevy::core::TaskPoolPlugin::default())
            .add(bevy::core::TypeRegistrationPlugin::default())
            .add(bevy::core::FrameCountPlugin::default())
            .add(bevy::time::TimePlugin::default())
            .add(bevy::diagnostic::DiagnosticsPlugin::default())
            .add(bevy::input::InputPlugin::default())
            .add(bevy::window::WindowPlugin::default())
            // Don't add WinitPlugin. This owns "core loop" (runner).
            // Bevy winit and render should be excluded
            .add(VulkanoWinitPlugin::default())
    }
}

fn main() {
    App::new()
        .insert_non_send_resource(BevyVulkanoSettings {
            // Since we're only drawing gui, let's clear each frame
            is_gui_overlay: true,
            ..BevyVulkanoSettings::default()
        })
        .add_plugins(PluginBundle.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1024.0, 1024.0).into(),
                title: "Bevy Vulkano Game Of Life".to_string(),
                present_mode: bevy::window::PresentMode::Immediate,
                resizable: true,
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(input_actions)
        .add_system(update_camera)
        .add_system(update_mouse)
        .add_system(draw_matter)
        .add_system(simulate)
        // Gui
        .add_system(user_interface)
        // Render after update
        .add_system(render.in_base_set(CoreSet::PostUpdate))
        .run();
}

/// Creates our simulation & render pipelines
fn setup(
    mut commands: Commands,
    window_query: Query<Entity, With<Window>>,
    context: Res<BevyVulkanoContext>,
    windows: NonSend<BevyVulkanoWindows>,
) {
    let window_entity = window_query.single();
    let primary_window = windows.get_vulkano_window(window_entity).unwrap();

    // Create our render pass
    let fill_screen = FillScreenRenderPass::new(
        context.context.memory_allocator().clone(),
        primary_window.renderer.graphics_queue(),
        primary_window.renderer.swapchain_format(),
    );
    let simulator = CASimulator::new(
        context.context.memory_allocator(),
        primary_window.renderer.compute_queue(),
    );

    // Create simple orthographic camera
    let mut camera = OrthographicCamera::default();
    // Zoom camera to fit vertical pixels
    camera.zoom_to_fit_vertical_pixels(CANVAS_SIZE_Y, HEIGHT as u32);
    // Insert resources
    commands.insert_resource(fill_screen);
    commands.insert_resource(camera);
    commands.insert_resource(simulator);
    commands.insert_resource(PreviousMousePos(None));
    commands.insert_resource(CurrentMousePos(None));
    commands.insert_resource(DynamicSettings::default());
}

/// Step simulation
fn simulate(mut sim_pipeline: ResMut<CASimulator>) {
    sim_pipeline.step();
}

/// Render the simulation
fn render(
    simulator: Res<CASimulator>,
    camera: Res<OrthographicCamera>,
    mut fill_screen: ResMut<FillScreenRenderPass>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
) {
    let window_entity = window_query.single();
    let primary_window = vulkano_windows
        .get_vulkano_window_mut(window_entity)
        .unwrap();

    // Start frame
    let before = match primary_window.renderer.acquire() {
        Err(e) => {
            bevy::log::error!("Failed to start frame: {}", e);
            return;
        }
        Ok(f) => f,
    };

    let canvas_image = simulator.color_image();

    // Render
    let final_image = primary_window.renderer.swapchain_image_view();
    let after_images = fill_screen.draw(
        before,
        *camera,
        canvas_image,
        final_image.clone(),
        CLEAR_COLOR,
        false,
        false,
    );
    // Draw gui
    let after_gui = primary_window.gui.draw_on_image(after_images, final_image);
    // Finish Frame
    primary_window.renderer.present(after_gui, true);
}

/// Update camera (if window is resized)
fn update_camera(window_query: Query<&Window>, mut camera: ResMut<OrthographicCamera>) {
    let window = window_query.single();
    camera.update(window.width(), window.height());
}

/// Input actions for camera movement, zoom and pausing
fn input_actions(
    time: Res<Time>,
    mut camera: ResMut<OrthographicCamera>,
    keyboard_input: Res<Input<KeyCode>>,
    mut mouse_input_events: EventReader<MouseWheel>,
) {
    // Move camera with arrows & WASD
    let up = keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up);
    let down = keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down);
    let left = keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left);
    let right = keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right);

    let x_axis = -(right as i8) + left as i8;
    let y_axis = -(up as i8) + down as i8;

    let mut move_delta = Vec2::new(x_axis as f32, y_axis as f32);
    if move_delta != Vec2::ZERO {
        move_delta /= move_delta.length();
        camera.pos += move_delta * time.delta_seconds() * CAMERA_MOVE_SPEED;
    }

    // Zoom camera with mouse scroll
    for e in mouse_input_events.iter() {
        if e.y < 0.0 {
            camera.scale *= 1.05;
        } else {
            camera.scale *= 1.0 / 1.05;
        }
    }
}

/// Draw matter to our grid
fn draw_matter(
    mut simulator: ResMut<CASimulator>,
    prev: Res<PreviousMousePos>,
    current: Res<CurrentMousePos>,
    mouse_button_input: Res<Input<MouseButton>>,
    settings: Res<DynamicSettings>,
) {
    if let Some(current) = current.0 {
        if mouse_button_input.pressed(MouseButton::Left) {
            let line = get_canvas_line(prev.0, current);
            simulator.draw_matter(&line, settings.brush_radius, settings.draw_matter);
        }
    }
}

/// Mouse position from last frame
#[derive(Debug, Copy, Clone, Resource)]
pub struct PreviousMousePos(pub Option<MousePos>);

/// Mouse position now
#[derive(Debug, Copy, Clone, Resource)]
pub struct CurrentMousePos(pub Option<MousePos>);

/// Update mouse position
fn update_mouse(
    window_query: Query<&Window>,
    mut _prev: ResMut<PreviousMousePos>,
    mut _current: ResMut<CurrentMousePos>,
    camera: Res<OrthographicCamera>,
) {
    _prev.0 = _current.0;
    let primary = window_query.single();
    if primary.cursor_position().is_some() {
        _current.0 = Some(MousePos {
            world: cursor_to_world(primary, camera.pos, camera.scale),
        });
    }
}
