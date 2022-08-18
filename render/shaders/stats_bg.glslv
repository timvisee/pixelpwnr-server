#version 150 core

in vec2 a_Pos;
out vec4 v_Color;

void main() {
    v_Color = vec4(1.0, 1.0, 1.0, 0.8);
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}
