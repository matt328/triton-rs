use std::sync::Arc;

use anyhow::Context;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use triton::{FrameSystem, GeometrySystem, LightingPass, Pass, Renderer};
use vulkano::{
    command_buffer::allocator::{
        StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
    },
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

    let triton_renderer = Renderer::new(&event_loop)?;

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

    let geometry_system = GeometrySystem::new(
        queue.clone(),
        frame_system.deferred_subpass(),
        memory_allocator.clone(),
        command_buffer_allocator.clone(),
    )?;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, window_id } if window_id == triton_renderer.window_id() => {
            match event {
                WindowEvent::Resized(_) => {
                    triton_renderer.resize();
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    triton_renderer.resize();
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
        } if window_id == triton_renderer.window_id() => {
            *control_flow = ControlFlow::Exit;
        }

        Event::RedrawRequested(window_id) if window_id == triton_renderer.window_id() => {
            let image_extent: [u32; 2] = triton_renderer.window_size().into();
            if image_extent.contains(&0) {
                return;
            }

            triton_renderer.render();
        }

        Event::MainEventsCleared => {
            triton_renderer.request_redraw();
        }

        _ => (),
    })
}

fn render_lighting(mut lighting: LightingPass<'_, '_>) -> anyhow::Result<()> {
    lighting.ambient_light([0.1, 0.1, 0.1])?;
    lighting.directional_light(Vector3::new(0.2, -0.1, -0.7), [0.6, 0.6, 0.6])?;
    lighting.point_light(Vector3::new(0.5, -0.5, -0.1), [1.0, 0.0, 0.0])?;
    lighting.point_light(Vector3::new(-0.9, 0.2, -0.15), [0.0, 1.0, 0.0])?;
    lighting.point_light(Vector3::new(0.0, 0.5, -0.05), [0.0, 0.0, 1.0])?;
    Ok(())
}
