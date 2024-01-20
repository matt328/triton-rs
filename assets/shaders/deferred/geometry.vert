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

    out_normal = normalize(model_matrix * vec4(normal, 0.0));
    gl_Position = frame_data.proj * model_view * vec4(position, 1.0);
}
