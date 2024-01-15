use anyhow::Context;
use cgmath::{Matrix4, SquareMatrix};
use vulkano::sync::{self, GpuFuture};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::{dpi::PhysicalSize, event_loop::EventLoop, window::WindowId};

use crate::Pass;

pub struct Renderer {
    context: VulkanoContext,
    windows: VulkanoWindows,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
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

        Ok(Renderer { context, windows })
    }

    pub fn resize(&self) {}

    pub fn request_redraw(&self) {}

    pub fn window_size(&self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: 0,
            height: 0,
        }
    }

    pub fn window_id(&self) -> WindowId {
        let f: u64 = 0;
        WindowId::from(f)
    }

    pub fn render(&self) -> anyhow::Result<()> {
        let mut renderer = self
            .windows
            .get_primary_renderer()
            .context("getting primary renderer")?;
        let acquire_future = match renderer.acquire() {
            Ok(future) => future,
            Err(vulkano::VulkanError::OutOfDate) => {
                renderer.resize();
                sync::now(self.context.device().clone()).boxed()
            }
            Err(e) => panic!("Failed to acquire swapchain future: {}", e),
        };

        if let Ok(mut frame) = frame_system.frame(
            acquire_future,
            renderer.swapchain_image_view().clone(),
            Matrix4::identity(),
        ) {
            let mut after_future: Option<Box<dyn GpuFuture>> = None;

            // TODO figure out how winit decides to handle Results today
            while let Some(pass) = frame.next_pass().unwrap() {
                match pass {
                    Pass::Deferred(mut draw_pass) => {
                        let cb = geometry_system
                            .draw(draw_pass.viewport_dimensions())
                            .context("drawing geometry")
                            .unwrap();

                        if let Err(e) = draw_pass.execute(cb) {
                            log::error!("{}", e);
                        }
                    }
                    Pass::Lighting(lighting) => {
                        if let Err(e) = render_lighting(lighting) {
                            log::error!("{}", e);
                        }
                    }
                    Pass::Finished(af) => {
                        after_future = Some(af);
                    }
                }
            }
            renderer.present(after_future.unwrap(), true);
        }
        Ok(())
    }
}
