#version 330 core

layout (points) in;
layout (triangle_strip, max_vertices=4) out;

in VS_OUT {
    vec2 dimension;
} gs_in [];

void main() {
    vec4 x_offset = vec4(gs_in[0].dimension.x, 0, 0, 0);
    vec4 y_offset = vec4(0, gs_in[0].dimension.y, 0, 0);

    gl_Position = gl_in[0].gl_Position;
    EmitVertex();
    gl_Position = gl_in[0].gl_Position + y_offset;
    EmitVertex();
    gl_Position = gl_in[0].gl_Position + x_offset;
    EmitVertex();
    gl_Position = gl_in[0].gl_Position + x_offset + y_offset;
    EmitVertex();
}
