use anyhow::{Context, Error, Result};
use std::sync::Arc;
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::PipelineLayout;
use vulkano::pipeline::PipelineShaderStageCreateInfo;
use vulkano::Validated;
use vulkano::VulkanError;

use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::rasterization::PolygonMode;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::render_pass::FramebufferCreateInfo;
use vulkano::render_pass::Subpass;
use vulkano_util::context::VulkanoContext;

use vulkano_util::renderer::VulkanoWindowRenderer;

use vulkano::format::Format;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::{
    device::{Device, DeviceOwned},
    memory::allocator::StandardMemoryAllocator,
};

use crate::framework::shaders::{fs, vs, Effect};

use super::shaders::Vertex2;

pub struct GraphicsContext {}

impl GraphicsContext {
    pub fn new(
        context: &VulkanoContext,
        renderer: &VulkanoWindowRenderer,
    ) -> anyhow::Result<GraphicsContext> {
        let render_pass = vulkano::single_pass_renderpass!(context.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: renderer.swapchain_format(),
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .context("Creating render pass")?;

        let vs = vs::load(context.device().clone())
            .unwrap()
            .entry_point("main")
            .context("Loading Vertex Shader")?;
        let fs = fs::load(context.device().clone())
            .unwrap()
            .entry_point("main")
            .context("Loading Pixel Shader")?;

        let default_effect = Effect::builder(vs, fs).build();

        let (pipeline, framebuffers) = GraphicsContext::window_size_dependent_setup(
            &context.memory_allocator(),
            &default_effect,
            images,
            render_pass,
        );

        Ok(GraphicsContext {})
    }
    /// (Re)creates the pipeline and Framebuffers
    fn window_size_dependent_setup(
        memory_allocator: &StandardMemoryAllocator,
        default_effect: &Effect,
        images: &[Arc<SwapchainImage>],
        render_pass: Arc<RenderPass>,
    ) -> (
        anyhow::Result<Arc<GraphicsPipeline>>,
        anyhow::Result<Vec<Arc<Framebuffer>>, FramebufferCreationError>,
    ) {
        let dimensions = images[0].dimensions().width_height();

        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(memory_allocator, dimensions, Format::D16_UNORM).unwrap(),
        )
        .unwrap();

        // Create a Framebuffer for each image
        let framebuffers: Result<Vec<Arc<Framebuffer>>, FramebufferCreationError> = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view, depth_buffer.clone()],
                        ..Default::default()
                    },
                )
            })
            .collect();

        let default_pipeline = GraphicsContext::create_pipeline(
            memory_allocator.device(),
            default_effect,
            dimensions,
            render_pass.clone(),
        )
        .context("Create Default Pipeline");

        (default_pipeline, framebuffers)
    }

    fn create_pipeline(
        device: &Arc<Device>,
        effect: &Effect,
        dimensions: [u32; 2],
        render_pass: Arc<RenderPass>,
    ) -> Result<Arc<GraphicsPipeline>, Validated<VulkanError>> {
        let pipeline = {
            let device = memory_allocator.device();
            let vertex_input_state = [Position::per_vertex(), Normal::per_vertex()]
                .definition(
                    &effect
                        .vertex
                        .entry_point("main")
                        .unwrap()
                        .info()
                        .input_interface,
                )
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
            .unwrap();
            let subpass = Subpass::from(render_pass, 0).unwrap();

            GraphicsPipeline::new(
                device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::viewport_fixed_scissor_irrelevant([
                        Viewport {
                            offset: [0.0, 0.0],
                            extent: [dimensions[0] as f32, dimensions[1] as f32],
                            depth_range: 0.0..=1.0,
                        },
                    ])),
                    rasterization_state: Some(RasterizationState::default()),
                    depth_stencil_state: Some(DepthStencilState::simple_depth_test()),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::new(subpass.num_color_attachments())),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
        };
        pipeline
    }
}
