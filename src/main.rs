use std::time::Instant;

use anyhow::Context;

use tracing::{event, span, Level};
use tracing_tracy::client::frame_mark;
use triton::{app::App, shaders::Position};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use tracing_subscriber::layer::SubscriberExt;

pub const WINDOW_WIDTH: f32 = 1024.0;
pub const WINDOW_HEIGHT: f32 = 1024.0;

fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    let _span = span!(Level::TRACE, "root").entered();

    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    let event_loop = EventLoop::new();

    let mut vulkan_app = App::new(&event_loop).context("Creating App")?;

    let vertices = [
        Position {
            position: [-0.5, -0.25],
        },
        Position {
            position: [0.0, 0.5],
        },
        Position {
            position: [0.25, -0.1],
        },
    ];
    let vertex_buffer = Buffer::from_iter(
        vulkan_app.context.memory_allocator(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        vertices,
    )?;

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
                let _span = span!(Level::TRACE, "redraw_requested").entered();
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
                    let _span = span!(
                        Level::TRACE,
                        "update_loop",
                        accumulated_time,
                        fixed_time_step
                    )
                    .entered();
                    prev_object_rotation = object_rotation;
                    event!(Level::TRACE, accumulated_time);
                    object_rotation = update_game(object_rotation);
                    accumulated_time -= fixed_time_step;
                }

                let blending_factor = accumulated_time / fixed_time_step;

                let _rendered = vulkan_app.render_game(
                    prev_object_rotation,
                    object_rotation,
                    blending_factor,
                    &vertex_buffer,
                );

                frame_mark();
                previous_instant = current_instant;
            }
            Event::MainEventsCleared => {
                vulkan_app
                    .windows
                    .get_primary_window()
                    .map(|w| w.request_redraw());
            }
            _ => {}
        }
    })
}

/// Mutates the game state, and produces the next state
fn update_game(state: f64) -> f64 {
    let _span = span!(Level::TRACE, "update_game").entered();
    let new_state = state + 1.0;
    new_state
}
