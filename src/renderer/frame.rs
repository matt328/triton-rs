use std::sync::Arc;

use anyhow::Context;
use cgmath::Matrix4;
use vulkano::{
    command_buffer::{
        AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, SubpassBeginInfo, SubpassContents,
    },
    render_pass::Framebuffer,
    sync::GpuFuture,
};

use crate::FrameSystem;

use super::pass::{DrawPass, LightingPass, Pass};

pub struct Frame<'a> {
    pub system: &'a mut FrameSystem,
    num_pass: u8,
    pub framebuffer: Arc<Framebuffer>,
    before_main_cb_future: Option<Box<dyn GpuFuture>>,
    pub command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    pub world_to_framebuffer: Matrix4<f32>,
}

impl<'a> Frame<'a> {
    pub fn new(
        system: &'a mut FrameSystem,
        framebuffer: Arc<Framebuffer>,
        before_main_cb_future: Option<Box<dyn GpuFuture>>,
        command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
        world_to_framebuffer: Matrix4<f32>,
    ) -> Self {
        Frame {
            system,
            num_pass: 0,
            framebuffer,
            before_main_cb_future,
            command_buffer_builder,
            world_to_framebuffer,
        }
    }

    pub fn next_pass<'f>(&'f mut self) -> anyhow::Result<Option<Pass<'f, 'a>>> {
        let ret = match {
            let current_pass = self.num_pass;
            self.num_pass += 1;
            current_pass
        } {
            0 => Some(Pass::Deferred(DrawPass { frame: self })),

            1 => {
                self.command_buffer_builder
                    .as_mut()
                    .context("command buffer builder")?
                    .next_subpass(
                        Default::default(),
                        SubpassBeginInfo {
                            contents: SubpassContents::SecondaryCommandBuffers,
                            ..Default::default()
                        },
                    )
                    .context("advancing to next subpass")?;
                Some(Pass::Lighting(LightingPass { frame: self }))
            }

            2 => {
                self.command_buffer_builder
                    .as_mut()
                    .context("getting command buffer builder")?
                    .end_render_pass(Default::default())
                    .context("ending render pass")?;

                let command_buffer = self
                    .command_buffer_builder
                    .take()
                    .context("take command buffer builder")?
                    .build()
                    .context("build")?;

                let after_main_cb = self
                    .before_main_cb_future
                    .take()
                    .context("taking before main cb future")?
                    .then_execute(self.system.gfx_queue.clone(), command_buffer)
                    .context("executing primary command buffer")?;

                Some(Pass::Finished(Box::new(after_main_cb)))
            }

            _ => None,
        };

        Ok(ret)
    }
}
