#version 460 core

const uint MAX_MATERIALS = 256;

in vec4 fragPosition;
in vec4 normal;
in flat uint materialId;  // used for indexing into materials

struct Material {
    vec4 albedo;
    float roughness;
    float metallic;
};

layout(std140, binding = 1) uniform Materials { Material materials[MAX_MATERIALS]; };

layout(location = 0) out vec4 gPosition;
layout(location = 1) out vec4 gNormal;
layout(location = 2) out vec4 gAlbedo;
layout(location = 3) out vec2 gRoughnessAndMetallic;

void main() {
    gPosition = fragPosition;
    gNormal = normalize(normal);
    gAlbedo = materials[materialId].albedo;
    gRoughnessAndMetallic.x = materials[materialId].roughness;
    gRoughnessAndMetallic.y = materials[materialId].metallic;
}