use std::sync::Arc;

use anyhow::Context;
use log::info;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features, Queue,
        QueueCreateInfo, QueueFlags,
    },
    image::{view::ImageView, Image, ImageUsage},
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline},
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    swapchain::{
        acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{self, GpuFuture},
    Validated, Version, VulkanError, VulkanLibrary,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::shaders::{create_pipeline, Position};

pub struct Renderer {
    recreate_swapchain: bool,
    window_size: PhysicalSize<u32>,
    swapchain: Arc<Swapchain>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    attachment_image_views: Vec<Arc<ImageView>>,
    viewport: Viewport,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    vertex_buffer: Subbuffer<[Position]>,
    device: Arc<Device>,
}

impl Renderer {
    pub fn new(
        required_extensions: InstanceExtensions,
        window: Arc<Window>,
    ) -> anyhow::Result<Self> {
        let library = VulkanLibrary::new().context("Creating Vulkano Library")?;

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                #[cfg(target_os = "macos")]
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                max_api_version: Some(Version::V1_2),
                enabled_extensions: InstanceExtensions {
                    #[cfg(target_os = "macos")]
                    khr_portability_enumeration: true,
                    ..required_extensions.clone()
                },
                ..Default::default()
            },
        )
        .context("Creating Instance")?;

        let surface = Surface::from_window(instance.clone(), window.clone())
            .context("Getting Surface from Window")?;

        let mut device_extensions = DeviceExtensions {
            khr_dynamic_rendering: true,
            #[cfg(target_os = "macos")]
            khr_portability_subset: true,
            khr_swapchain: true,
            ..Default::default()
        };

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .context("Enumerating Physical Devices")?
            .filter(|p| {
                p.api_version() >= Version::V1_3 || p.supported_extensions().khr_dynamic_rendering
            })
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .context("Getting Physical Device and Queue")?;

        info!(
            "Using Device {}, type: {:?}",
            physical_device.properties().device_name,
            physical_device.properties().device_type
        );

        if physical_device.api_version() < Version::V1_3 {
            device_extensions.khr_dynamic_rendering = true;
        }

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                enabled_features: Features {
                    dynamic_rendering: true,
                    ..Default::default()
                },

                ..Default::default()
            },
        )
        .context("Creating Device and Queues")?;

        let queue = queues.next().context("Getting queue")?;

        let (swapchain, images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can only pass
            // values that are allowed by the capabilities.
            let surface_capabilities = device
                .physical_device()
                .surface_capabilities(&surface, Default::default())
                .context("Getting Surface Capabilities")?;

            // Choosing the internal format that the images will have.
            let image_format = device
                .physical_device()
                .surface_formats(&surface, Default::default())
                .context("Getting Surface Formats")?[0]
                .0;

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: surface_capabilities.min_image_count.max(2),
                    image_format,
                    image_extent: window.inner_size().into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha: surface_capabilities
                        .supported_composite_alpha
                        .into_iter()
                        .next()
                        .context("Finding composite alpha capability")?,
                    ..Default::default()
                },
            )
            .context("Creating Swapchain")?
        };

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let pipeline = create_pipeline(&device, &swapchain)?;

        // Viewport is Dynamic so just set it up with 0s initially
        let mut viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let attachment_image_views = window_size_dependent_setup(&images, &mut viewport).unwrap();

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let vertices = [
            Position {
                position: [-0.5, -0.25],
            },
            Position {
                position: [0.0, 0.5],
            },
            Position {
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
        .context("creating vertex buffer")?;

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        Ok(Renderer {
            recreate_swapchain: true,
            window_size: window.inner_size(),
            swapchain,
            previous_frame_end,
            attachment_image_views,
            viewport,
            command_buffer_allocator,
            queue,
            pipeline,
            vertex_buffer,
            device,
        })
    }

    pub fn resized(&mut self, new_size: PhysicalSize<u32>) {
        self.recreate_swapchain = true;
        self.window_size = new_size;
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        let image_extent: [u32; 2] = self.window_size.into();
        if image_extent.contains(&0) {
            return Ok(());
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            let (new_swapchain, new_images) = self
                .swapchain
                .recreate(SwapchainCreateInfo {
                    image_extent,
                    ..self.swapchain.create_info()
                })
                .expect("failed to recreate swapchain");

            self.swapchain = new_swapchain;

            self.attachment_image_views =
                window_size_dependent_setup(&new_images, &mut self.viewport).unwrap();

            self.recreate_swapchain = false;
        }
        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Ok(());
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .context("Creating command buffer builder")?;

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some([0.0, 0.0, 1.0, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        self.attachment_image_views[image_index as usize].clone(),
                    )
                })],
                ..Default::default()
            })?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())
            .context("Setting Viewport")?
            .bind_pipeline_graphics(self.pipeline.clone())
            .context("Binding Pipeline")?
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .context("Binding Vertex Buffers")?
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .context("Drawing")?
            .end_rendering()
            .context("Ending Rendering")?;

        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .context("Taking from previous future")?
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .context("Executing Queue")?
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }

        Ok(())
    }
}

fn window_size_dependent_setup(
    images: &[Arc<Image>],
    viewport: &mut Viewport,
) -> anyhow::Result<Vec<Arc<ImageView>>> {
    let extent = images[0].extent();
    viewport.extent = [extent[0] as f32, extent[1] as f32];
    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).context("Creating ImageView"))
        .collect()
}
