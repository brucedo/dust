#version 460

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput overlayFragment;
layout(location = 0) out vec4 outColor;

void main() {
  outColor = subpassLoad(overlayFragment);
    // outColor = vec4(0.5, 0.5, 0.5, 1.0);
}
