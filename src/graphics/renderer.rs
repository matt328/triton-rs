use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator, CommandBufferExecFuture,
        PrimaryAutoCommandBuffer,
    },
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::ImageUsage,
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::StandardMemoryAllocator,
    pipeline::graphics::viewport::Viewport,
    render_pass::RenderPass,
    shader::ShaderModule,
    swapchain::{
        self, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
        SwapchainPresentInfo,
    },
    sync::{
        self,
        future::{FenceSignalFuture, JoinFuture},
        GpuFuture,
    },
    Validated, VulkanError,
};
use winit::{dpi::PhysicalSize, window::Window};

use super::{
    helpers,
    mesh::{BasicMesh, MeshBuilder},
    shaders,
};

pub struct Renderer {
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,

    viewport: Viewport,
    window_resized: bool,
    dimensions: PhysicalSize<u32>,
    need_swapchain_recreation: bool,

    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,

    command_buffer_allocator: StandardCommandBufferAllocator,
    queue: Arc<Queue>,

    previous_fence_i: u32,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    fences: Vec<
        Option<
            Arc<
                FenceSignalFuture<
                    PresentFuture<
                        CommandBufferExecFuture<
                            JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>,
                        >,
                    >,
                >,
            >,
        >,
    >,

    mesh: BasicMesh,
}

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

        let (swapchain, images) = {
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

        let viewport = Viewport {
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

        let command_buffers = helpers::get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipeline,
            &framebuffers,
            &mesh.vertex_buffer,
        )?;

        let frames_in_flight = images.len();

        Ok(Renderer {
            device,
            swapchain,
            render_pass,
            viewport,
            vs,
            fs,
            command_buffer_allocator,
            command_buffers,
            queue,
            mesh,
            window_resized: true,
            dimensions: window.inner_size(),
            need_swapchain_recreation: true,
            fences: vec![None; frames_in_flight],
            previous_fence_i: 0,
        })
    }

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.window_resized = true;
        self.dimensions = new_size;
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        if self.window_resized || self.need_swapchain_recreation {
            self.need_swapchain_recreation = false;

            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent: self.dimensions.into(),
                    ..self.swapchain.create_info()
                })
                .context("failed to recreate swapchain")?;

            self.swapchain = new_swapchain;

            let new_framebuffers = helpers::get_framebuffers(&new_images, self.render_pass.clone())
                .context("recreating framebuffers")?;

            if self.window_resized {
                self.viewport.extent = self.dimensions.into();

                let new_pipeline = helpers::get_pipeline(
                    self.device.clone(),
                    self.vs.clone(),
                    self.fs.clone(),
                    self.render_pass.clone(),
                    self.viewport.clone(),
                )?;

                self.command_buffers = helpers::get_command_buffers(
                    &self.command_buffer_allocator,
                    &self.queue,
                    &new_pipeline,
                    &new_framebuffers,
                    &self.mesh.vertex_buffer,
                )?;
            }
        }

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.need_swapchain_recreation = true;
                    return Ok(());
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            self.need_swapchain_recreation = true;
        }

        // wait for the fence related to this image to finish (normally this would be the oldest fence)
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None)?;
        }

        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
            // Create a NowFuture
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed(),
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(
                self.queue.clone(),
                self.command_buffers[image_i as usize].clone(),
            )
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future.map_err(Validated::unwrap) {
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                self.need_swapchain_recreation = true;
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        self.previous_fence_i = image_i;
        Ok(())
    }
}
