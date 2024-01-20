#version 450

// The `color_input` parameter of the `draw` method.
layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput u_diffuse;

layout(push_constant) uniform PushConstants {
    // The `ambient_color` parameter of the `draw` method.
    vec4 color;
} push_constants;

layout(location = 0) out vec4 f_color;

void main() {
    // Load the value at the current pixel.
    vec3 in_diffuse = subpassLoad(u_diffuse).rgb;
    f_color.rgb = push_constants.color.rgb * in_diffuse;
    f_color.a = 1.0;
}
