#version 460 core

in vec2 texcoord;

uniform sampler2D text;

out vec4 color;

void main() {
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(text, texcoord).a);
    color = vec4(1.0, 1.0, 1.0, 1.0) * sampled;
}