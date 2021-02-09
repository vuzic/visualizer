use std::sync::Mutex;

use image::{DynamicImage, ImageBuffer, Pixel, RgbImage, Rgba, RgbaImage};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::audiosys::AudioFeatures;


pub struct Visualizer {
    verbose: i32,
    image: (u32, u32),
}

lazy_static! {
    // static ref SIGMOID: Sigmoid = Sigmoid::new();
    static ref CLUT: Clut = Clut::new();
    static ref COUNT: Mutex<usize> = Mutex::new(0);
}

type F32Image = ImageBuffer<Rgba<f32>, Vec<f32>>;

impl Visualizer {
    pub fn new(w: u32, h: u32, verbose: i32) -> Self {
        Self {
            verbose,
            image: (w, h),
        }
    }

    pub fn render(&self, params: &Params, features: &AudioFeatures) -> RgbImage {
        let (bins, length) = features.get_size();

        let scales = features.get_scales();
        let energy = features.get_energy();
        let diff = features.get_diff();

        let mut color_buffer = F32Image::new(length as u32, bins as u32);

        let mut vt_warp = vec![1.; length];

        for i in 0..length {
            let amp = features.get_amplitudes(i);
            let mut s = 0.;
            for j in 0..bins {
                let val = scales[j] * (amp[j] - 1.0);
                color_buffer.put_pixel(
                    i as u32,
                    j as u32,
                    self.get_hsv(params, val as f32, energy[j] as f32, i as f32),
                );

                s += amp[j];
            }
            let warp = s as f32 / bins as f32;
            vt_warp[i] = params.vt_warp.0 * warp + params.vt_warp.1;
        }

        let mut hz_warp: Vec<f32> = diff
            .iter()
            .map(|&x| params.hz_warp.0 * x as f32 + params.hz_warp.1)
            .collect();
        for i in 1..hz_warp.len() - 1 {
            hz_warp[i] = (hz_warp[i - 1] + hz_warp[i] + hz_warp[i + 1]) / 3.;
        }

        let (w, h) = self.image;
        let (wf, hf) = (w as f32, h as f32);
        let (wo, ho) = (wf / 2., hf / 2.);
        let (lf, bf) = (length as f32, bins as f32);
        let mut image = F32Image::new(w, h);
        for i in 0..length {
            for j in 0..bins {
                let p = Point(i as f32 / lf, j as f32 / bf);
                let Point(x, y) = apply_warp(p, hz_warp[j], vt_warp[i]);

                let cpx = color_buffer.get_pixel(i as u32, j as u32);
                for (r, q) in &[(-1., -1.), (-1., 1.), (1., -1.), (1., 1.)] {
                    let (x, y) = (wo + r * x * wo, ho + q * y * ho);
                    let px =
                        image.get_pixel_mut((x as u32).clamp(0, w - 1), (y as u32).clamp(0, h - 1));
                    px.apply2(cpx, |a, b| a + b);
                }
            }
        }

        #[inline]
        fn to_u8(x: f32) -> u8 {
            (x * 255.5).clamp(0., 255.5) as u8
        }

        let out = RgbaImage::from_vec(
            w,
            h,
            image
                .pixels()
                .flat_map(|px| {
                    vec![
                        to_u8(px[1] * px[0]),
                        to_u8(px[2] * px[0]),
                        to_u8(px[3] * px[0]),
                        to_u8(px[0]),
                    ]
                    .into_iter()
                })
                .collect(),
        )
        .unwrap();

        let out = if params.blur > 0. {
            DynamicImage::ImageRgba8(out).blur(params.blur)
        } else {
            DynamicImage::ImageRgba8(out)
        }
        .to_rgb8();

        let mut count = COUNT.lock().unwrap();
        *count += 1;

        out
    }

