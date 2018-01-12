#version 150 core

uniform sampler2D t_Image;

in vec4 v_Color;
in vec2 v_Uv;
out vec4 Target0;

void main() {
    // Extract pixel data
    vec3 pixels = texture(t_Image, v_Uv).rgb;

    // Draw the image pixels
    Target0 = vec4(pixels, 1.0);
}
