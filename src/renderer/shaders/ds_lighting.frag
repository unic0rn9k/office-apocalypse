#version 460 core

const uint MAX_LIGHTS = 256;
const uint MAX_MATERIALS = 256;

in vec2 texcoord;

uniform sampler2D gFragPosition;
uniform sampler2D gNormal; 
uniform sampler2D gAlbedo;
uniform sampler2D gRoughnessAndMetalness;

struct Light {
    vec3 position;
    vec3 color;
};

layout(std140, binding = 0) uniform Lights { Light lights[MAX_LIGHTS]; };

out vec4 color;

void main() {
    vec4 fragPos = texture(gFragPosition, texcoord);
    vec4 normal  =  texture(gNormal, texcoord);
    vec4 albedo  = texture(gAlbedo, texcoord);
    vec2 RoughnessAndMetalness = texture(gRoughnessAndMetalness, texcoord).xy;

    color = albedo;
}