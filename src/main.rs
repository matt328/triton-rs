use std::time::Instant;

use anyhow::Context;

use log::info;
use tracing::{event, span, Level};
use tracing_tracy::client::frame_mark;
use tracy_client::ProfiledAllocator;
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
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    #[cfg(feature = "tracing")]
    info!("Tracing enabled");

    #[cfg(feature = "tracing")]
    #[global_allocator]
    static GLOBAL: ProfiledAllocator<std::alloc::System> =
        ProfiledAllocator::new(std::alloc::System, 100);

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    let _root = span!(Level::INFO, "root").entered();

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
    let max_frame_time: f64 = 0.166666;
    let mut accumulated_time: f64 = 0.0;
    let fixed_time_step: f64 = 1.0 / 240.0;
    let mut current_instant = Instant::now();

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
                let _redraw = span!(Level::INFO, "redraw_requested", accumulated_time).entered();
                current_instant = Instant::now();

                let mut elapsed = current_instant
                    .duration_since(previous_instant)
                    .as_secs_f64();

                event!(Level::INFO, elapsed);

                if elapsed > max_frame_time {
                    elapsed = max_frame_time;
                }
                accumulated_time += elapsed;

                // Keep updating as much as we can between render ticks
                while accumulated_time >= fixed_time_step {
                    let _update_span = span!(Level::INFO, "update_loop",).entered();
                    event!(Level::INFO, accumulated_time);
                    prev_object_rotation = object_rotation;
                    object_rotation = update_game(object_rotation);
                    accumulated_time -= fixed_time_step;
                }

                let blending_factor = accumulated_time / fixed_time_step;

                event!(Level::INFO, blending_factor);
                let current_state =
                    blend_state(prev_object_rotation, object_rotation, blending_factor);

                let _rendered = vulkan_app.render_game(current_state, &vertex_buffer);

                #[cfg(feature = "tracing")]
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
    let _update_game_span = span!(Level::INFO, "update_game").entered();
    let new_state = state + 1.0;
    new_state
}

fn blend_state(previous_state: f64, next_state: f64, blending_factor: f64) -> f64 {
    let _blend_state_span = span!(Level::INFO, "blend_state").entered();
    previous_state + (blending_factor * (next_state - previous_state))
}
