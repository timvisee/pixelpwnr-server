#version 150 core

in vec4 v_Color;
out vec4 Target0;

void main() {
    // Draw the color
    Target0 = v_Color;
}
