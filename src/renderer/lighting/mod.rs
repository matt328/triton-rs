pub use ambient::Ambient;
pub use directional::Directional;
pub use point::Point;

mod ambient;
mod directional;
mod point;

use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

#[derive(BufferContents, Vertex)]
#[repr(C)]
struct LightingVertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}
