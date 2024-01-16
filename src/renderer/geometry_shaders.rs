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

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 color;
            layout(location = 2) in vec3 normal;

            layout(location = 0) out vec3 out_color;
            layout(location = 1) out vec4 out_normal;

            layout(set = 0, binding = 0) uniform FrameData {
                mat4 view;
                mat4 proj;
            }
            frame_data;
            
            struct ObjectData {
                mat4 model;
            };

            layout(std140, set = 1, binding = 0) readonly buffer ObjectBuffer {
                ObjectData objects[];
            }
            object_buffer;

            void main() {
                out_color = color;

                mat4 model_matrix = object_buffer.objects[gl_BaseInstance].model;
                mat4 model_view = frame_data.view * model_matrix;

                out_normal = model_view * vec4(normal.xyz, 0.0);
                gl_Position = frame_data.proj * model_view * vec4(position, 1.0);
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) in vec3 in_color;
            layout(location = 1) in vec4 in_normal;

            layout(location = 0) out vec4 f_color;
            layout(location = 1) out vec4 f_normal;

            void main() {
                f_color = vec4(in_color, 1.0);
                f_normal = in_normal;
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
