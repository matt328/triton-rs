use super::graphics::GraphicsContext;
use anyhow::Context;
use log::info;
use std::{sync::Arc, time::Instant};
use vulkano::swapchain::Surface;
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::VulkanoWindows,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub struct Game {
    graphics_context: GraphicsContext,
}

impl Game {
    pub fn new(context: &VulkanoContext, surface: Arc<Surface>) -> anyhow::Result<Game> {
        Ok(Game {
            graphics_context: GraphicsContext::new(context, surface.clone())
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
        let s = renderer.unwrap().surface();

        let game = Arc::new(Game::new(&context, s.clone()).context("Failed to create Game")?);

        let mut previous_instant = Instant::now();
        let max_frame_time: f64 = 0.0;
        let mut accumulated_time: f64 = 0.0;
        let fixed_time_step: f64 = 1.0 / 240.0;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if !game.window_handler(&event) {
                *control_flow = ControlFlow::Exit;
            }

            match event {
                Event::RedrawRequested(_) => {
                    let current_instant = Instant::now();
                    let mut elapsed = current_instant
                        .duration_since(previous_instant)
                        .as_secs_f64();
                    if elapsed > max_frame_time {
                        elapsed = max_frame_time;
                    }
                    accumulated_time += elapsed;

                    while accumulated_time >= fixed_time_step {
                        game.update();
                        accumulated_time -= fixed_time_step;
                    }

                    let blending_factor = accumulated_time / fixed_time_step;
                    game.render(blending_factor);

                    previous_instant = current_instant;
                }
                Event::MainEventsCleared => {
                    vulkano_windows
                        .get_primary_window()
                        .unwrap()
                        .request_redraw();
                }
                _ => {}
            }
        })
    }
}
