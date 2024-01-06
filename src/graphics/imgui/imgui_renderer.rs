use std::sync::Arc;

use anyhow::Context;
use imgui::{
    internal::RawWrapper, Condition, Context as ImGuiContext, DrawCmd, DrawCmdParams, DrawIdx,
    DrawVert,
};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use tracing::{span, Level};
use vulkano::{
    buffer::{
        allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
        Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
    },
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyBufferToImageInfo, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
        RenderPassBeginInfo, SubpassBeginInfo, SubpassContents,
    },
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    format::Format,
    image::{
        sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo},
        view::ImageView,
        Image, ImageCreateInfo, ImageType, ImageUsage,
    },
    memory::allocator::{
        AllocationCreateInfo, FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter,
    },
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, ColorBlendAttachmentState, ColorBlendState, ColorComponents,
            },
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Scissor, Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
        PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, Subpass},
    sync::GpuFuture,
    DeviceSize,
};
use winit::window::Window;

use super::shader;

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

pub struct ImGuiRenderer {
    imgui: ImGuiContext,
    pipeline: Arc<GraphicsPipeline>,
    font_texture: ImGuiTexture,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer_pool: SubbufferAllocator,
    index_buffer_pool: SubbufferAllocator,
    descriptor_set_allocator: StandardDescriptorSetAllocator,
    viewport: Viewport,
    window: Arc<Window>,
    platform: WinitPlatform,
}

impl ImGuiRenderer {
    pub fn new(
        device: Arc<Device>,
        window: Arc<Window>,
        command_buffer_allocator: &StandardCommandBufferAllocator,
        memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
        images: &[Arc<Image>],
        viewport: Viewport,
        image_upload_queue: Arc<Queue>,
    ) -> anyhow::Result<Self> {
        let mut imgui = ImGuiContext::create();
        imgui.set_ini_filename(None);
        imgui.set_renderer_name(Some(format!("triton-vulkano-renderer")));

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Rounded);

        let vs = shader::vs::load(device.clone())?;
        let fs = shader::fs::load(device.clone())?;

