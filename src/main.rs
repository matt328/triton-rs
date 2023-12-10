use std::{sync::Arc, time::Instant};

use anyhow::Context;
use log::error;
use triton::{
    app::App,
    shaders::{create_pipeline, Position},
};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    device::Queue,
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    sync::GpuFuture,
};
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
    )
    .unwrap();

    let pipeline = create_pipeline(
        vulkan_app.context.device(),
        &vulkan_app.windows.get_primary_renderer().unwrap(),
    );

    let viewport = Viewport {
        offset: [0.0, 0.0],
        extent: [WINDOW_HEIGHT, WINDOW_WIDTH],
        depth_range: 0.0..=1.0,
    };

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

                let f = render_game(
                    blending_factor,
                    future,
                    prev_object_rotation,
                    object_rotation,
                    &vulkan_app.command_buffer_allocator,
                    &renderer.graphics_queue(),
                    renderer.swapchain_image_view().clone(),
                    viewport.clone(),
                    pipeline.clone(),
                    &vertex_buffer,
                );

                renderer.present(f, false);

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
    state + 1.0
}

/// Calculates the 'current' state by applying the blending factor between the two states
fn render_game(
    blend_factor: f64,
    future: Box<dyn GpuFuture>,
    previous_state: f64,
    next_state: f64,
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    queue: &Arc<Queue>,
    image: Arc<ImageView>,
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer: &Subbuffer<[Position]>,
) -> Box<dyn GpuFuture> {
    let _state = previous_state + (blend_factor * (next_state - previous_state));

    let mut builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    builder
        .begin_rendering(RenderingInfo {
            color_attachments: vec![Some(RenderingAttachmentInfo {
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                clear_value: Some([0.0, 0.0, 1.0, 1.0].into()),
                ..RenderingAttachmentInfo::image_view(image)
            })],
            ..Default::default()
        })
        .unwrap()
        .set_viewport(0, [viewport.clone()].into_iter().collect())
        .bind_pipeline_graphics(pipeline.clone())
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .draw(vertex_buffer.len() as u32, 1, 0, 0)
        .unwrap()
        .end_rendering()
        .unwrap();

    let command_buffer = builder.build().unwrap();

    Box::new(future.then_execute(queue.clone(), command_buffer).unwrap())
}
