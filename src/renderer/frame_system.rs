use std::sync::Arc;

use anyhow::Context;
use cgmath::Matrix4;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBufferBeginInfo, CommandBufferLevel,
        CommandBufferUsage, RecordingCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo,
        SubpassContents,
    },
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::Queue,
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    memory::allocator::{
        AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, StandardMemoryAllocator,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};

use super::{frame::Frame, lighting};

pub struct FrameSystem {
    pub gfx_queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,

    render_pass: Arc<RenderPass>,

    pub diffuse_buffer: Arc<ImageView>,
    pub normals_buffer: Arc<ImageView>,
    pub depth_buffer: Arc<ImageView>,

    pub ambient_lighting_system: lighting::Ambient,
    pub directional_lighting_system: lighting::Directional,
    pub point_lighting_system: lighting::Point,
}

impl FrameSystem {
    pub fn new(
        gfx_queue: Arc<Queue>,
        image_format: Format,
        memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    ) -> anyhow::Result<Self> {
        let render_pass = vulkano::ordered_passes_renderpass!(
            gfx_queue.device().clone(),
            attachments: {
                final_color: {
                    format: image_format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                diffuse: {
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                },
                normals: {
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                },
                depth_stencil: {
                    format: Format::D16_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                },
            },
            passes: [
                {
                    color: [diffuse, normals],
                    depth_stencil: {depth_stencil},
                    input: [],
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [diffuse, normals, depth_stencil],
                },
            ],
        )
        .context("creating RenderPass")?;

        // create temp images that will be recreated when frame() is called
        let diffuse_buffer = ImageView::new_default(
            Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    extent: [1, 1, 1],
                    usage: ImageUsage::COLOR_ATTACHMENT
                        | ImageUsage::TRANSIENT_ATTACHMENT
                        | ImageUsage::INPUT_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .context("creating initial diffuse buffer image")?,
        )
        .context("creating initial diffuse buffer image view")?;

        let normals_buffer = ImageView::new_default(
            Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::R16G16B16A16_SFLOAT,
                    extent: [1, 1, 1],
                    usage: ImageUsage::TRANSIENT_ATTACHMENT | ImageUsage::INPUT_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .context("creating initial normals buffer image")?,
        )
        .context("creating initial normals buffer image view")?;

        let depth_buffer = ImageView::new_default(
            Image::new(
                memory_allocator.clone(),
                ImageCreateInfo {
                    image_type: ImageType::Dim2d,
                    format: Format::D16_UNORM,
                    extent: [1, 1, 1],
                    usage: ImageUsage::TRANSIENT_ATTACHMENT | ImageUsage::INPUT_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default(),
            )
            .context("creating initial depth buffer image")?,
        )
        .context("creating initial depth buffer image view")?;

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            gfx_queue.device().clone(),
            Default::default(),
        ));

        let lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let ambient_lighting_system = lighting::Ambient::new(
            gfx_queue.clone(),
            lighting_subpass.clone(),
            memory_allocator.clone(),
            command_buffer_allocator.clone(),
            descriptor_set_allocator.clone(),
        )
        .context("creating ambient lighting system")?;

        let directional_lighting_system = lighting::Directional::new(
            gfx_queue.clone(),
            lighting_subpass.clone(),
            memory_allocator.clone(),
            command_buffer_allocator.clone(),
            descriptor_set_allocator.clone(),
        )
        .context("creating directional lighting system")?;

        let point_lighting_system = lighting::Point::new(
            gfx_queue.clone(),
            lighting_subpass,
            memory_allocator.clone(),
            command_buffer_allocator.clone(),
            descriptor_set_allocator,
        )
        .context("creating point lighting system")?;

        Ok(FrameSystem {
            gfx_queue,
            memory_allocator,
            command_buffer_allocator,
            render_pass,
            diffuse_buffer,
            normals_buffer,
            depth_buffer,
            ambient_lighting_system,
            directional_lighting_system,
            point_lighting_system,
        })
    }

    pub fn frame<F>(
        &mut self,
        before_future: F,
        final_image_view: Arc<ImageView>,
        world_to_framebuffer: Matrix4<f32>,
    ) -> anyhow::Result<Frame>
    where
        F: GpuFuture + 'static,
    {
        let extent = final_image_view.image().extent();

        if self.diffuse_buffer.image().extent() != extent {
            self.diffuse_buffer = ImageView::new_default(
                Image::new(
                    self.memory_allocator.clone(),
                    ImageCreateInfo {
                        extent,
                        format: Format::A2B10G10R10_UNORM_PACK32,
                        usage: ImageUsage::COLOR_ATTACHMENT
                            | ImageUsage::TRANSIENT_ATTACHMENT
                            | ImageUsage::INPUT_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .context("creating new diffuse buffer")?,
            )
            .context("creating new diffuse buffer image view")?;

            self.normals_buffer = ImageView::new_default(
                Image::new(
                    self.memory_allocator.clone(),
                    ImageCreateInfo {
                        extent,
                        format: Format::R16G16B16A16_SFLOAT,
                        usage: ImageUsage::COLOR_ATTACHMENT
                            | ImageUsage::TRANSIENT_ATTACHMENT
                            | ImageUsage::INPUT_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .context("creating new normals buffer")?,
            )
            .context("creating new normals buffer image view")?;

            self.depth_buffer = ImageView::new_default(
                Image::new(
                    self.memory_allocator.clone(),
                    ImageCreateInfo {
                        extent,
                        format: Format::D16_UNORM,
                        usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT
                            | ImageUsage::TRANSIENT_ATTACHMENT
                            | ImageUsage::INPUT_ATTACHMENT,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .context("creating new depth buffer")?,
            )
            .context("creating new depth buffer image view")?;
        }

        let framebuffer = Framebuffer::new(
            self.render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![
                    final_image_view,
                    self.diffuse_buffer.clone(),
                    self.normals_buffer.clone(),
                    self.depth_buffer.clone(),
                ],
                ..Default::default()
            },
        )
        .context("creating framebuffer")?;

        let mut command_buffer_builder = RecordingCommandBuffer::new(
            self.command_buffer_allocator.clone(),
            self.gfx_queue.queue_family_index(),
            CommandBufferLevel::Primary,
            CommandBufferBeginInfo {
                usage: CommandBufferUsage::OneTimeSubmit,
                ..Default::default()
            },
        )
        .context("creating primary command buffer")?;

        command_buffer_builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![
                        Some([0.0, 0.0, 0.0, 0.0].into()),
                        Some([0.0, 0.0, 0.0, 0.0].into()),
                        Some([0.0, 0.0, 0.0, 0.0].into()),
                        Some(1.0f32.into()),
                    ],
                    ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::SecondaryCommandBuffers,
                    ..Default::default()
                },
            )
            .context("beginning renderpass on primary command buffer")?;

        Ok(Frame::new(
            self,
            framebuffer,
            Some(Box::new(before_future)),
            Some(command_buffer_builder),
            world_to_framebuffer,
        ))
    }

    #[inline]
    pub fn deferred_subpass(&self) -> Subpass {
        Subpass::from(self.render_pass.clone(), 0).unwrap()
    }
}
