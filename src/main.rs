use std::sync::Arc;

use anyhow::Context;

use tracing::{span, Level};
use triton::shaders::{create_pipeline, Position};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderingAttachmentInfo, RenderingInfo,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Features,
        QueueCreateInfo, QueueFlags,
    },
    image::{view::ImageView, Image, ImageUsage},
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::viewport::Viewport,
    render_pass::{AttachmentLoadOp, AttachmentStoreOp},
    swapchain::{
        acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{self, GpuFuture},
    Validated, Version, VulkanError, VulkanLibrary,
};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::WindowBuilder,
};

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    #[cfg(feature = "tracing")]
    info!("Tracing enabled");

    #[cfg(feature = "tracing")]
    #[global_allocator]
    static GLOBAL: ProfiledAllocator<std::alloc::System> =
        ProfiledAllocator::new(std::alloc::System, 100);

    #[cfg(feature = "tracing")]
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    )
    .expect("set up the subscriber");

    let _root = span!(Level::INFO, "root").entered();

    let event_loop = EventLoop::new().context("Creating Event Loop")?;

    let library = VulkanLibrary::new().context("Creating Vulkano Library")?;

    let required_extensions =
        Surface::required_extensions(&event_loop).context("querying required extensions")?;

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

    let window = Arc::new(
        WindowBuilder::new()
            .build(&event_loop)
            .context("Creating Window")?,
    );
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

    log::info!(
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

    let (mut swapchain, images) = {
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

    let mut attachment_image_views = window_size_dependent_setup(&images, &mut viewport).unwrap();

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

    let mut recreate_swapchain = false;

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    event_loop
        .run(move |event, elwt: &EventLoopWindowTarget<()>| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    elwt.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(_),
                    ..
                } => {
                    recreate_swapchain = true;
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let image_extent: [u32; 2] = window.inner_size().into();

                    if image_extent.contains(&0) {
                        return;
                    }
                    previous_frame_end.as_mut().unwrap().cleanup_finished();

                    if recreate_swapchain {
                        let (new_swapchain, new_images) = swapchain
                            .recreate(SwapchainCreateInfo {
                                image_extent,
                                ..swapchain.create_info()
                            })
                            .expect("failed to recreate swapchain");

                        swapchain = new_swapchain;

                        attachment_image_views =
                            window_size_dependent_setup(&new_images, &mut viewport).unwrap();

                        recreate_swapchain = false;
                    }
                    let (image_index, suboptimal, acquire_future) = match acquire_next_image(
                        swapchain.clone(),
                        None,
                    )
                    .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                    if suboptimal {
                        recreate_swapchain = true;
                    }

                    let mut builder = AutoCommandBufferBuilder::primary(
                        command_buffer_allocator.clone(),
                        queue.queue_family_index(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap();

                    builder
                        .begin_rendering(RenderingInfo {
                            color_attachments: vec![Some(RenderingAttachmentInfo {
                                load_op: AttachmentLoadOp::Clear,
                                store_op: AttachmentStoreOp::Store,
                                clear_value: Some([0.0, 0.0, 1.0, 1.0].into()),
                                ..RenderingAttachmentInfo::image_view(
                                    attachment_image_views[image_index as usize].clone(),
                                )
                            })],
                            ..Default::default()
                        })
                        .unwrap()
                        .set_viewport(0, [viewport.clone()].into_iter().collect())
                        .unwrap()
                        .bind_pipeline_graphics(pipeline.clone())
                        .unwrap()
                        .bind_vertex_buffers(0, vertex_buffer.clone())
                        .unwrap()
                        .draw(vertex_buffer.len() as u32, 1, 0, 0)
                        .unwrap()
                        .end_rendering()
                        .unwrap();

                    let command_buffer = builder.build().unwrap();

                    let future = previous_frame_end
                        .take()
                        .unwrap()
                        .join(acquire_future)
                        .then_execute(queue.clone(), command_buffer)
                        .unwrap()
                        .then_swapchain_present(
                            queue.clone(),
                            SwapchainPresentInfo::swapchain_image_index(
                                swapchain.clone(),
                                image_index,
                            ),
                        )
                        .then_signal_fence_and_flush();

                    match future.map_err(Validated::unwrap) {
                        Ok(future) => {
                            previous_frame_end = Some(future.boxed());
                        }
                        Err(VulkanError::OutOfDate) => {
                            recreate_swapchain = true;
                            previous_frame_end = Some(sync::now(device.clone()).boxed());
                        }
                        Err(e) => {
                            println!("failed to flush future: {e}");
                            previous_frame_end = Some(sync::now(device.clone()).boxed());
                        }
                    }
                }
                _ => (),
            }
        })
        .context("Executing Event Loop")
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
