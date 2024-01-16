use std::sync::Arc;

use anyhow::Context;
use cgmath::Matrix4;
use tracing::{span, Level};
use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        BufferUsage, Subbuffer,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBuffer, CommandBufferBeginInfo,
        CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsage,
        RecordingCommandBuffer,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, DescriptorSet, DescriptorSetsCollection,
        WriteDescriptorSet,
    },
    device::Queue,
    memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator},
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
        DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::Subpass,
};

use crate::game::Transform;

use super::{
    geometry_shaders::{
        fs,
        vs::{self, FrameData, ObjectData},
        VertexPositionColorNormal,
    },
    mesh::MeshBuilder,
    render_data::RenderData,
};

pub struct GeometrySystem {
    gfx_queue: Arc<Queue>,
    subpass: Subpass,
    pipeline: Arc<GraphicsPipeline>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    render_data: RenderData,
    storage_buffer_allocator: SubbufferAllocator,
    uniform_buffer_allocator: SubbufferAllocator,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
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
            let vertex_input_state = VertexPositionColorNormal::per_vertex()
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
            gfx_queue.device().clone(),
            Default::default(),
        ));

        Ok(GeometrySystem {
            gfx_queue,
            subpass,
            pipeline,
            command_buffer_allocator,
            memory_allocator,
            render_data: { Default::default() },
            storage_buffer_allocator,
            uniform_buffer_allocator,
            descriptor_set_allocator,
        })
    }

    /// Builds a secondary command buffer that draws the triangle on the current subpass.
    pub fn draw(&mut self, viewport_dimensions: [u32; 2]) -> anyhow::Result<Arc<CommandBuffer>> {
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

        let descriptor_sets = self.create_descriptor_sets(&self.render_data)?;

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
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_sets,
            )
            .context("binding descriptor sets")?;

        for data in self.render_data.render_iter() {
            let (index, mesh) = data;
            unsafe {
                builder
                    .bind_vertex_buffers(0, mesh.vertex_buffer.clone())?
                    .bind_index_buffer(mesh.index_buffer.clone())?
                    .draw_indexed(mesh.index_buffer.len() as u32, 1, 0, 0, index)
            }?;
        }

        self.render_data.reset_object_data();

        builder.end().context("building command buffer")
    }

    pub fn create_mesh(
        &mut self,
        verts: Vec<VertexPositionColorNormal>,
        indices: Vec<u16>,
    ) -> anyhow::Result<usize> {
        let position = self.render_data.mesh_position();
        let mesh = MeshBuilder::default()
            .with_vertices(verts)
            .with_indices(indices)
            .build(self.memory_allocator.clone())
            .context("building mesh")?;
        self.render_data.add_mesh(mesh);
        Ok(position)
    }

    pub fn enqueue_mesh(&mut self, mesh_id: usize, transform: Transform) {
        let d = ObjectData {
            model: transform.model().into(),
        };
        self.render_data.add_object_data(mesh_id, d);
    }

    pub fn set_camera_params(&mut self, cam_matrices: (Matrix4<f32>, Matrix4<f32>)) {
        self.render_data.update_cam_matrices(cam_matrices);
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
        let object_data_buffer_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
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
        let uniform_buffer_set = DescriptorSet::new(
            self.descriptor_set_allocator.clone(),
            self.pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(0, uniform_buffer)],
            [],
        )
        .context("creating uniform buffer descriptor set")?;
        uniform_set.exit();
        Ok(vec![uniform_buffer_set, object_data_buffer_set])
    }
}
