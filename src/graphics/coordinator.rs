use std::sync::Arc;

use anyhow::Context;

use cgmath::Matrix4;
use log::{error, info};

use tracing::{event, span, Level};
#[cfg(target_os = "macos")]
use vulkano::instance::InstanceCreateFlags;

use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferExecFuture, CommandBufferUsage,
    },
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::ImageUsage,
    instance::{
        debug::{DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo},
        Instance, InstanceCreateInfo, InstanceExtensions,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::viewport::Viewport,
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

use crate::{game::Transform, graphics::imgui::ImGuiRenderer};

use super::{
    basic_renderer::BasicRenderer,
    helpers,
    mesh::MeshBuilder,
    render_data::RenderData,
    renderer::Renderer,
    shaders::{self, VertexPositionColor},
};
type MyJoinFuture = JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>;
type MyCommandBufferFuture = CommandBufferExecFuture<MyJoinFuture>;
type MyPresentFuture = PresentFuture<MyCommandBufferFuture>;
type MyFenceSignalFuture = FenceSignalFuture<MyPresentFuture>;
type FenceSignalFuturesList = Vec<Option<Arc<MyFenceSignalFuture>>>;

pub struct RenderCoordinator {
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,

    viewport: Viewport,
    window_resized: bool,
    dimensions: PhysicalSize<u32>,
    need_swapchain_recreation: bool,

    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: StandardCommandBufferAllocator,

    queue: Arc<Queue>,

    // Per Frame Data
    previous_fence_i: u32,
    fences: FenceSignalFuturesList,
    uniform_buffers: Vec<Subbuffer<shaders::vs_position_color::FrameData>>,

    render_data: RenderData,
    basic_renderer: Box<dyn Renderer>,
    imgui_renderer: ImGuiRenderer,
    callback: Option<DebugUtilsMessenger>,
}

impl RenderCoordinator {
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

        let callback = unsafe {
            DebugUtilsMessenger::new(
                instance.clone(),
                DebugUtilsMessengerCreateInfo::user_callback(DebugUtilsMessengerCallback::new(
                    |message_severity, message_type, callback_data| {
                        log::info!("{:?}", callback_data.message);
                    },
                )),
            )
            .ok()
        };

        let surface = Surface::from_window(instance.clone(), window.clone())?;

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            khr_shader_draw_parameters: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) =
            helpers::select_physical_device(&instance, &surface, &device_extensions)?;

        info!(
            "Current Graphics Device is {}",
            physical_device.properties().device_name
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            },
        )
        .context("creating logical device")?;

        // TODO hashmap of queue type to Option<Queue> instead of a single queue

        let queue = queues.next().context("getting a queue")?;

        let (swapchain, images) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .context("getting surface capabilities")?;

            let dimensions = window.inner_size();
            let composite_alpha = caps
                .supported_composite_alpha
                .into_iter()
                .next()
                .context("getting supported composite alpha")?;
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

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        let frames_in_flight = images.len();

        type BuffersType = Subbuffer<shaders::vs_position_color::FrameData>;

        let uniform_buffers = (0..swapchain.image_count())
            .map(|_| {
                Buffer::new_sized(
                    memory_allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::UNIFORM_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                )
                .context("")
            })
            .collect::<anyhow::Result<Vec<BuffersType>>>()?;

        let basic_renderer = Box::new(BasicRenderer::new(
            device.clone(),
            memory_allocator.clone(),
            &images,
            viewport.clone(),
        )?);

        log::info!("Before ImGuiRenderer New");
        let imgui_renderer = ImGuiRenderer::new(
            device.clone(),
            window.clone(),
            &command_buffer_allocator,
            memory_allocator.clone(),
            &images,
            viewport.clone(),
            queue.clone(),
        )?;

        log::info!("After ImGuiRenderer New");

        Ok(RenderCoordinator {
            device,
            swapchain,
            viewport,
            memory_allocator,
            command_buffer_allocator,
            queue,
            window_resized: true,
            dimensions: window.inner_size(),
            need_swapchain_recreation: true,
            fences: vec![None; frames_in_flight],
            previous_fence_i: 0,
            uniform_buffers,
            render_data: { Default::default() },
            basic_renderer,
            imgui_renderer,
            callback,
        })
    }

    pub fn create_mesh(
        &mut self,
        verts: Vec<VertexPositionColor>,
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

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.window_resized = true;
        self.dimensions = new_size;
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        let is_zero_sized_window = self.dimensions.height == 0 || self.dimensions.width == 0;

        if (self.window_resized || self.need_swapchain_recreation) && !is_zero_sized_window {
            self.resize_swapchain()?;
        }

        let acquire_image = span!(Level::INFO, "acquiring swapchain image").entered();
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
        acquire_image.exit();

        let fence_wait = span!(Level::INFO, "awaiting/recreating fence").entered();
        // If the current fence is a thing, wait on it, otherwise silently do nothing
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None)?;
        }

        // Set the previous_future to either the previous fence, or create a new one
        // either way, box it
        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            Some(fence) => fence.boxed(),
        };
        fence_wait.exit();

        self.update_uniforms(image_i as usize)?;

        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )?;

        // self.basic_renderer.record_command_buffer(
        //     image_i as usize,
        //     &mut builder,
        //     &self.render_data,
        // )?;

        self.imgui_renderer
            .record_command_buffer(image_i as usize, &mut builder)?;

        let command_buffer = builder.build().context("Building Command Buffer")?;

        self.render_data.reset_object_data();

        let span = span!(Level::INFO, "present").entered();
        let future = previous_future
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)?
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();
        span.exit();

        let await_fence_span = span!(Level::INFO, "await_fence").entered();
        self.fences[image_i as usize] = match future.map_err(Validated::unwrap) {
            #[allow(clippy::arc_with_non_send_sync)]
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                self.need_swapchain_recreation = true;
                None
            }
            Err(e) => {
                error!("failed to flush future: {:#?}", e);
                None
            }
        };
        await_fence_span.exit();

        self.previous_fence_i = image_i;
        Ok(())
    }

    pub fn enqueue_mesh(&mut self, mesh_id: usize, transform: Transform) {
        let d = shaders::vs_position_color::ObjectData {
            model: transform.model().into(),
        };
        self.render_data.add_object_data(mesh_id, d);
    }

    pub fn set_camera_params(&mut self, cam_matrices: (Matrix4<f32>, Matrix4<f32>)) {
        self.render_data.update_cam_matrices(cam_matrices);
    }

    pub fn update_uniforms(&mut self, index: usize) -> anyhow::Result<()> {
        let _span = span!(Level::INFO, "update_uniforms").entered();
        *self.uniform_buffers[index].write()? = shaders::vs_position_color::FrameData {
            view: self.render_data.cam_matrices().1.into(),
            proj: self.render_data.cam_matrices().0.into(),
        };
        Ok(())
    }

    pub fn resize_swapchain(&mut self) -> anyhow::Result<()> {
        let _resize_swapchain = span!(Level::INFO, "resizing swapchain").entered();
        event!(Level::INFO, "recreating swapchain");
        self.need_swapchain_recreation = false;

        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: self.dimensions.into(),
                ..self.swapchain.create_info()
            })
            .context("failed to recreate swapchain")?;

        self.swapchain = new_swapchain;

        if self.window_resized {
            self.viewport.extent = self.dimensions.into();
        }

        let result = self.basic_renderer.resize(&new_images);

        self.window_resized = false;
        result
    }
}
