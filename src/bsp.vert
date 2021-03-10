#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;
layout(location = 2) in vec2 a_tex_coords_lightmap;
layout(location = 3) in vec3 a_normal;
layout(location = 4) in vec4 a_colour;

layout(location = 0) out vec4 v_colour;
layout(location = 1) out vec2 v_tex_coords;
layout(location = 2) out vec2 v_tex_coords_lightmap;

layout(set = 2, binding = 0)
uniform Uniforms {
    mat4 u_view_proj;
    mat4 model;
};

void main() {
    v_colour = a_colour;
    v_tex_coords = a_tex_coords;
    v_tex_coords_lightmap = a_tex_coords_lightmap;
    gl_Position = u_view_proj * model * vec4(a_position, 1.0);
}