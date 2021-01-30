#version 450
precision mediump float;

layout(location = 0) in vec2 pos;

void main() {
  vec2 p = 2. * pos + 1.;
  gl_Position = vec4(p, 0., 1.);
}
