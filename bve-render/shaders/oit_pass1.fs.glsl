#version 450

layout(early_fragment_tests) in;

layout(location = 0) in vec2 texcoord;
layout(location = 1) in vec3 normal;
layout(location = 2) flat in vec4 mesh_color;
layout(location = 0) out vec4 outColor;

struct Node {
    vec4 color;
    float depth;
    uint next;
};

layout(set = 0, binding = 0) uniform utexture2D colorTexture;
layout(set = 0, binding = 1) uniform sampler main_sampler;
layout(set = 1, binding = 0, r32ui) uniform uimage2D head_pointers;
layout(set = 1, binding = 1) uniform OIT {
    uint max_nodes;
};
layout(set = 1, binding = 2) buffer NodeBuffer {
    uint next_index;
    Node nodes[];
};

void main() {
    vec4 tex_color = pow(vec4(texture(usampler2D(colorTexture, main_sampler), texcoord)) / 255, vec4(2.2));
    vec4 color = tex_color * mesh_color;
    vec4 out_color = vec4(pow(color.rgb, vec3(1 / 2.2)), color.a);

    uint node_idx = atomicAdd(next_index, 1);
    if (node_idx < max_nodes) {
        uint prev_head = imageAtomicExchange(head_pointers, ivec2(gl_FragCoord.xy), node_idx);

        nodes[node_idx].color = out_color;
        nodes[node_idx].depth = gl_FragCoord.z;
        nodes[node_idx].next = prev_head;
    }
}