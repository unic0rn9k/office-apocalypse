#version 450 core
layout (location = 0) in vec3 position;

void vsmain() {
    gl_Position = vec4(position, 1.0);
}

void psmain() {

}