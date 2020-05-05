#version 450

#include "opaque_signature.glsl"

void main() {
    uint z = compute_frustum().z;
    for (int x = 0; x < frustum_count.x; ++x) {
        for (int y = 0; y < frustum_count.y; ++y) {
            uint frustum_index = get_frustum_list_index(uvec2(x, y), frustum_count.xy);
            Frustum frustum = frustums[frustum_index];
            if (contains_point(frustum, vec3(view_position))) {
                out_color = vec4(vec3(x, y, z) / vec3(frustum_count.xyz - 1), 1.0);
                return;
            }
        }
    }
    out_color = vec4(0.0, 0.0, 1.0, 1.0);
}
