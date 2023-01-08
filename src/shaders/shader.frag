#version 460 core

in flat uint materialId;

struct Material {
    vec4 albedo;
    float roughness;
    float metalness;
};

layout(std140, binding = 1) uniform Materials {
    Material materials[256];
};

out vec4 color;

void main() {
    color = materials[materialId].albedo;
}