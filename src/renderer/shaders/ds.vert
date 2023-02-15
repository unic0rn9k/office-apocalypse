#version 460 core

const uint MAX_CHUNKS = 170;

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;

layout(location = 2) in vec3 a_offset;
layout(location = 3) in uint a_chunkId;
layout(location = 4) in uint a_materialId;

struct Chunk {
  mat4 modelMatrix;
  mat4 mvpMatrix;
};

layout(std140, binding = 0) uniform Chunks { Chunk chunks[MAX_CHUNKS]; };

out vec4 fragPosition;
out vec4 normal;
out uint materialId;

void main() {
  vec4 position = vec4(a_position + a_offset, 1.0);

  gl_Position = chunks[a_chunkId].mvpMatrix * position;

  fragPosition = chunks[a_chunkId].modelMatrix * position;
  normal = vec4(a_normal, 0.0);
  materialId = a_materialId;
}