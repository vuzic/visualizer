#version 450
#define PI 3.141592653589793

precision mediump float;

layout(set = 0, binding = 0) uniform sampler2D texHSLuv;

// .s is scale, .t is offset
layout(std140, set = 1, binding = 0) uniform ColorParams {
  uniform vec2 valueScale;
  uniform vec2 lightnessScale;
  uniform vec2 alphaScale;
  uniform float period;
  uniform float cycle;
  uniform vec3 gamma;
  uniform vec2 stateSize;
  uniform int columnIndex;
} uColorParams;

layout(set = 2, binding = 0) uniform sampler2D texAmplitudes;
layout(set = 2, binding = 1) uniform sampler2D texDrivers;

layout(location = 0) out vec4 fragColor;

float sigmoid(in float x) {
  return (1. + x / (1. + abs(x))) / 2.;
}

vec4 getHSLuv(in float amp, in float ph, in float phi) {
  vec2 vs = uColorParams.valueScale;
  vec2 ls = uColorParams.lightnessScale;
  vec2 as = uColorParams.alphaScale;

  float hue = (0.5 * (uColorParams.cycle * phi + ph) / PI);
  // texture can wrap so no mod
  // hue -= 0.5 * (sign(mod(hue, 1.)) - 1.);

  float val = ls.s * sigmoid(vs.s * amp + vs.t) + ls.t;
  float alpha = sigmoid(as.s * amp + as.t);

  vec3 color = texture(texHSLuv, vec2(hue, val)).rgb;
  color = pow(color, uColorParams.gamma);
  return vec4(color, alpha);
}

float getAmp(in ivec2 index) {
  index.x = uColorParams.columnIndex - index.x;
  if (index.x < 0) index.x += int(uColorParams.stateSize.x); 
  return texelFetch(texAmplitudes, ivec2(index.y, index.x), 0).r;
}

// .s = scale, .t = energy
vec2 getDrivers(in ivec2 index) {
  return texelFetch(texDrivers, index, 0).rg;
}

void main () {
  float x = gl_FragCoord.x;
  float ws = (2. * PI) / uColorParams.period;
  float phi = x * ws;

  float decay = x / uColorParams.stateSize.x;
  decay = 1. - decay * decay;

  ivec2 index = ivec2(gl_FragCoord.x, gl_FragCoord.y);
  float amp = getAmp(index);
  vec2 drivers = getDrivers(ivec2(gl_FragCoord.y, 0));

  amp = drivers.s * (amp - 1.);
  vec4 color = getHSLuv(amp, drivers.t, phi);
  fragColor = color * vec4(vec3(decay), 1.);
}