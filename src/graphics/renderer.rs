use std::sync::Arc;

use anyhow::Context;

use cgmath::{Matrix4, SquareMatrix};
use log::{error, info};

use tracing::{event, span, Level};
#[cfg(target_os = "macos")]
use vulkano::instance::InstanceCreateFlags;

use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        Buffer, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder,
        CommandBufferExecFuture, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::ImageUsage,
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::{graphics::viewport::Viewport, GraphicsPipeline, Pipeline},
    render_pass::{Framebuffer, RenderPass},
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

use crate::game::{Camera, Transform};

use super::{
    helpers,
    mesh::{BasicMesh, MeshBuilder},
    shaders::{self, VertexPositionColor},
};
type MyJoinFuture = JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>;
type MyCommandBufferFuture = CommandBufferExecFuture<MyJoinFuture>;
type MyPresentFuture = PresentFuture<MyCommandBufferFuture>;
type MyFenceSignalFuture = FenceSignalFuture<MyPresentFuture>;
type FenceSignalFuturesList = Vec<Option<Arc<MyFenceSignalFuture>>>;

pub struct Renderer {
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,

    viewport: Viewport,
    window_resized: bool,
    dimensions: PhysicalSize<u32>,
    need_swapchain_recreation: bool,

    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: StandardCommandBufferAllocator,
    buffer_allocator: SubbufferAllocator,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,

    queue: Arc<Queue>,
    meshes: Vec<BasicMesh>,

    pipeline: Arc<GraphicsPipeline>,
    render_pass: Arc<RenderPass>,

    // Per Frame Data
    previous_fence_i: u32,
    fences: FenceSignalFuturesList,
    uniform_buffers: Vec<Subbuffer<shaders::vs_position_color::FrameData>>,
    framebuffers: Vec<Arc<Framebuffer>>,

    object_data: Vec<shaders::vs_position_color::ObjectData>,
    cam_matrices: (Matrix4<f32>, Matrix4<f32>),
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

        let render_pass = helpers::get_render_pass(device.clone(), swapchain.clone())?;

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let framebuffers =
            helpers::get_framebuffers(&images, render_pass.clone(), memory_allocator.clone())?;

        let vs = shaders::vs_position_color::load(device.clone())
            .expect("failed to create shader module");
        let fs = shaders::fs_basic::load(device.clone()).expect("failed to create shader module");

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

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let command_buffer_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        // Create the buffer allocator.
        let buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::STORAGE_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

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