    fn get_hsv(&self, params: &Params, val: f32, e: f32, phi: f32) -> Rgba<f32> {
        use std::f32::consts::PI;

        let vs = params.value_scale;
        let ls = params.lightness_scale;
        let als = params.alpha_scale;
        let cs = params.color_cycle_rate;
        let phi = 2.0 * PI * phi / params.color_period;

        let hue = 0.5 * (cs * e + phi) / PI;
        // let value = ls.0 * SIGMOID.f(vs.0 * val + vs.1) + ls.1;
        let value = ls.0 * sigmoid_fast(vs.0 * val + vs.1) + ls.1;
        // let alpha = params.max_alpha * SIGMOID.f(als.0 * val + als.1);
        let alpha = params.max_alpha * sigmoid_fast(als.0 * val + als.1);

        // if *COUNT.lock().unwrap() % 256 == 0 {
        //     println!("hue: {}, value: {}", hue, value);
        // }
        let color = CLUT.lookup(hue, value);
        Rgba([alpha, color.0, color.1, color.2])
    }
}

struct Point(f32, f32);

#[inline]
fn powf_fast(x: f32, y: f32) -> f32 {
    use fast_math::{exp2_raw, log2_raw};
    exp2_raw(y * log2_raw(x))
}

#[inline]
fn sigmoid_fast(x: f32) -> f32 {
    use fast_math::exp_raw;
    1. / (1. + exp_raw(-x))
}

fn apply_warp(xy: Point, w: f32, s: f32) -> Point {
    let Point(x, y) = xy;
    let x = if x <= 0. {
        powf_fast(x + 1., w) - 1.
    } else {
        1. - powf_fast(1. - x, w)
    };
    let y = if y <= 0. {
        let s = (1. + y / 2.) * s;
        powf_fast(1. + y, s) - 1.
    } else {
        let s = (1. - y / 2.) * s;
        1. - powf_fast(1. - y, s)
    };
    Point(x, y) //x.clamp(0., 1.), y.clamp(0., 1.))
}

// struct Sigmoid {
//     lut: Vec<f32>,
// }

// impl Sigmoid {
//     const SIZE: usize = 2048;
//     const RANGE: f32 = 10.0;
//     const SCALE: f32 = Self::SIZE as f32 / (2. * Self::RANGE);

//     fn new() -> Self {
//         let mut lut = vec![0.; Self::SIZE];
//         let hl = (Self::SIZE / 2) as f32;
//         for i in 0..Self::SIZE {
//             let x = (i as f32 - hl) / hl * Self::RANGE;
//             lut[i] = 1. / (1. + f32::exp(-x));
//         }
//         Self { lut }
//     }

//     fn f(&self, x: f32) -> f32 {
//         if x >= Self::RANGE {
//             self.lut[Self::SIZE - 1]
//         } else if x <= -Self::RANGE {
//             self.lut[0]
//         } else {
//             let idx = (x * Self::SCALE) as usize + Self::SIZE / 2;
//             self.lut[idx]
//         }
//     }
// }

struct Clut {
    lut: Vec<Vec<(f32, f32, f32)>>,
}

impl Clut {
    const HUES: usize = 360;
    const VALUES: usize = 256;

    fn new() -> Self {
        use hsluv::hsluv_to_rgb;
        let mut lut = vec![vec![(0., 0., 0.); Self::VALUES]; Self::HUES];
        for h in 0..Self::HUES {
            for v in 0..Self::VALUES {
                let c = hsluv_to_rgb((h as f64, 100., 100. * v as f64 / 256.));
                let c = Self::gamma(c);
                lut[h][v] = (c.0 as f32, c.1 as f32, c.2 as f32);
            }
        }
        Self { lut }
    }

    fn gamma(c: (f64, f64, f64)) -> (f64, f64, f64) {
        (c.0 * c.0, c.1 * c.1, c.2 * c.2)
    }

    /// lookup hue + value normalized to range [0,1)
    fn lookup(&self, h: f32, v: f32) -> (f32, f32, f32) {
        let hu = ((h * Self::HUES as f32) as isize % Self::HUES as isize).abs() as usize;
        let vi = (v * Self::VALUES as f32) as isize;
        let vu = isize::max(isize::min(vi, Self::VALUES as isize - 1), 0) as usize;
        self.lut[hu][vu]
    }
}
