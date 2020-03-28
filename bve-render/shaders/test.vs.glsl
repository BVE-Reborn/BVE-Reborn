#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 texcoord;

layout(set = 0, binding = 0) uniform Locals {
    mat4 transform;
};

void main() {
    gl_Position = transform * vec4(position, 1.0);
}
