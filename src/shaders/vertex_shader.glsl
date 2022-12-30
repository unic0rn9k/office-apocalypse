#version 450 core

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_texcoord;

uniform Matrix { mat4 vp; };

out vec2 texcoord;

void main() {
  gl_Position = vp * vec4(a_position, 1.0);
  texcoord = a_texcoord;
}