mod shader;

use anyhow::Context;
use vulkano::buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo};
use vulkano::buffer::BufferContents;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::image::sampler::Sampler;
use vulkano::image::ImageCreateInfo;
use vulkano::memory::allocator::{FreeListAllocator, GenericMemoryAllocator, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Scissor, Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::{
    buffer::BufferUsage,
    command_buffer::{PrimaryAutoCommandBuffer, SubpassContents},
    image::view::ImageView,
    render_pass::RenderPass,
};

use vulkano::format::{ClearValue, Format};
use vulkano::render_pass::Framebuffer;
use vulkano::render_pass::Subpass;

use std::fmt;
use std::sync::Arc;

use imgui::{internal::RawWrapper, DrawCmd, DrawCmdParams, DrawVert, TextureId, Textures};

#[derive(Default, Debug, Clone, BufferContents, Vertex, Copy)]
#[repr(C)]
struct ImGuiVertex {
    #[format(R32G32_SFLOAT)]
    pub pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    #[format(R32_SFLOAT)]
    pub col: u32,
}

impl From<DrawVert> for ImGuiVertex {
    fn from(v: DrawVert) -> ImGuiVertex {
        unsafe { std::mem::transmute(v) }
    }
}

#[derive(Debug)]
pub enum RendererError {
    BadTexture(TextureId),
    BadImageDimensions(ImageCreateInfo),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &Self::BadTexture(ref t) => {
                write!(f, "The Texture ID could not be found: {:?}", t)
            }
            &Self::BadImageDimensions(d) => {
                write!(f, "Image Dimensions not supported (must be Dim2d): {:?}", d)
            }
        }
    }
}

impl std::error::Error for RendererError {}

pub type Texture = (Arc<ImageView>, Arc<Sampler>);

pub struct Renderer {
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    font_texture: Texture,
    textures: Textures<Texture>,
    vertex_buffer_allocator: SubbufferAllocator,
    index_buffer_allocator: SubbufferAllocator,
}

