#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 outColor;

layout(set = 0, binding = 0) uniform FrameData {
  mat4 view;
  mat4 proj;
}
frameData;

struct ObjectData {
  mat4 model;
};

// all object matrices
layout(std140, set = 1, binding = 0) readonly buffer ObjectBuffer {
  ObjectData objects[];
}
objectBuffer;

void main() {
  mat4 modelMatrix = objectBuffer.objects[gl_BaseInstance].model;
  mat4 modelview = frameData.view * modelMatrix;
  outColor = color;
  gl_Position = frameData.proj * modelview * vec4(position, 1.0);
}
