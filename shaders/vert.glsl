#version 330 core
layout(location=0) in vec2 position;
layout(location=1) in vec2 dimension;

uniform vec2 projection;
uniform vec2 offset;

out VS_OUT {
    vec2 dimension;
} vs_out;

void main() {
    vs_out.dimension = 2 * dimension / projection;
    gl_Position = vec4((2 * (position + offset) - projection) / projection, 0, 1);
}
