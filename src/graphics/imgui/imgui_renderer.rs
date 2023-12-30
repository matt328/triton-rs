use std::sync::Arc;

use anyhow::Context;
use imgui::{Context as ImGuiContext, DrawVert, TextureId, Textures};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use vulkano::{
    buffer::{allocator::SubbufferAllocator, BufferContents},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        PrimaryAutoCommandBuffer,
    },
    device::{Device, Queue},
    image::{sampler::Sampler, view::ImageView, Image},
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
    render_pass::{RenderPass, Subpass},
};
use winit::window::Window;

use super::shader;

#[derive(Default, Debug, Clone, BufferContents, Vertex, Copy)]
#[repr(C)]
struct ImGuiVertex {
    #[format(R32G32_SFLOAT)]
    pub pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_SFLOAT)]
    pub col: u32,
}

impl From<DrawVert> for ImGuiVertex {
    fn from(v: DrawVert) -> ImGuiVertex {
        unsafe { std::mem::transmute(v) }
    }
}

pub type ImGuiTexture = (Arc<ImageView>, Arc<Sampler>);

pub struct ImGuiRenderer {
    imgui: ImGuiContext,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    // font_texture: ImGuiTexture,
    // textures: Textures<ImGuiTexture>,
    // vertex_buffer_pool: SubbufferAllocator,
    // index_buffer_pool: SubbufferAllocator,
}

impl ImGuiRenderer {
    pub fn new(
        device: Arc<Device>,
        window: &Window,
        command_buffer_allocator: &StandardCommandBufferAllocator,
        images: &[Arc<Image>],
        viewport: Viewport,
        image_upload_queue: Arc<Queue>,
    ) -> anyhow::Result<Self> {
        let imgui = ImGuiContext::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);

        let vs = shader::vs::load(device.clone())?;
        let fs = shader::fs::load(device.clone())?;

        let format = images[0].format();

        let render_pass = vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    format: format,
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )?;

        let pipeline = {
            let vs_entry = vs.entry_point("main").context("getting entry point")?;
            let fs_entry = fs.entry_point("main").context("getting entry point")?;

            let vertex_input_state = ImGuiVertex::per_vertex()
                .definition(&vs_entry.info().input_interface)
                .context("creating vertex input state")?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs_entry),
                PipelineShaderStageCreateInfo::new(fs_entry),
            ];

            let layout = PipelineLayout::new(
                device.clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(device.clone())
                    .context("creating pipeline layout info")?,
            )?;

            let subpass = Subpass::from(render_pass.clone(), 0).context("creating subpass")?;

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState {
                        viewports: [viewport].into_iter().collect(),
                        ..Default::default()
                    }),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState::default(),
                    )),
                    depth_stencil_state: Some(DepthStencilState {
                        depth: Some(DepthState::simple()),
                        ..Default::default()
                    }),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        let mut uploads = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            image_upload_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let textures = Textures::new();

        // let font_texture = Self::upload_font_texture(imgui.fonts(), device.clone(), queue.clone());

        Ok(ImGuiRenderer {
            imgui,
            render_pass,
            pipeline,
        })
    }

    pub fn record_command_buffer(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        frame_index: usize,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
