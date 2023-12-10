#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 outColor;

layout(set = 0, binding = 0) uniform FrameData {
  mat4 model;
  mat4 view;
  mat4 proj;
}
uniforms;

void main() {
  mat4 modelview = uniforms.view * uniforms.model;
  outColor = color;
  gl_Position = uniforms.proj * modelview * vec4(position, 1.0);
}
