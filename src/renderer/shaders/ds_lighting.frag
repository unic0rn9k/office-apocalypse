#version 460 core

const float PI = 3.14159265359;

const uint MAX_LIGHTS = 256;
const uint MAX_MATERIALS = 256;

in vec2 texcoord;

uniform sampler2D gWorldPosition;
uniform sampler2D gNormal; 
uniform sampler2D gAlbedo;
uniform sampler2D gRoughnessAndMetallic;

struct Light {
    vec4 position;
    vec4 color;
};

layout(std140, binding = 0) uniform Lights { Light lights[MAX_LIGHTS]; };

layout(std140, binding = 1) uniform Camera {
    vec4 position;
} camera;

out vec4 color;

void main() {
    vec3 worldPosition = texture(gWorldPosition, texcoord).xyz;
    vec3 normal  =  texture(gNormal, texcoord).xyz;
    vec3 albedo  = texture(gAlbedo, texcoord).xyz;

    vec2 roughnessAndMetallic = texture(gRoughnessAndMetallic, texcoord).xy;
    float roughness = roughnessAndMetallic.x;
    float metallic = roughnessAndMetallic.y;

    color = vec4(albedo, 1.0);
}