use std::sync::Arc;

use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
    image::Image,
};

use super::render_data::RenderData;

pub trait Renderer {
    fn resize(&mut self, images: &[Arc<Image>]) -> anyhow::Result<()>;

    fn record_command_buffer(
        &self,
        frame_index: usize,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        render_data: &RenderData,
    ) -> anyhow::Result<()>;
}
