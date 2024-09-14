#version 460

layout(location = 0) in vec4 overlayFragment;
layout(location = 1) out vec4 outColor;

void main() {
    outColor = overlayFragment;
  }
