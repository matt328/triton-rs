use std::sync::Arc;

use anyhow::Context;
use cgmath::{Matrix4, SquareMatrix, Vector3};
use vulkano::{command_buffer::CommandBuffer, sync::GpuFuture};

use super::frame::Frame;

pub enum Pass<'f, 's: 'f> {
    Deferred(DrawPass<'f, 's>),
    Lighting(LightingPass<'f, 's>),
    Finished(Box<dyn GpuFuture>),
}

pub struct DrawPass<'f, 's: 'f> {
    pub frame: &'f mut Frame<'s>,
}

impl<'f, 's: 'f> DrawPass<'f, 's> {
    pub fn execute(&mut self, command_buffer: Arc<CommandBuffer>) -> anyhow::Result<()> {
        self.frame
            .command_buffer_builder
            .as_mut()
            .context("getting command buffer builder")?
            .execute_commands(command_buffer)?;
        Ok(())
    }

    pub fn viewport_dimensions(&self) -> [u32; 2] {
        self.frame.framebuffer.extent()
    }

    #[allow(dead_code)]
    pub fn world_to_framebuffer_matrix(&self) -> Matrix4<f32> {
        self.frame.world_to_framebuffer
    }
}

pub struct LightingPass<'f, 's: 'f> {
    pub frame: &'f mut Frame<'s>,
}

impl<'f, 's: 'f> LightingPass<'f, 's> {
    pub fn ambient_light(&mut self, color: [f32; 3]) -> anyhow::Result<()> {
        let command_buffer = self
            .frame
            .system
            .ambient_lighting_system
            .draw(
                self.frame.framebuffer.extent(),
                self.frame.system.diffuse_buffer.clone(),
                color,
            )
            .context("ambient lighting draw")?;
        self.frame
            .command_buffer_builder
            .as_mut()
            .context("getting command buffer builder")?
            .execute_commands(command_buffer)
            .context("executing commands")?;
        Ok(())
    }

    pub fn directional_light(
        &mut self,
        direction: Vector3<f32>,
        color: [f32; 3],
    ) -> anyhow::Result<()> {
        let command_buffer = self
            .frame
            .system
            .directional_lighting_system
            .draw(
                self.frame.framebuffer.extent(),
                self.frame.system.diffuse_buffer.clone(),
                self.frame.system.normals_buffer.clone(),
                direction,
                color,
            )
            .context("drawing directional lights")?;

        self.frame
            .command_buffer_builder
            .as_mut()
            .context("getting command buffer builder")?
            .execute_commands(command_buffer)
            .context("executing commands")?;
        Ok(())
    }

    pub fn point_light(&mut self, position: Vector3<f32>, color: [f32; 3]) -> anyhow::Result<()> {
        let command_buffer = {
            self.frame
                .system
                .point_lighting_system
                .draw(
                    self.frame.framebuffer.extent(),
                    self.frame.system.diffuse_buffer.clone(),
                    self.frame.system.normals_buffer.clone(),
                    self.frame.system.depth_buffer.clone(),
                    self.frame
                        .world_to_framebuffer
                        .invert()
                        .context("inverting matrix")?,
                    position,
                    color,
                )
                .context("drawing point lights")?
        };

        self.frame
            .command_buffer_builder
            .as_mut()
            .context("getting command buffer builder")?
            .execute_commands(command_buffer)
            .context("executing commands")?;
        Ok(())
    }
}
