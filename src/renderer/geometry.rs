use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBuffer, CommandBufferBeginInfo,
        CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsage,
        RecordingCommandBuffer,
    },
    device::Queue,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
};

use super::geometry_shaders::VertexPositionColorNormal;

pub struct GeometrySystem {
    gfx_queue: Arc<Queue>,
    vertex_buffer: Subbuffer<[VertexPositionColorNormal]>,
    subpass: Subpass,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
}

/*
    TODO:
    - remove vertex data from this class.
    - add a RenderData to this class
    - Add methods to the renderer to delegate into this class's RenderData
    - Add create_mesh method to this class

    - change this method's draw call to draw all the data the RenderData has.
    - change the draw call to draw indexed like the other renderer does

    - update the shader to use the object data, as well as the camera data
    - add or update this System's descriptor set to pass the object data and camera data
      to the shader.
    - That should be it?
*/

impl GeometrySystem {
    /// Initializes a triangle drawing system.
    pub fn new(
        gfx_queue: Arc<Queue>,
        subpass: Subpass,
        memory_allocator: Arc<StandardMemoryAllocator>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    ) -> anyhow::Result<Self> {
        let vertices = [
            TriangleVertex {
                position: [-0.5, -0.25],
            },
            TriangleVertex {
                position: [0.0, 0.5],
            },
            TriangleVertex {
                position: [0.25, -0.1],
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
        .expect("failed to create buffer");

        let pipeline = {
            let device = gfx_queue.device();
            let vs = vs::load(device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .expect("shader entry point not found");
            let fs = fs::load(device.clone())
                .expect("failed to create shader module")
                .entry_point("main")
                .expect("shader entry point not found");
            let vertex_input_state = TriangleVertex::per_vertex()
                .definition(&vs.info().input_interface)
                .unwrap();
            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];
            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .unwrap(),
            )
            .context("creating pipeline layout")?;

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState::default()),
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState::simple()),
                        ..Default::default()
                    }),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState::default(),
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.clone().into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .context("creating graphics pipeline")?
        };

        Ok(GeometrySystem {
            gfx_queue,
            vertex_buffer,
            subpass,
            pipeline,
            command_buffer_allocator,
        })
    }

    /// Builds a secondary command buffer that draws the triangle on the current subpass.
    pub fn draw(&self, viewport_dimensions: [u32; 2]) -> anyhow::Result<Arc<CommandBuffer>> {
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
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0, 0.0],
                    extent: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .context("setting viewport")?
            .bind_pipeline_graphics(self.pipeline.clone())
            .context("binding pipeline graphics")?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .context("binding vertex buffers")?;
        unsafe {
            builder
                .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
                .context("drawing")?;
        }

        builder.end().context("building command buffer")
    }
}
