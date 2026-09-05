#version 440

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec4 glyphColor;
} ubuf;

layout(binding = 1) uniform sampler2D source;

void main()
{
    vec4 pixel = texture(source, qt_TexCoord0);
    float luminance = dot(pixel.rgb, vec3(0.2126, 0.7152, 0.0722));
    float mask = pixel.a * (1.0 - smoothstep(0.28, 0.72, luminance));
    vec4 color = vec4(ubuf.glyphColor.rgb * ubuf.glyphColor.a, ubuf.glyphColor.a);
    fragColor = color * mask * ubuf.qt_Opacity;
}