impl Renderer {
    /// Initialize the renderer object, including vertex buffers, ImGui font textures,
    /// and the Vulkan graphics pipeline.
    ///
    /// ---
    ///
    /// `ctx`: the ImGui `Context` object
    ///
    /// `device`: the Vulkano `Device` object for the device you want to render the UI on.
    ///
    /// `queue`: the Vulkano `Queue` object for the queue the font atlas texture will be created on.
    ///
    /// `format`: the Vulkano `Format` that the render pass will use when storing the frame in the target image.
    pub fn init(
        ctx: &mut imgui::Context,
        device: Arc<Device>,
        queue: Arc<Queue>,
        format: Format,
        memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    ) -> Result<Renderer, Box<dyn std::error::Error>> {
        let vs = shader::vs::load(device.clone()).unwrap();
        let fs = shader::fs::load(device.clone()).unwrap();
        let vs_entry_point = vs.entry_point("main").context("getting vs entry point")?;
        let fs_entry_point = fs.entry_point("main").context("getting fs entry point")?;

        let render_pass = vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    format: format,
                    samples: 1,
                    load_op: Load,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .context("Creating RenderPass")?;

        let subpass = Subpass::from(render_pass.clone(), 0).context("creating imgui subpass")?;

        let vertex_input_state = ImGuiVertex::per_vertex()
            .definition(&vs_entry_point.info().input_interface)
            .context("creating vertex input state")?;

        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry_point),
            PipelineShaderStageCreateInfo::new(fs_entry_point),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .context("creating pipeline layout info")?,
        )?;

        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    ..Default::default()
                }),
                dynamic_state: [DynamicState::Viewport, DynamicState::Scissor]
                    .into_iter()
                    .collect(),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .context("Creating Pipeline")?;

        let textures = Textures::new();

        let font_texture = Self::upload_font_texture(ctx.fonts(), device.clone(), queue.clone())?;

        ctx.set_renderer_name(Some(format!(
            "imgui-vulkano-renderer {}",
            env!("CARGO_PKG_VERSION")
        )));

        let vertex_buffer_allocator = SubbufferAllocator::new(
            memory_allocator,
            SubbufferAllocatorCreateInfo {
                // We want to use the allocated subbuffers as vertex buffers.
                buffer_usage: BufferUsage::VERTEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let index_buffer_allocator = SubbufferAllocator::new(
            memory_allocator,
            SubbufferAllocatorCreateInfo {
                // We want to use the allocated subbuffers as vertex buffers.
                buffer_usage: BufferUsage::INDEX_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        Ok(Renderer {
            render_pass,
            pipeline: pipeline as Arc<GraphicsPipeline>,
            font_texture,
            textures,
            vertex_buffer_allocator,
            index_buffer_allocator,
        })
    }

    /// Appends the draw commands for the UI frame to an `AutoCommandBufferBuilder`.
    ///
    /// ---
    ///
    /// `cmd_buf_builder`: An `AutoCommandBufferBuilder` from vulkano to add commands to
    ///
    /// `device`: the Vulkano `Device` object for the device you want to render the UI on
    ///
    /// `queue`: the Vulkano `Queue` object for buffer creation
    ///
    /// `target`: the target image to render to
    ///
    /// `draw_data`: the ImGui `DrawData` that each UI frame creates
    pub fn draw_commands(
        &mut self,
        cmd_buf_builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        _queue: Arc<Queue>,
        target: ImageView,
        draw_data: &imgui::DrawData,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

        let x = target.image().extent();
        let vp = Viewport {
            extent: [x[0] as f32, x[1] as f32],
            offset: [0.0, 0.0],
            depth_range: 0.0..=1.0,
        };

        let mut dynamic_state = DynamicState::default();
        dynamic_state.viewports = Some(vec![vp]);
        dynamic_state.scissors = Some(vec![Scissor::default()]);

        let clip_off = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        let layout = self.pipeline.descriptor_set_layout(0).unwrap();

        let framebuffer = Arc::new(
            Framebuffer::start(self.render_pass.clone())
                .add(target)?
                .build()?,
        );

        cmd_buf_builder.begin_render_pass(
            framebuffer,
            SubpassContents::Inline,
            vec![ClearValue::None],
        )?;

        for draw_list in draw_data.draw_lists() {
            let vertex_buffer = self
                .vertex_buffer_allocator
                .allocate_slice(draw_list.vtx_buffer().len() as u64)
                .unwrap();

            let data: Vec<ImGuiVertex> = draw_list
                .vtx_buffer()
                .iter()
                .map(|&v| ImGuiVertex::from(v))
                .collect();

            vertex_buffer
                .write()
                .unwrap()
                .copy_from_slice(data.as_slice());

            let indices = draw_list.idx_buffer();

            let index_buffer = self
                .index_buffer_allocator
                .allocate_slice(indices.len() as u64)
                .unwrap();

            index_buffer.write().unwrap().copy_from_slice(indices);

            for cmd in draw_list.commands() {
                match cmd {
                    DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                clip_rect,
                                texture_id,
                                // vtx_offset,
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
                            if let Some(ref mut scissors) = dynamic_state.scissors {
                                scissors[0] = Scissor {
                                    offset: [
                                        f32::max(0.0, clip_rect[0]).floor() as u32,
                                        f32::max(0.0, clip_rect[1]).floor() as u32,
                                    ],
                                    extent: [
                                        (clip_rect[2] - clip_rect[0]).abs().ceil() as u32,
                                        (clip_rect[3] - clip_rect[1]).abs().ceil() as u32,
                                    ],
                                };
                            }

                            let tex = self.lookup_texture(texture_id)?;

                            let set = Arc::new(
                                PersistentDescriptorSet::start(layout.clone())
                                    .add_sampled_image(tex.0.clone(), tex.1.clone())?
                                    .build()?,
                            );

                            cmd_buf_builder.draw_indexed(
                                self.pipeline.clone(),
                                &dynamic_state,
                                vec![vertex_buffer.clone()],
                                index_buffer
                                    .clone()
                                    .into_buffer_slice()
                                    .slice(idx_offset..(idx_offset + count))
                                    .unwrap(),
                                set,
                                pc,
                                vec![],
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
        cmd_buf_builder.end_render_pass()?;

        Ok(())
    }

    /// Update the ImGui font atlas texture.
    ///
    /// ---
    ///
    /// `ctx`: the ImGui `Context` object
    ///
    /// `device`: the Vulkano `Device` object for the device you want to render the UI on.
    ///
    /// `queue`: the Vulkano `Queue` object for the queue the font atlas texture will be created on.
    pub fn reload_font_texture(
        &mut self,
        ctx: &mut imgui::Context,
        device: Arc<Device>,
        queue: Arc<Queue>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.font_texture = Self::upload_font_texture(ctx.fonts(), device, queue)?;
        Ok(())
    }

    /// Get the texture library that the renderer uses
    pub fn textures(&mut self) -> &mut Textures<Texture> {
        &mut self.textures
    }

    fn upload_font_texture(
        mut fonts: imgui::FontAtlasRefMut,
        device: Arc<Device>,
        queue: Arc<Queue>,
    ) -> Result<Texture, Box<dyn std::error::Error>> {
        let texture = fonts.build_rgba32_texture();

        let (image, fut) = ImmutableImage::from_iter(
            texture.data.iter().cloned(),
            ImageDimensions::Dim2d {
                width: texture.width,
                height: texture.height,
                array_layers: 1,
            },
            vulkano::image::MipmapsCount::One,
            Format::R8G8B8A8Srgb,
            queue.clone(),
        )?;

        fut.then_signal_fence_and_flush()?.wait(None)?;

        let sampler = Sampler::simple_repeat_linear(device.clone());

        fonts.tex_id = TextureId::from(usize::MAX);
        Ok((ImageView::new(image)?, sampler))
    }

    fn lookup_texture(&self, texture_id: TextureId) -> Result<&Texture, RendererError> {
        if texture_id.id() == usize::MAX {
            Ok(&self.font_texture)
        } else if let Some(texture) = self.textures.get(texture_id) {
            Ok(texture)
        } else {
            Err(RendererError::BadTexture(texture_id))
        }
    }
}
