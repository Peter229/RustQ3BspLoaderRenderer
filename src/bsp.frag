#version 450

layout(location = 0) in vec4 v_colour;
layout(location = 1) in vec2 v_tex_coords;
layout(location = 2) in vec2 v_tex_coords_lightmap;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

layout(set = 1, binding = 0) uniform texture2D l_t_diffuse;
layout(set = 1, binding = 1) uniform sampler l_s_diffuse;

void main() {
    f_color = texture(sampler2D(l_t_diffuse, l_s_diffuse), v_tex_coords_lightmap) * texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords) * v_colour * 70;
}