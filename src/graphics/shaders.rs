use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct Position {
    #[format(R32G32_SFLOAT)]
    pub position: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, BufferContents, Vertex)]
pub struct VertexPositionColor {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    color: [f32; 3],
}

pub const VERTICES: [VertexPositionColor; 8] = [
    VertexPositionColor {
        position: [-1.0, -1.0, -1.0],
        color: [0.0, 0.0, 0.0],
    },
    VertexPositionColor {
        position: [1.0, -1.0, -1.0],
        color: [1.0, 0.0, 0.0],
    },
    VertexPositionColor {
        position: [1.0, 1.0, -1.0],
        color: [1.0, 1.0, 0.0],
    },
    VertexPositionColor {
        position: [-1.0, 1.0, -1.0],
        color: [0.0, 1.0, 0.0],
    },
    VertexPositionColor {
        position: [-1.0, -1.0, 1.0],
        color: [0.0, 0.0, 1.0],
    },
    VertexPositionColor {
        position: [1.0, -1.0, 1.0],
        color: [1.0, 0.0, 1.0],
    },
    VertexPositionColor {
        position: [1.0, 1.0, 1.0],
        color: [1.0, 1.0, 1.0],
    },
    VertexPositionColor {
        position: [-1.0, 1.0, 1.0],
        color: [0.0, 1.0, 1.0],
    },
];

pub const INDICES: [u16; 36] = [
    0, 1, 2, 2, 3, 0, 1, 5, 6, 6, 2, 1, 7, 6, 5, 5, 4, 7, 4, 0, 3, 3, 7, 4, 4, 5, 1, 1, 0, 4, 3, 2,
    6, 6, 7, 3,
];

pub mod vs_position_color {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "assets/shaders/basic/vert.glsl"
    }
}

pub mod fs_basic {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "assets/shaders/basic/frag.glsl"
    }
}

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec2 position;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}
