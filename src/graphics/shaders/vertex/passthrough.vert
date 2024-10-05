#version 460

// layout(location = 0) in vec3 inVertex;
layout(location = 0) out vec4 color;

vec2 positions[3] = vec2[] (
  vec2(0.0, -1.0), 
  vec2(1.0, 1.0), 
  vec2(-1.0, 1.0)
);

void main() {
  // gl_Position = vec4(inVertex, 1.0);
  gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
  color = vec4(1.0, 1.0, 1.0, 1.0);
  // gl_PointSize = 1.0;
}
