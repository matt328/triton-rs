use std::sync::Arc;

use anyhow::Context;
use imgui::{Context as IGContext, DrawVert};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use vulkano::{
    buffer::{allocator::SubbufferAllocator, BufferContents},
    device::Device,
    image::{sampler::Sampler, view::ImageView},
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, ColorBlendAttachmentState, ColorBlendState, ColorComponents,
            },
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

use super::imgui_shader;

#[derive(Default, Debug, Clone, BufferContents, Vertex, Copy)]
#[repr(C)]
struct ImGuiVertex {
    #[format(R32G32_SFLOAT)]
    pub pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_UINT)]
    pub col: u32,
}

impl From<DrawVert> for ImGuiVertex {
    fn from(v: DrawVert) -> ImGuiVertex {
        unsafe { std::mem::transmute(v) }
    }
}

pub type ImGuiTexture = (Arc<ImageView>, Arc<Sampler>);

pub struct ImGuiContext {
    imgui: IGContext,
    pipeline: Arc<GraphicsPipeline>,
    font_texture: ImGuiTexture,
    vertex_buffer_pool: SubbufferAllocator,
    index_buffer_pool: SubbufferAllocator,
    platform: WinitPlatform,
}
impl ImGuiContext {
    pub fn new(
        device: Arc<Device>,
        window: Arc<Window>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> anyhow::Result<Self> {
        let mut imgui = IGContext::create();
        imgui.set_ini_filename(None);
        imgui.set_renderer_name(Some(format!("triton-vulkano-renderer")));

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);

        let vs = imgui_shader::vs::load(device.clone())?;
        let fs = imgui_shader::fs::load(device.clone())?;

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
                        viewports: [viewport.clone()].into_iter().collect(),
                        ..Default::default()
                    }),
                    dynamic_state: [DynamicState::Viewport, DynamicState::Scissor]
                        .into_iter()
                        .collect(),
                    rasterization_state: Some(RasterizationState::default()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        ColorBlendAttachmentState {
                            blend: Some(AttachmentBlend::alpha()),
                            color_write_enable: true,
                            color_write_mask: ColorComponents::all(),
                        },
                    )),
                    depth_stencil_state: None,
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        Ok(ImGuiContext {
            imgui,
            platform,
            pipeline,
        })
    }
}
