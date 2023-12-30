use std::sync::Arc;

use anyhow::Context;
use tracing::{span, Level};
use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        BufferUsage, Subbuffer,
    },
    command_buffer::{
        AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo,
        SubpassContents,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, DescriptorSetsCollection,
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Device,
    image::Image,
    memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline, Pipeline},
    render_pass::Framebuffer,
};

use super::{
    helpers::{self},
    render_data::RenderData,
    renderer::Renderer,
    shaders::{self, vs_position_color::FrameData},
};

pub struct BasicRenderer {
    device: Arc<Device>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    framebuffers: Vec<Arc<Framebuffer>>,
    pipeline: Arc<GraphicsPipeline>,
    storage_buffer_allocator: SubbufferAllocator,
    uniform_buffer_allocator: SubbufferAllocator,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    viewport: Viewport,
}

impl BasicRenderer {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<StandardMemoryAllocator>,
        images: &[Arc<Image>],
        viewport: Viewport,
    ) -> anyhow::Result<Self> {
        let format = images[0].format();
        let render_pass = helpers::get_render_pass(device.clone(), format)?;

        let framebuffers =
            helpers::get_framebuffers(images, render_pass.clone(), memory_allocator.clone())?;

        let vs = shaders::vs_position_color::load(device.clone())
            .context("failed to create shader module")?;
        let fs =
            shaders::fs_basic::load(device.clone()).context("failed to create shader module")?;

        let pipeline = helpers::get_pipeline(
            device.clone(),
            vs,
            fs,
            render_pass.clone(),
            viewport.clone(),
        )?;

        let storage_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::STORAGE_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let uniform_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Ok(BasicRenderer {
            device,
            memory_allocator,
            framebuffers,
            pipeline,
            descriptor_set_allocator,
            storage_buffer_allocator,
            uniform_buffer_allocator,
            viewport,
        })
    }

    fn create_descriptor_sets(
        &self,
        render_data: &RenderData,
    ) -> anyhow::Result<impl DescriptorSetsCollection> {
        // Update the object data buffer
        let object_buffer_span = span!(Level::INFO, "update object buffer").entered();

        let objects = render_data.object_data();

        let object_data_buffer = self
            .storage_buffer_allocator
            .allocate_slice(objects.len() as _)?;

        object_data_buffer.write()?.copy_from_slice(&objects);

        object_buffer_span.exit();

        // (re)create the object data descriptor set
        let span_ds = span!(Level::INFO, "create object descriptor set").entered();
        let object_data_buffer_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            self.pipeline.layout().set_layouts()[1].clone(),
            [WriteDescriptorSet::buffer(0, object_data_buffer)],
            [],
        )
        .context("Creating Object Data Descriptor Set")?;
        span_ds.exit();

        // Update the uniform buffer
        let uniform_buffer: Subbuffer<FrameData> =
            self.uniform_buffer_allocator.allocate_sized()?;

        *uniform_buffer.write()? = FrameData {
            view: render_data.cam_matrices().1.into(),
            proj: render_data.cam_matrices().0.into(),
        };

        // (re)create the uniform buffer descriptor set
        let uniform_set = span!(Level::INFO, "create uniform descriptor set").entered();
        let uniform_buffer_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            self.pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(0, uniform_buffer)],
            [],
        )
        .context("creating uniform buffer descriptor set")?;
        uniform_set.exit();
        Ok(vec![uniform_buffer_set, object_data_buffer_set])
    }
}
impl Renderer for BasicRenderer {
    fn resize(&mut self, images: &[Arc<Image>]) -> anyhow::Result<()> {
        let format = images[0].format();
        let render_pass = helpers::get_render_pass(self.device.clone(), format)?;

        self.framebuffers =
            helpers::get_framebuffers(images, render_pass.clone(), self.memory_allocator.clone())?;

        Ok(())
    }

    fn record_command_buffer(
        &self,
        frame_index: usize,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        render_data: &RenderData,
    ) -> anyhow::Result<()> {
        let descriptor_sets = self.create_descriptor_sets(render_data)?;

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.392, 0.494, 0.929, 1.0].into()), Some(1f32.into())],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[frame_index].clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_sets,
            )?;

        tracing::event!(Level::INFO, "{render_data:?}");
        for data in render_data.render_iter() {
            let (index, mesh) = data;
            builder
                .bind_vertex_buffers(0, mesh.vertex_buffer.clone())?
                .bind_index_buffer(mesh.index_buffer.clone())?
                .draw_indexed(mesh.index_buffer.len() as u32, 1, 0, 0, index)?;
        }

        builder.end_render_pass(Default::default())?;
        Ok(())
    }
}
