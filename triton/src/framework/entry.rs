use anyhow::Context;
use log::info;
use std::{error::Error, sync::Arc, time::Instant, thread::current};
use vulkano::{
    instance::{Instance, InstanceCreateInfo},
    swapchain::Surface,
    VulkanLibrary,
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    renderer::VulkanoWindowRenderer,
    window::VulkanoWindows,
};
use vulkano_win::create_surface_from_winit;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{self, EventLoop, ControlFlow},
    window::WindowBuilder,
};
use std::ops::Sub;
use super::graphics::GraphicsContext;

pub struct Game {
    graphics_context: GraphicsContext,
}

impl Game {
    pub fn new(instance: Arc<Instance>, surface: Arc<Surface>) -> anyhow::Result<Game> {
        Ok(Game {
            graphics_context: GraphicsContext::new(instance.clone(), surface.clone())
                .context("Game creating graphics context")?,
        })
    }

    pub fn update(&self) {
        // info!("update");
    }

    pub fn render(&self, blending_factor: f64) {
        // info!("Render: {}", blending_factor);
    }

    pub fn window_handler(&self, event: &Event<()>) -> bool {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                return false;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                // self.recreate_swapchain = true;
                return true;
            }
            _ => true,
        }
    }
}

pub struct Application {}

impl Application {
    pub fn new() -> anyhow::Result<Application> {
        Ok(Application {})
    }

    pub fn run(&self) -> anyhow::Result<()> {
        info!("Running Application");

        let context = VulkanoContext::new(VulkanoConfig::default());

        info!(
            "Using device: {} (type: {:?})",
            context.device_name(),
            context.device_type()
        );

        let event_loop = EventLoop::new();
        let mut vulkano_windows = VulkanoWindows::default();
        let _id1 =
            vulkano_windows.create_window(&event_loop, &context, &Default::default(), |_| {});
        let renderer = vulkano_windows.get_primary_renderer();

        let game =
            Arc::new(Game::new(context.instance().clone(), renderer.unwrap().surface.clone()).context("Failed to create Game")?);

        let current_instant = Instant::now();
        let previous_instant = Instant::now();
        let max_frame_time: f64 = 0.0;
        let last_frame_time: f64 = 0.0;
        let running_time: f64 = 0.0;
        let accumulated_time: f64 = 0.0;
        let fixed_time_step: f64 = 1.0 / 240.0;
        let number_of_updates = 0;
        let number_of_renders = 0;
        let blending_factor = 0.0;
        let exit_next_iteration = false;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if !game.window_handler(&event) {
                *control_flow = ControlFlow::Exit;
            }

            match event {
                Event::RedrawRequested(_) => {
                    current_instant = Instant::now();
                    let mut elapsed = current_instant.sub(previous_instant);
                    if elapsed > max_frame_time {
                        elapsed = max_frame_time;
                    }
                    last_frame_time = elapsed;
                    running_time += elapsed;
                    accumulated_time += elapsed;

                    while accumulated_time >= fixed_time_step {
                        game.update();
                        accumulated_time -= fixed_time_step;
                        number_of_updates += 1;
                    }

                    blending_factor = accumulated_time / fixed_time_step;
                    game.render(blending_factor);

                    number_of_renders += 1;
                    previous_instant = current_instant;
                    return true;
                }
                Event::MainEventsCleared => {
                    vulkano_windows.get_primary_window().unwrap().request_redraw();
                }
                _ => {}
            }
        })
    }
}
