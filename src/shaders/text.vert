#version 460 core

layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

layout(std140) uniform Matrices {
    mat4 projection;
} matrices;

out vec2 texcoord;

void main() {
    gl_Position = matrices.projection * vec4(a_position, 0.0, 1.0);
    texcoord = a_texcoord; 
}