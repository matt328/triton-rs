use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
};

use super::shaders::Position;

#[derive(Default)]
pub struct MeshBuilder {
    vertices: Option<Vec<Position>>,
}

impl MeshBuilder {
    pub fn with_vertices(mut self, value: Vec<Position>) -> Self {
        self.vertices = Some(value);
        self
    }

    pub fn build(self, memory_allocator: Arc<dyn MemoryAllocator>) -> anyhow::Result<BasicMesh> {
        let vertices = self.vertices.unwrap_or_default();

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

        Ok(BasicMesh { vertex_buffer })
    }
}

pub struct BasicMesh {
    pub vertex_buffer: Subbuffer<[Position]>,
}
