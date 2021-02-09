use serde::{Serialize, Deserialize};

#[cfg(feature = "gpu")]
pub mod warpgrid;

#[cfg(feature = "ledpanel")]
pub mod ledpanel;
#[cfg(feature = "ledpanel")]
pub mod cpurender;

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Params {
    value_scale: (f32, f32),
    lightness_scale: (f32, f32),
    alpha_scale: (f32, f32),
    max_alpha: f32,
    color_cycle_rate: f32,
    color_period: f32,
    blur: f32,
    hz_warp: (f32, f32),
    vt_warp: (f32, f32),
}

impl Default for Params {
    fn default() -> Self {
        Self {
            value_scale: (1.0, 0.0),
            lightness_scale: (0.76, 0.0),
            alpha_scale: (1.0, -1.0),
            max_alpha: 0.125,
            color_cycle_rate: 1. / 16.,
            color_period: 512.,
            blur: 1.0,
            hz_warp: (1.0, 1.0),
            vt_warp: (1.0, 1.0),
        }
    }
}
