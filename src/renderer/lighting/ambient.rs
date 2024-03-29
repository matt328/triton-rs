use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBuffer, CommandBufferBeginInfo,
        CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsage,
        RecordingCommandBuffer,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, DescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    image::view::ImageView,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState,
            },
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
};

use super::LightingVertex;

pub struct Ambient {
    gfx_queue: Arc<Queue>,
    vertex_buffer: Subbuffer<[LightingVertex]>,
    subpass: Subpass,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

impl Ambient {
    /// Initializes the ambient lighting system.
    pub fn new(
        gfx_queue: Arc<Queue>,
        subpass: Subpass,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> anyhow::Result<Self> {
        // TODO: vulkano doesn't allow us to draw without a vertex buffer, otherwise we could
        //       hard-code these values in the shader
        let vertices = [
            LightingVertex {
                position: [-1.0, -1.0],
            },
            LightingVertex {
                position: [-1.0, 3.0],
            },
            LightingVertex {
                position: [3.0, -1.0],
            },
        ];
        let vertex_buffer = Buffer::from_iter(
            memory_allocator,
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
        .context("creating vertex buffer")?;

        let pipeline = {
            let device = gfx_queue.device();
            let vs = vs::load(device.clone())
                .context("vertex shader module")?
                .entry_point("main")
                .context("vertex shader module entry point")?;

            let fs = fs::load(device.clone())
                .context("fragment shader module")?
                .entry_point("main")
                .context("fragment shader module entry point")?;

            let vertex_input_state = LightingVertex::per_vertex()
                .definition(&vs.info().input_interface)
                .context("vertex_input_state")?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .context("pipeline dsl create info")?,
            )
            .context("pipeline layout")?;

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend {
                                color_blend_op: BlendOp::Add,
                                src_color_blend_factor: BlendFactor::One,
                                dst_color_blend_factor: BlendFactor::One,
                                alpha_blend_op: BlendOp::Max,
                                src_alpha_blend_factor: BlendFactor::One,
                                dst_alpha_blend_factor: BlendFactor::One,
                            }),
                            ..Default::default()
                        },
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.clone().into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .context("graphics pipeline")?
        };

        Ok(Ambient {
            gfx_queue,
            vertex_buffer,
            subpass,
            pipeline,
            command_buffer_allocator,
            descriptor_set_allocator,
        })
    }

    /// Builds a secondary command buffer that applies ambient lighting.
    ///
    /// This secondary command buffer will read `color_input`, multiply it with `ambient_color`
    /// and write the output to the current framebuffer with additive blending (in other words
    /// the value will be added to the existing value in the framebuffer, and not replace the
    /// existing value).
    ///
    /// - `viewport_dimensions` contains the dimensions of the current framebuffer.
    /// - `color_input` is an image containing the albedo of each object of the scene. It is the
    ///   result of the deferred pass.
    /// - `ambient_color` is the color to apply.
    pub fn draw(
        &self,
        viewport_dimensions: [u32; 2],
        color_input: Arc<ImageView>,
        ambient_color: [f32; 3],
    ) -> anyhow::Result<Arc<CommandBuffer>> {
        let push_constants = fs::PushConstants {
            color: [ambient_color[0], ambient_color[1], ambient_color[2], 1.0],
        };

        let layout = self
            .pipeline
            .layout()
            .set_layouts()
            .get(0)
            .context("pipeline set layouts")?;

        let descriptor_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            layout.clone(),
            [WriteDescriptorSet::image_view(0, color_input)],
            [],
        )?;

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
            depth_range: 0.0..=1.0,
        };

        let mut builder = RecordingCommandBuffer::new(
            self.command_buffer_allocator.clone(),
            self.gfx_queue.queue_family_index(),
            CommandBufferLevel::Secondary,
            CommandBufferBeginInfo {
                usage: CommandBufferUsage::MultipleSubmit,
                inheritance_info: Some(CommandBufferInheritanceInfo {
                    render_pass: Some(self.subpass.clone().into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )?;

        builder
            .set_viewport(0, [viewport].into_iter().collect())?
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )?
            .push_constants(self.pipeline.layout().clone(), 0, push_constants)?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())?;
        unsafe {
            builder.draw(self.vertex_buffer.len() as u32, 1, 0, 0)?;
        }

        builder.end().context("ending command buffer")
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "assets/shaders/deferred/ambient.vert"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "assets/shaders/deferred/ambient.frag"
    }
}
