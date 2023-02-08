#version 460 core
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

uniform float scale;

uniform Matrices { mat4 ortho; };

out vec2 texcoord;

void main() {
  gl_Position = ortho * vec4(a_position * scale, 0.0, 1.0);
  texcoord = a_texcoord;
}