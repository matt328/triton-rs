use anyhow::{Context, Error, Result};
use std::sync::Arc;
use vulkano::image::view::ImageView;
use vulkano::image::AttachmentImage;
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

use vulkano::format::Format;
use vulkano::image::ImageAccess;
use vulkano::image::SwapchainImage;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, FramebufferCreationError, RenderPass};
use vulkano::sync::GpuFuture;
use vulkano::{
    device::{physical::PhysicalDevice, Device, DeviceOwned, Queue},
    image::ImageUsage,
    memory::allocator::StandardMemoryAllocator,
    swapchain::{Surface, Swapchain, SwapchainCreateInfo},
    sync,
};
use winit::window::Window;

use crate::framework::shaders::{fs, vs, Effect};

use super::shaders::Vertex2;

pub struct GraphicsContext {
    physical_device: Arc<PhysicalDevice>,
    queue_family_index: u32,
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: StandardMemoryAllocator,
}

impl GraphicsContext {
    pub fn new(context: &VulkanoContext, surface: Arc<Surface>) -> anyhow::Result<GraphicsContext> {
        let (mut swapchain, images) = {
            let surface_capabilities = context
                .device()
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .unwrap();
            let image_format = Some(
                context
                    .device()
                    .physical_device()
                    .surface_formats(&surface, Default::default())
                    .unwrap()[0]
                    .0,
            );

            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            Swapchain::new(
                context.device().clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count,
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .unwrap(),
                    ..Default::default()
                },
            )
            .context("Creating Swapchain")?
        };

        let memory_allocator = StandardMemoryAllocator::new_default(context.device().clone());

        let frame_future = Some(sync::now(context.device().clone()).boxed());

        let render_pass = vulkano::single_pass_renderpass!(context.device().clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
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

        let vs = vs::load(context.device().clone()).context("Loading Vertex Shader")?;
        let fs = fs::load(context.device().clone()).context("Loading Pixel Shader")?;

        let default_effect = Effect::builder(vs, fs).build();

        let (pipeline, framebuffers) = GraphicsContext::window_size_dependent_setup(
            &memory_allocator,
            &default_effect,
            images.as_slice(),
            render_pass,
        );

        Ok(GraphicsContext {
            physical_device,
            queue_family_index,
            device,
            queue,
            memory_allocator,
        })
    }

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
    ) -> Result<Arc<GraphicsPipeline>> {
        let input_state = <Vertex2 as Vertex>::per_vertex();

        let viewport_state = ViewportState::viewport_fixed_scissor_irrelevant([Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        }]);

        let subpass = Subpass::from(render_pass, 0)
            .ok_or(Error::msg("Failed to create subpass"))
            .unwrap();

        GraphicsPipeline::start()
            .vertex_input_state(input_state)
            .vertex_shader(effect.vertex.entry_point("main").unwrap(), ())
            .fragment_shader(effect.fragment.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(viewport_state)
            .rasterization_state(RasterizationState::new().polygon_mode(PolygonMode::Line))
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .render_pass(subpass)
            .build(device.clone())
            .context("Failed to build default pipeline")
    }
}
