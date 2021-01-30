#version 450
precision highp float;
precision highp sampler2D;

uniform sampler2D texWarp;
uniform sampler2D texScale;

// uColumnIndex should be normalized [0,1) based on the actual width of texScale
uniform float uColumnIndex;

uniform float uzScale;
uniform float uOffset;

layout (std140) uniform uCameraMatrix {
  mat4 uView;
  mat4 uTransform;
  mat4 uProjection;
};

in vec3 vertPos;
in vec2 texPos;
out vec2 fragTexPos;

float x, y, s, wv, sv;

float fetchValue(in sampler2D tex, in float index) {
  return texture(tex, vec2(index, 0.0)).r;
}

void main() {

  x = vertPos.x;
  y = vertPos.y;

  float warpIndex = abs(y);
  float scaleIndex = mod(uColumnIndex - abs(x), 1.0);

  float ss = 1.0;// - abs(x) / 2.0;

  sv = ss * fetchValue(texScale, scaleIndex);
  wv = fetchValue(texWarp, warpIndex);
  // sv = wv + 0.000001 * sv;

  float elev = (wv + sv);

  // wtf why +/-1.1? (<- adds cool overlapping effect, but should parameterize)
  float os = 1.0 + uOffset;

  if (x <= 0.0) {
    x = pow(x + os, wv) - 1.0;
  } else {
    x = 1.0 - pow(abs(x - os), wv);
  }

  if (y <= 0.0) {
    s = (1. + y/2.) * sv;
    y = pow(y + 1.0, s) - 1.0;
  } else {
    s = (1. - y/2.) * sv;
    y = 1.0 - pow(abs(y - 1.0), s);
  }

  // float z = elev * vertPos.z;
  const float z = 1.0;

  fragTexPos = abs(2.0 * texPos - 1.0);

  x = mix(x, elev * x, uzScale);
  y = mix(y, elev * y, uzScale);

  // vec4 pos = vec4(elev * x, elev * y, 1.0, 1.0);
  vec4 pos = vec4(x, y, z, 1.0);

  gl_Position = uProjection * uTransform * uView * pos;
}