#version 460 core

const uint MAX_MATERIALS = 256;

in vec4 fragPosition;
in vec4 normal;
in flat uint materialId;

struct Material {
    vec4 albedo;
    float roughness;
    float metalness;
};

layout(std140, binding = 1) uniform Materials { Material materials[MAX_MATERIALS]; };

layout(location = 0) out vec4 gPosition;
layout(location = 1) out vec4 gNormal;
layout(location = 2) out vec4 gAlbedo;
layout(location = 3) out vec2 gRoughnessAndMetalness;

void main() {
    gPosition = fragPosition;
    gNormal = normalize(normal);
    gAlbedo = materials[materialId].albedo;
    gRoughnessAndMetalness = vec2(materials[materialId].roughness, materials[materialId].metalness);
}