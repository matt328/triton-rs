use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo},
    image::ImageUsage,
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    swapchain::{Surface, Swapchain, SwapchainCreateInfo},
};
use winit::window::Window;

use super::{helpers, mesh::MeshBuilder, shaders};

pub struct Renderer {}

impl Renderer {
    pub fn new(extensions: InstanceExtensions, window: Arc<Window>) -> anyhow::Result<Self> {
        let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");

        let create_info = InstanceCreateInfo {
            #[cfg(target_os = "macos")]
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: InstanceExtensions {
                #[cfg(target_os = "macos")]
                khr_portability_enumeration: true,
                ..extensions
            },
            ..Default::default()
        };

        let instance = Instance::new(library, create_info).context("creating instance")?;

        let surface = Surface::from_window(instance.clone(), window.clone())?;

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) =
            helpers::select_physical_device(&instance, &surface, &device_extensions)?;

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
                ..Default::default()
            },
        )
        .context("creating logical device")?;

        let queue = queues.next().context("getting a queue")?;

        let (mut swapchain, images) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .context("getting surface capabilities")?;

            let dimensions = window.inner_size();
            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
            let image_format = physical_device
                .surface_formats(&surface, Default::default())
                .context("getting surface formats")?[0]
                .0;

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    ..Default::default()
                },
            )?
        };

        let render_pass = helpers::get_render_pass(device.clone(), swapchain.clone())?;

        let framebuffers = helpers::get_framebuffers(&images, render_pass.clone())?;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let vertices = vec![
            shaders::Position {
                position: [-0.5, -0.5],
            },
            shaders::Position {
                position: [0.0, 0.5],
            },
            shaders::Position {
                position: [0.5, -0.25],
            },
        ];

        let mesh = MeshBuilder::default()
            .with_vertices(vertices)
            .build(memory_allocator.clone())
            .context("building mesh")?;

        let vs = shaders::vs::load(device.clone()).expect("failed to create shader module");
        let fs = shaders::fs::load(device.clone()).expect("failed to create shader module");

        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let pipeline = helpers::get_pipeline(
            device.clone(),
            vs.clone(),
            fs.clone(),
            render_pass.clone(),
            viewport.clone(),
        )?;

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let mut command_buffers = helpers::get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipeline,
            &framebuffers,
            &mesh.vertex_buffer,
        );

        Ok(Renderer {})
    }
}