        Ok(Renderer {
            device,
            swapchain,
            render_pass,
            viewport,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            queue,
            window_resized: true,
            dimensions: window.inner_size(),
            need_swapchain_recreation: true,
            fences: vec![None; frames_in_flight],
            previous_fence_i: 0,
            uniform_buffers,
            meshes: vec![],
            framebuffers,
            pipeline,
            object_data: vec![],
            buffer_allocator,
            cam_matrices: (Matrix4::identity(), Matrix4::identity()),
        })
    }

    pub fn create_mesh(
        &mut self,
        verts: Vec<VertexPositionColor>,
        indices: Vec<u16>,
    ) -> anyhow::Result<usize> {
        let position = self.meshes.len();
        let mesh = MeshBuilder::default()
            .with_vertices(verts)
            .with_indices(indices)
            .build(self.memory_allocator.clone())
            .context("building mesh")?;
        self.meshes.push(mesh);
        Ok(position)
    }

    pub fn window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.window_resized = true;
        self.dimensions = new_size;
    }

    pub fn draw(&mut self) -> anyhow::Result<()> {
        let resize_swapchain = span!(Level::INFO, "resizing swapchain").entered();

        let f = &mut self.fences[self.previous_fence_i as usize].as_mut();
        match f {
            Some(f) => f.cleanup_finished(),
            None => {}
        };

        let is_zero = self.dimensions.height == 0 || self.dimensions.width == 0;

        if (self.window_resized || self.need_swapchain_recreation) && !is_zero {
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

            self.framebuffers = Vec::new();

            self.framebuffers = helpers::get_framebuffers(
                &new_images,
                self.render_pass.clone(),
                self.memory_allocator.clone(),
            )
            .context("recreating framebuffers")?;

            if self.window_resized {
                self.viewport.extent = self.dimensions.into();
            }
            self.window_resized = false;
        }
        resize_swapchain.exit();

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
        fence_wait.exit();

        let update_uniforms = span!(Level::INFO, "update_uniforms").entered();
        // Update Per Frame buffers here
        *self.uniform_buffers[image_i as usize].write()? = shaders::vs_position_color::FrameData {
            view: self.cam_matrices.1.into(),
            proj: self.cam_matrices.0.into(),
        };
        update_uniforms.exit();

        event!(Level::INFO, "objects: {}", self.object_data.len());

        let command_buffer = self.record_command_buffer(image_i, &self.meshes)?;

        self.object_data = Vec::new();

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

        let span2 = span!(Level::INFO, "await_fence").entered();
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
        span2.exit();

        self.previous_fence_i = image_i;
        Ok(())
    }

    // TODO: eventually hook up correlating mesh_id to transforms here
    // Use the mesh_id to order the transforms in the buffer
    // Then loop over meshes, passing mesh_id to the draw call as instance_id
    pub fn enqueue_mesh(&mut self, _mesh_id: usize, transform: Transform) {
        let d = shaders::vs_position_color::ObjectData {
            model: transform.model().into(),
        };
        self.object_data.push(d);
    }

    pub fn set_camera_params(&mut self, cam_matrices: (Matrix4<f32>, Matrix4<f32>)) {
        self.cam_matrices = cam_matrices;
    }

    pub fn record_command_buffer(
        &self,
        index: u32,
        meshes: &Vec<BasicMesh>,
    ) -> anyhow::Result<Arc<PrimaryAutoCommandBuffer>> {
        let _span = span!(Level::INFO, "record_command_buffer").entered();
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
        )?;

        let object_buffer_span = span!(Level::INFO, "update object buffer").entered();
        let object_data_buffer = self
            .buffer_allocator
            .allocate_slice(self.object_data.len() as _)?;
        object_data_buffer
            .write()?
            .copy_from_slice(&self.object_data);
        object_buffer_span.exit();

        let span_ds = span!(Level::INFO, "create object descriptor set").entered();
        let object_data_buffer_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            self.pipeline.layout().set_layouts()[1].clone(),
            [WriteDescriptorSet::buffer(0, object_data_buffer)],
            [],
        )
        .context("Creating Object Data Descriptor Set")?;
        span_ds.exit();

        let uniform_set = span!(Level::INFO, "create uniform descriptor set").entered();
        let uniform_buffer_set = PersistentDescriptorSet::new(
            &self.descriptor_set_allocator,
            self.pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::buffer(
                0,
                self.uniform_buffers[index as usize].clone(),
            )],
            [],
        )
        .context("creating uniform buffer descriptor set")?;
        uniform_set.exit();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.392, 0.494, 0.929, 1.0].into()), Some(1f32.into())],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[index as usize].clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?
            .set_viewport(0, [self.viewport.clone()].into_iter().collect())?
            .bind_pipeline_graphics(self.pipeline.clone())?
            .bind_descriptor_sets(
                vulkano::pipeline::PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                vec![uniform_buffer_set.clone(), object_data_buffer_set.clone()],
            )?;
        event!(Level::INFO, "meshes: {}", meshes.len());
        for (i, mesh) in meshes.iter().enumerate() {
            builder
                .bind_vertex_buffers(0, mesh.vertex_buffer.clone())?
                .bind_index_buffer(mesh.index_buffer.clone())?
                .draw_indexed(mesh.index_buffer.len() as u32, 1, 0, 0, i as u32)?;
        }

        builder.end_render_pass(Default::default())?;

        builder.build().context("building command buffer")
    }
}
