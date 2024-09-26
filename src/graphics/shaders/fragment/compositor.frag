#version 460

layout(location = 0) in vec4 overlayFragment;
layout(location = 0) out vec4 outColor;

void main() {
    // outColor = overlayFragment;
    outColor = vec4(0.5, 0.5, 0.5, 1.0);
  }
