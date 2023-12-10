use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
};

use super::shaders::VertexPositionColor;

#[derive(Default)]
pub struct MeshBuilder {
    vertices: Option<Vec<VertexPositionColor>>,
    indices: Option<Vec<u16>>,
}

impl MeshBuilder {
    pub fn with_vertices(mut self, value: Vec<VertexPositionColor>) -> Self {
        self.vertices = Some(value);
        self
    }

    pub fn with_indices(mut self, value: Vec<u16>) -> Self {
        self.indices = Some(value);
        self
    }

    pub fn build(self, memory_allocator: Arc<dyn MemoryAllocator>) -> anyhow::Result<BasicMesh> {
        let vertices = self.vertices.unwrap_or_default();

        let vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
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

        let indices = self.indices.unwrap_or_default();
        let index_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices,
        )
        .context("creating index buffer")?;

        Ok(BasicMesh {
            vertex_buffer,
            index_buffer,
        })
    }
}

pub struct BasicMesh {
    pub vertex_buffer: Subbuffer<[VertexPositionColor]>,
    pub index_buffer: Subbuffer<[u16]>,
}
