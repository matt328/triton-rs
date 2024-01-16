use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

#[repr(C)]
#[derive(Clone, Copy, BufferContents, Vertex)]
pub struct VertexPositionColorNormal {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    color: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    normal: [f32; 3],
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 color;
            layout(location = 2) in vec3 normal;

            layout(location = 0) out vec3 out_color;
            layout(location = 1) out vec3 out_normal;

            void main() {
                out_color = color;
                out_normal = normal;
                gl_Position = vec4(position, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec3 in_color;
            layout(location = 1) in vec3 in_normal;

            layout(location = 0) out vec4 f_color;
            layout(location = 1) out vec4 f_normal;

            void main() {
                f_color = vec4(in_color, 1.0);
                f_normal = vec4(in_normal, 0.0);
            }
        ",
    }
}

pub const CUBE_VERTICES: [VertexPositionColorNormal; 24] = [
    // Front face
    VertexPositionColorNormal {
        position: [-1.0, -1.0, 1.0],
        color: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    },
    VertexPositionColorNormal {
        position: [1.0, -1.0, 1.0],
        color: [1.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, 1.0],
        color: [1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, 1.0, 1.0],
        color: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    },
    // Right face
    VertexPositionColorNormal {
        position: [1.0, -1.0, 1.0],
        color: [1.0, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, -1.0, -1.0],
        color: [1.0, 0.0, 1.0],
        normal: [1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, -1.0],
        color: [1.0, 1.0, 1.0],
        normal: [1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, 1.0],
        color: [1.0, 1.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    },
    // Back face
    VertexPositionColorNormal {
        position: [-1.0, -1.0, -1.0],
        color: [0.0, 0.0, 1.0],
        normal: [0.0, 0.0, -1.0],
    },
    VertexPositionColorNormal {
        position: [1.0, -1.0, -1.0],
        color: [1.0, 0.0, 1.0],
        normal: [0.0, 0.0, -1.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, -1.0],
        color: [1.0, 1.0, 1.0],
        normal: [0.0, 0.0, -1.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, 1.0, -1.0],
        color: [0.0, 1.0, 1.0],
        normal: [0.0, 0.0, -1.0],
    },
    // Left face
    VertexPositionColorNormal {
        position: [-1.0, -1.0, 1.0],
        color: [0.0, 0.0, 0.0],
        normal: [-1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, -1.0, -1.0],
        color: [0.0, 0.0, 1.0],
        normal: [-1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, 1.0, -1.0],
        color: [0.0, 1.0, 1.0],
        normal: [-1.0, 0.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, 1.0, 1.0],
        color: [0.0, 1.0, 0.0],
        normal: [-1.0, 0.0, 0.0],
    },
    // Top face
    VertexPositionColorNormal {
        position: [-1.0, 1.0, 1.0],
        color: [0.0, 1.0, 0.0],
        normal: [0.0, 1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, 1.0],
        color: [1.0, 1.0, 0.0],
        normal: [0.0, 1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, 1.0, -1.0],
        color: [1.0, 1.0, 1.0],
        normal: [0.0, 1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, 1.0, -1.0],
        color: [0.0, 1.0, 1.0],
        normal: [0.0, 1.0, 0.0],
    },
    // Bottom face
    VertexPositionColorNormal {
        position: [-1.0, -1.0, 1.0],
        color: [0.0, 0.0, 0.0],
        normal: [0.0, -1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, -1.0, 1.0],
        color: [1.0, 0.0, 0.0],
        normal: [0.0, -1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [1.0, -1.0, -1.0],
        color: [1.0, 0.0, 1.0],
        normal: [0.0, -1.0, 0.0],
    },
    VertexPositionColorNormal {
        position: [-1.0, -1.0, -1.0],
        color: [0.0, 0.0, 1.0],
        normal: [0.0, -1.0, 0.0],
    },
];

pub const CUBE_INDICES: [u16; 36] = [
    // Front face
    0, 1, 2, 2, 3, 0, // Right face
    4, 5, 6, 6, 7, 4, // Back face
    8, 9, 10, 10, 11, 8, // Left face
    12, 13, 14, 14, 15, 12, // Top face
    16, 17, 18, 18, 19, 16, // Bottom face
    20, 21, 22, 22, 23, 20,
];
