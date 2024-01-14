use std::sync::Arc;

use anyhow::Context;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use triton::{FrameSystem, GeometrySystem, Pass};
use vulkano::{
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
    memory,
    sync::{self, GpuFuture},
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

pub fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();

    let context = VulkanoContext::new(VulkanoConfig::default());

    let mut windows = VulkanoWindows::default();

    windows.create_window(&event_loop, &context, &WindowDescriptor::default(), |ci| {
        ci.image_format = vulkano::format::Format::B8G8R8A8_UNORM;
        ci.min_image_count = ci.min_image_count.max(2);
    });

    let queue = windows
        .get_primary_renderer()
        .context("get primary renderer")?
        .graphics_queue();

    let image_format = windows
        .get_primary_renderer()
        .context("get primary renderer")?
        .swapchain_format();

    let memory_allocator = context.memory_allocator();

    let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
        context.device().clone(),
        StandardCommandBufferAllocatorCreateInfo {
            secondary_buffer_count: 32,
            ..Default::default()
        },
    ));

    let mut frame_system = FrameSystem::new(
        queue.clone(),
        image_format,
        memory_allocator.clone(),
        command_buffer_allocator.clone(),
    )
    .context("creating FrameSystem")?;

    let mut geometry_system = GeometrySystem::new(
        queue.clone(),
        frame_system.deferred_subpass(),
        memory_allocator.clone(),
        command_buffer_allocator.clone(),
    )?;

    event_loop.run(move |event, _, control_flow| {
        let renderer = windows.get_primary_renderer_mut().unwrap();
        match event {
            Event::WindowEvent { event, window_id } if window_id == renderer.window().id() => {
                match event {
                    WindowEvent::Resized(_) => {
                        renderer.resize();
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize();
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => (),
                }
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } if window_id == renderer.window().id() => {
                *control_flow = ControlFlow::Exit;
            }

            Event::RedrawRequested(window_id) if window_id == renderer.window().id() => {
                let image_extent: [u32; 2] = renderer.window().inner_size().into();
                if image_extent.contains(&0) {
                    return;
                }

                let acquire_future = match renderer.acquire() {
                    Ok(future) => future,
                    Err(vulkano::VulkanError::OutOfDate) => {
                        renderer.resize();
                        sync::now(context.device().clone()).boxed()
                    }
                    Err(e) => panic!("Failed to acquire swapchain future: {}", e),
                };

                let mut frame = frame_system
                    .frame(
                        acquire_future,
                        renderer.swapchain_image_view().clone(),
                        Matrix4::identity(),
                    )
                    .context("getting a frame")
                    .unwrap(); // TODO figure out how winit decides to handle Results today

                let mut after_future: Option<Box<dyn GpuFuture>> = None;

                // TODO figure out how winit decides to handle Results today
                while let Some(pass) = frame.next_pass().unwrap() {
                    match pass {
                        Pass::Deferred(mut draw_pass) => {
                            let cb = geometry_system
                                .draw(draw_pass.viewport_dimensions())
                                .context("drawing geometry")
                                .unwrap();
                            // TODO figure out how winit decides to handle Results today
                            draw_pass.execute(cb).context("executing draw pass");
                        }
                        Pass::Lighting(mut lighting) => {
                            lighting.ambient_light([0.1, 0.1, 0.1]);
                            lighting
                                .directional_light(Vector3::new(0.2, -0.1, -0.7), [0.6, 0.6, 0.6]);
                            lighting.point_light(Vector3::new(0.5, -0.5, -0.1), [1.0, 0.0, 0.0]);
                            lighting.point_light(Vector3::new(-0.9, 0.2, -0.15), [0.0, 1.0, 0.0]);
                            lighting.point_light(Vector3::new(0.0, 0.5, -0.05), [0.0, 0.0, 1.0]);
                        }
                        Pass::Finished(af) => {
                            after_future = Some(af);
                        }
                    }
                }
                renderer.present(after_future.unwrap(), true);
            }

            Event::MainEventsCleared => {
                renderer.window().request_redraw();
            }

            _ => (),
        }
    })
}
