#version 460 core
in vec2 texcoord;

uniform sampler2D textAtlas;

uniform vec4 textColor;

out vec4 color;

void main() {
  vec4 sampled = vec4(1.0, 1.0, 1.0, texture(textAtlas, texcoord).a);
  color = vec4(0.0, 1.0, 0.0, 1.0) * sampled;
}