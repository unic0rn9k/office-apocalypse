#version 460 core

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_offset;
layout(location = 2) in uint a_materialId;

layout(std140, binding = 0) uniform Matrices {
  mat4 m;
  mat4 mvp;
}
matrices;

out uint materialId;

void main() {
  gl_Position = matrices.mvp * (vec4(a_position + a_offset, 1.0));
  materialId = a_materialId;
}