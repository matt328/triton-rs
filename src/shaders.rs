use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::BufferContents,
    device::Device,
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            subpass::PipelineRenderingCreateInfo,
            vertex_input::{Vertex, VertexDefinition},
            viewport::ViewportState,
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    swapchain::Swapchain,
};

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Position {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}

pub fn create_pipeline(
    device: &Arc<Device>,
    swapchain: &Swapchain,
) -> anyhow::Result<Arc<GraphicsPipeline>> {
    let pipeline = {
        let vs = vs::load(device.clone())?
            .entry_point("main")
            .context("Loading Vertex Shader")?;
        let fs = fs::load(device.clone())?
            .entry_point("main")
            .context("Loading Fragment Shader")?;

        let vertex_input_state = Position::per_vertex()
            .definition(&vs.info().input_interface)
            .context("Creating Vertex Input State")?;

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .context("Creating DSL CreateInfo")?,
        )
        .context("Creating DSL")?;

        let subpass = PipelineRenderingCreateInfo {
            color_attachment_formats: vec![Some(swapchain.image_format())],
            ..Default::default()
        };

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
                    subpass.color_attachment_formats.len() as u32,
                    ColorBlendAttachmentState::default(),
                )),
                dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .context("Creating Pipeline")?
    };
    Ok(pipeline)
}
