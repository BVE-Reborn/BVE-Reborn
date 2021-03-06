#version 450

#include "gamma.glsl"

layout (local_size_x = 8, local_size_y = 8) in;

layout (set = 0, binding = 0, rgba8) uniform image2D inTexture;
layout (set = 0, binding = 1, rgba8) uniform image2D outTexture;
layout (set = 0, binding = 2) uniform Locals {
    uvec2 texture_dimensions;
};

vec4 load_gamma(ivec2 position) {
    return srgb_to_linear(imageLoad(inTexture, position));
}

void store_gamma(ivec2 location, vec4 value) {
    imageStore(outTexture, location, linear_to_srgb(value));
}

bool ivec2_lt(ivec2 lhs, ivec2 rhs) {
    return lhs.x < rhs.x && lhs.y < rhs.y;
}

bool ivec2_le(ivec2 lhs, ivec2 rhs) {
    return lhs.x <= rhs.x && lhs.y <= rhs.y;
}

vec4 load_strip_blue(ivec2 location) {
    vec4 value = load_gamma(location);
    if (value.xyz == vec3(0.0, 0.0, 1.0)) {
        value = vec4(0.0);
    }
    return value;
}

vec4 conditional_load(ivec2 location) {
    if (ivec2_le(ivec2(0), location) && ivec2_lt(location, ivec2(texture_dimensions))) {
        vec4 value = load_strip_blue(location);
        if (value.w == 0.0) {
            value = vec4(0.0);
        } else {
            value.w = 1.0;
        }
        return value;
    } else {
        return vec4(0.0);
    }
}

void main() {
    ivec2 location = ivec2(gl_GlobalInvocationID.xy);
    if (!(location.x < texture_dimensions.x && location.y < texture_dimensions.y)) {
        return;
    }
    vec4 texel11 = load_strip_blue(location);
    if (texel11.w == 0.0) {
        vec4 texel00 = conditional_load(location + ivec2(-1, -1));
        vec4 texel10 = conditional_load(location + ivec2(0, -1));
        vec4 texel20 = conditional_load(location + ivec2(1, -1));
        vec4 texel01 = conditional_load(location + ivec2(-1, 0));
        vec4 texel21 = conditional_load(location + ivec2(1, 0));
        vec4 texel02 = conditional_load(location + ivec2(-1, 1));
        vec4 texel12 = conditional_load(location + ivec2(0, 1));
        vec4 texel22 = conditional_load(location + ivec2(1, 1));
        
        vec4 sum = texel00 + texel01 + texel02 + texel10 + texel12 + texel20 + texel21 + texel22;
        float scale = sum.w;
        vec3 average = sum.xyz / scale;
        store_gamma(location, vec4(average, 0.0));
    } else {
        store_gamma(location, texel11);
    }
}