        let format = images[0].format();

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )?;

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

        let texture = imgui.fonts().build_rgba32_texture();

        image::save_buffer(
            "image.png",
            texture.data,
            texture.width,
            texture.height,
            image::ColorType::Rgba8,
        )?;

        let format = Format::R8G8B8A8_SRGB;
        let extent = [texture.width, texture.height, 1];
        let array_layers = 1;

        let buffer_size = format.block_size()
            * extent
                .into_iter()
                .map(|e| e as DeviceSize)
                .product::<DeviceSize>()
            * array_layers as DeviceSize;

        let upload_buffer: Subbuffer<[u8]> = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            buffer_size,
        )?;

        upload_buffer.write()?.copy_from_slice(texture.data);

        let image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format,
                extent,
                array_layers,
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        let mut uploads = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            image_upload_queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        uploads.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
            upload_buffer,
            image.clone(),
        ))?;

        let command_buffer = uploads.build()?;

        command_buffer
            .execute(image_upload_queue.clone())?
            .then_signal_fence_and_flush()?
            .wait(None)?;

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToBorder; 3],
                lod: 0.0..=1.0,
                ..Default::default()
            },
        )?;

        let font_texture = (ImageView::new_default(image)?, sampler);

        let vertex_buffer_pool = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::VERTEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let index_buffer_pool = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::INDEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let f: anyhow::Result<Vec<Arc<Framebuffer>>> = images
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone())?;
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .context("Creating ImGui Framebuffers")
            })
            .collect();

        let descriptor_set_allocator =
            StandardDescriptorSetAllocator::new(device.clone(), Default::default());

        Ok(ImGuiRenderer {
            imgui,
            pipeline,
            font_texture,
            vertex_buffer_pool,
            index_buffer_pool,
            framebuffers: f?,
            descriptor_set_allocator,
            viewport,
            window,
            platform,
        })
    }

    pub fn record_command_buffer(
        &mut self,
        frame_index: usize,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    ) -> anyhow::Result<()> {
        self.platform
            .prepare_frame(self.imgui.io_mut(), &self.window)?;

        let draw_data = {
            let _span = span!(Level::INFO, "Create UI").entered();
            let ui = self.imgui.new_frame();
            let mut value = 0;
            let choices = ["test test this is 1", "test test this is 2"];
            ui.window("Hello world")
                .size([300.0, 110.0], Condition::FirstUseEver)
                .build(|| {
                    ui.text_wrapped("Hello world!");
                    ui.text_wrapped("こんにちは世界！");
                    if ui.button(choices[value]) {
                        value += 1;
                        value %= 2;
                    }

                    ui.button("This...is...imgui-rs!");
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(format!(
                        "Mouse Position: ({:.1},{:.1})",
                        mouse_pos[0], mouse_pos[1]
                    ));
                });

            self.imgui.render()
        };

        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if !(fb_width > 0.0 && fb_height > 0.0) {
            return Ok(());
        }
        let left = draw_data.display_pos[0];
        let right = draw_data.display_pos[0] + draw_data.display_size[0];
        let top = draw_data.display_pos[1];
        let bottom = draw_data.display_pos[1] + draw_data.display_size[1];

        let pc = shader::vs::VertPC {
            matrix: [
                [(2.0 / (right - left)), 0.0, 0.0, 0.0],
                [0.0, (2.0 / (bottom - top)), 0.0, 0.0],
                [0.0, 0.0, -1.0, 0.0],
                [
                    (right + left) / (left - right),
                    (top + bottom) / (top - bottom),
                    0.0,
                    1.0,
                ],
            ],
        };

        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        let imgui_render_span = span!(Level::INFO, "Record Gui").entered();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![None],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffers[frame_index].clone())
                },
                SubpassBeginInfo {
                    contents: SubpassContents::Inline,
                    ..Default::default()
                },
            )?
            .set_viewport(0, vec![self.viewport.clone()].into())?
            .bind_pipeline_graphics(self.pipeline.clone())?;

        let layout = self.pipeline.layout().set_layouts().get(0).context("")?;

        for draw_list in draw_data.draw_lists() {
            let vertex_data: Vec<ImGuiVertex> = draw_list
                .vtx_buffer()
                .iter()
                .map(|&v| ImGuiVertex::from(v))
                .collect();

            let vertex_buffer = self
                .vertex_buffer_pool
                .allocate_slice(vertex_data.len() as _)?;

            vertex_buffer.write()?.copy_from_slice(&vertex_data);

            let index_data: Vec<DrawIdx> = draw_list.idx_buffer().iter().cloned().collect();

            let index_buffer: Subbuffer<[DrawIdx]> = self
                .index_buffer_pool
                .allocate_slice::<DrawIdx>(index_data.len() as _)?;

            index_buffer.write()?.copy_from_slice(&index_data);

            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                clip_rect,
                                // texture_id,
                                vtx_offset,
                                idx_offset,
                                ..
                            },
                    } => {
                        let clip_rect = [
                            (clip_rect[0] - clip_off[0]) * clip_scale[0],
                            (clip_rect[1] - clip_off[1]) * clip_scale[1],
                            (clip_rect[2] - clip_off[0]) * clip_scale[0],
                            (clip_rect[3] - clip_off[1]) * clip_scale[1],
                        ];

                        if clip_rect[0] < fb_width
                            && clip_rect[1] < fb_height
                            && clip_rect[2] >= 0.0
                            && clip_rect[3] >= 0.0
                        {
                            let scissors = vec![Scissor {
                                offset: [
                                    f32::max(0.0, clip_rect[0]).floor() as u32,
                                    f32::max(0.0, clip_rect[1]).floor() as u32,
                                ],
                                extent: [
                                    (clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                                    (clip_rect[3] - clip_rect[1]).abs().ceil() as u32,
                                ],
                            }];

                            let set = PersistentDescriptorSet::new(
                                &self.descriptor_set_allocator,
                                layout.clone(),
                                [WriteDescriptorSet::image_view_sampler(
                                    0,
                                    self.font_texture.0.clone(),
                                    self.font_texture.1.clone(),
                                )],
                                [],
                            )?;

                            builder
                                .set_scissor(0, scissors.into())?
                                .bind_descriptor_sets(
                                    PipelineBindPoint::Graphics,
                                    self.pipeline.layout().clone(),
                                    0,
                                    set,
                                )?
                                .bind_vertex_buffers(0, vertex_buffer.clone())?
                                .bind_index_buffer(index_buffer.clone())?
                                .push_constants(self.pipeline.layout().clone(), 0, pc)?
                                .draw_indexed(
                                    count as u32,
                                    1,
                                    idx_offset as u32,
                                    vtx_offset as i32,
                                    0,
                                )?;
                        }
                    }
                    DrawCmd::ResetRenderState => (), // TODO
                    DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        callback(draw_list.raw(), raw_cmd)
                    },
                }
            }
        }
        builder.end_render_pass(Default::default())?;
        imgui_render_span.exit();

        Ok(())
    }
}
