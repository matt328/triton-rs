use std::sync::Arc;

use anyhow::Context;
use log::info;
use tracing::{span, Level};
use vulkano::sync::GpuFuture;
use vulkano::{
    buffer::Subbuffer,
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{DeviceExtensions, Features},
    instance::{InstanceCreateInfo, InstanceExtensions},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    swapchain::PresentMode,
    Version,
};
use vulkano_util::{
    context::{VulkanoConfig, VulkanoContext},
    window::{VulkanoWindows, WindowDescriptor},
};
use winit::event_loop::EventLoop;

pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 720.0;

pub struct App {
    pub context: VulkanoContext,
    pub windows: VulkanoWindows,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,
}

#[cfg(target_os = "macos")]
use vulkano::instance::InstanceCreateFlags;

use crate::shaders::{create_pipeline, Position};

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> anyhow::Result<Self> {
        let config = VulkanoConfig {
            instance_create_info: InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                max_api_version: Some(Version::V1_2),
                enabled_extensions: InstanceExtensions {
                    #[cfg(target_os = "macos")]
                    khr_portability_enumeration: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            device_extensions: DeviceExtensions {
                khr_dynamic_rendering: true,
                #[cfg(target_os = "macos")]
                khr_portability_subset: true,
                khr_swapchain: true,
                ..Default::default()
            },
            device_features: Features {
                dynamic_rendering: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let context = VulkanoContext::new(config);
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            context.device().clone(),
            Default::default(),
        ));

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            context.device().clone(),
        ));

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [WINDOW_HEIGHT, WINDOW_WIDTH],
            depth_range: 0.0..=1.0,
        };

        let mut windows = VulkanoWindows::default();

        info!("Creating Window");
        let _id1 = windows.create_window(
            event_loop,
            &context,
            &WindowDescriptor {
                width: WINDOW_WIDTH,
                height: WINDOW_HEIGHT,
                title: "Triton Application".to_string(),
                ..Default::default()
            },
            |_| {},
        );

        let renderer = windows
            .get_primary_renderer()
            .context("getting primary renderer")?;

        let pipeline = create_pipeline(context.device(), renderer);

        Ok(App {
            context,
            windows,
            command_buffer_allocator,
            descriptor_set_allocator,
            viewport,
            pipeline,
        })
    }

    pub fn render_game(
        &mut self,
        state: f64,
        vertex_buffer: &Subbuffer<[Position]>,
    ) -> anyhow::Result<()> {
        let _span = span!(Level::INFO, "render_game").entered();

        let renderer = self
            .windows
            .get_primary_renderer_mut()
            .context("Could not get primary renderer")?;

        let future = renderer.acquire().context("Acquiring future")?;

        let queue = renderer.graphics_queue();

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .context("Creating Command Buffer Builder")?;

        let image = renderer.swapchain_image_view().clone();

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
            .context("begin rendering")?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .context("draw")?
            .end_rendering()
            .context("end rendering")?;

        let cmd_buffer_span = span!(Level::TRACE, "build_command_buffer").entered();
        let command_buffer = builder.build().unwrap();
        cmd_buffer_span.exit();

        let present = span!(Level::TRACE, "present").entered();
        renderer.present(
            future
                .then_execute(queue.clone(), command_buffer)
                .context("then execute")?
                .boxed(),
            false,
        );
        present.exit();

        Ok(())
    }
}
