#version 150 core

uniform sampler2D t_Image;

in vec4 v_Color;
in vec2 v_Uv;
out vec4 Target0;

void main() {
    // Extract pixel data
    vec4 pixel = texture(t_Image, v_Uv);

    // Draw the image pixels
    Target0 = pixel;
}
