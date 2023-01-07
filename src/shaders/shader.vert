#version 460 core

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_offset;
// layout(location = 2) in uint a_material;

uniform Matrix { mat4 mvp; };

void main() { gl_Position = mvp * (vec4(a_position + a_offset, 1.0)); }