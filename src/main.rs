use std::time::Instant;

use anyhow::Context;
use log::{error, info};
use triton::app::App;
use vulkano::sync::GpuFuture;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 1024.0;

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    let event_loop = EventLoop::new();

    let mut vulkan_app = App::default();

    vulkan_app.open(&event_loop);

    // fixed timestep items
    let mut previous_instant = Instant::now();
    let max_frame_time: f64 = 0.1;
    let mut accumulated_time: f64 = 0.0;
    let fixed_time_step: f64 = 1.0 / 240.0;

    let mut object_rotation = 0.0;
    let mut prev_object_rotation = 0.0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::RedrawRequested(_) => {
                let current_instant = Instant::now();
                let mut elapsed = current_instant
                    .duration_since(previous_instant)
                    .as_secs_f64();
                if elapsed > max_frame_time {
                    elapsed = max_frame_time;
                }
                accumulated_time += elapsed;

                // Keep updating as much as we can between render ticks
                while accumulated_time >= fixed_time_step {
                    prev_object_rotation = object_rotation;
                    object_rotation = update_game(object_rotation);
                    accumulated_time -= fixed_time_step;
                }

                let blending_factor = accumulated_time / fixed_time_step;
                let renderer = vulkan_app.windows.get_primary_renderer_mut().unwrap();

                let future = match renderer.acquire() {
                    Err(e) => {
                        error!("{e}");
                        return;
                    }
                    Ok(future) => future,
                };

                render_game(
                    blending_factor,
                    &future,
                    prev_object_rotation,
                    object_rotation,
                );

                renderer.present(future, false);

                previous_instant = current_instant;
            }
            Event::MainEventsCleared => {
                vulkan_app
                    .windows
                    .get_primary_window()
                    .unwrap()
                    .request_redraw();
            }
            _ => {}
        }
    })
}

/// Mutates the game state, and produces the next state
fn update_game(state: f64) -> f64 {
    info!("Update");
    state + 1.0
}

/// Calculates the 'current' state by applying the blending factor between the two states
fn render_game(
    blend_factor: f64,
    future: &Box<dyn GpuFuture>,
    previous_state: f64,
    next_state: f64,
) -> () {
    info!(
        "blend_factor: {} next_state: {} previous_state: {}",
        blend_factor, next_state, previous_state
    );
    let state = previous_state + (blend_factor * (next_state - previous_state));
    info!("render: {}", state);
}
