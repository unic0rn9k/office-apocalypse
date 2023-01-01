#version 450 core

layout(std140) uniform Camera { vec3 camera_position; };

uniform sampler2D texture_sampler;

in vec3 normal;
in vec2 texcoord;
in vec3 world;

out vec4 color;

void main() { color = texture(texture_sampler, texcoord); }