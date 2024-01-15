use std::sync::Arc;

use anyhow::{anyhow, Context};
use cgmath::{Matrix4, SquareMatrix, Vector3};
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
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowId};

use crate::{game::Transform, FrameSystem, GeometrySystem, LightingPass, Pass};

pub struct Renderer {
    context: VulkanoContext,
    windows: VulkanoWindows,
    frame_system: FrameSystem,
    geometry_system: GeometrySystem,
}

#[cfg(feature = "tracing")]
use tracing_tracy::client::frame_mark;

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let context = VulkanoContext::new(VulkanoConfig::default());

        let mut windows = VulkanoWindows::default();

        windows.create_window(event_loop, &context, &WindowDescriptor::default(), |ci| {
            ci.image_format = vulkano::format::Format::B8G8R8A8_UNORM;
            ci.min_image_count = ci.min_image_count.max(2);
        });

        let queue = windows
            .get_primary_renderer()
            .context("geting primary renderer")?
            .graphics_queue();

        let image_format = windows
            .get_primary_renderer()
            .context("geting primary renderer")?
            .swapchain_format();

        let memory_allocator = context.memory_allocator();

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            context.device().clone(),
            StandardCommandBufferAllocatorCreateInfo {
                secondary_buffer_count: 32,
                ..Default::default()
            },
        ));

        let frame_system = FrameSystem::new(
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
        )
        .context("creating Geometry System")?;

        Ok(Renderer {
            context,
            windows,
            frame_system,
            geometry_system,
        })
    }

    pub fn enqueue_mesh(&mut self, mesh_id: usize, transform: Transform) {}

    pub fn set_camera_params(&self, matrices: (Matrix4<f32>, Matrix4<f32>)) {}

    pub fn resize(&mut self) -> anyhow::Result<()> {
        if let Some(renderer) = self.windows.get_primary_renderer_mut() {
            renderer.resize();
            Ok(())
        } else {
            Err(anyhow!("No Primary Renderer available"))
        }
    }

    pub fn request_redraw(&self) -> anyhow::Result<()> {
        if let Some(window) = self.windows.get_primary_window() {
            window.request_redraw();
            Ok(())
        } else {
            Err(anyhow!("No Primary Window available"))
        }
    }

    pub fn window_size(&self) -> Option<PhysicalSize<u32>> {
        self.windows.get_primary_window().map(|w| w.inner_size())
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.windows.primary_window_id()
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        let renderer = self
            .windows
            .get_primary_renderer_mut()
            .context("getting primary renderer")?;

        let acquire_future = match renderer.acquire() {
            Ok(future) => future,
            Err(vulkano::VulkanError::OutOfDate) => {
                renderer.resize();
                sync::now(self.context.device().clone()).boxed()
            }
            Err(e) => return Err(anyhow!("Unexpected error acquiring swapchain image: {}", e)),
        };

        let mut frame = self.frame_system.frame(
            acquire_future,
            renderer.swapchain_image_view().clone(),
            Matrix4::identity(),
        )?;

        let mut after_future: Option<Box<dyn GpuFuture>> = None;

        while let Some(pass) = frame.next_pass()? {
            match pass {
                Pass::Deferred(mut draw_pass) => {
                    let cb = self
                        .geometry_system
                        .draw(draw_pass.viewport_dimensions())
                        .context("drawing geometry")?;
                    draw_pass.execute(cb)?;
                }
                Pass::Lighting(lighting) => {
                    Self::render_lighting(lighting)?;
                }
                Pass::Finished(af) => {
                    after_future = Some(af);
                }
            }
        }
        renderer.present(
            after_future.context("getting renderpass finish future")?,
            true,
        );

        Ok(())
    }

    fn render_lighting(mut lighting: LightingPass<'_, '_>) -> anyhow::Result<()> {
        lighting.ambient_light([0.1, 0.1, 0.1])?;
        lighting.directional_light(Vector3::new(0.2, -0.1, -0.7), [0.6, 0.6, 0.6])?;
        lighting.point_light(Vector3::new(0.5, -0.5, -0.1), [1.0, 0.0, 0.0])?;
        lighting.point_light(Vector3::new(-0.9, 0.2, -0.15), [0.0, 1.0, 0.0])?;
        lighting.point_light(Vector3::new(0.0, 0.5, -0.05), [0.0, 0.0, 1.0])?;
        Ok(())
    }
}
