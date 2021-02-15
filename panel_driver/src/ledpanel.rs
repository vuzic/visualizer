use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

use anyhow::Result;
use image::RgbImage;
use rpi_led_matrix::{LedColor, LedMatrix, LedMatrixOptions, LedRuntimeOptions};
use serde::{Deserialize, Serialize};

pub struct Panel {
    send_frame_: SyncSender<RgbImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    pub cols: Option<u32>,
    pub rows: Option<u32>,
    pub chain_length: Option<u32>,
    pub parallel: Option<u32>,
    pub hardware_mapping: Option<String>,
    pub pwm_dither_bits: Option<u32>,
    pub pwm_lsb_nanoseconds: Option<u32>,
    pub gpio_slowdown: Option<u32>,
}

impl Options {
    pub fn into_matrix_options(&self) -> (LedMatrixOptions, LedRuntimeOptions) {
        let mut opts = LedMatrixOptions::new();
        if let Some(v) = self.cols {
            opts.set_cols(v);
        }
        if let Some(v) = self.rows {
            opts.set_rows(v);
        }
        if let Some(v) = self.chain_length {
            opts.set_chain_length(v);
        }
        if let Some(v) = self.parallel {
            opts.set_parallel(v);
        }
        if let Some(v) = &self.hardware_mapping {
            opts.set_hardware_mapping(v);
        }
        if let Some(v) = self.pwm_dither_bits {
            opts.set_pwm_dither_bits(v);
        }
        if let Some(v) = self.pwm_lsb_nanoseconds {
            opts.set_pwm_lsb_nanoseconds(v);
        }

        let mut rt_opts = LedRuntimeOptions::new();
        if let Some(v) = self.gpio_slowdown {
            rt_opts.set_gpio_slowdown(v);
        }

        (opts, rt_opts)
    }

    pub fn frame_size(&self) -> (u32, u32) {
        (
            self.cols.unwrap_or_default() * self.chain_length.unwrap_or(1),
            self.rows.unwrap_or_default() * self.parallel.unwrap_or(1),
        )
    }
}

// FIXME: this isn't "default". Default should be None
impl Default for Options {
    fn default() -> Self {
        Self {
            cols: Some(64),
            rows: Some(32),
            chain_length: Some(3),
            parallel: Some(2),
            hardware_mapping: Some("vuzic".to_string()),
            pwm_dither_bits: Some(0),
            pwm_lsb_nanoseconds: Some(120),
            gpio_slowdown: Some(3),
        }
    }
}

impl Panel {
    pub fn new(verbose: i32, options: Options) -> Self {
        let (send_frame_, recv_frame) = sync_channel::<RgbImage>(1);

        thread::spawn(move || {
            let (mut options, rt_options) = options.into_matrix_options();
            // options.set_cols(64);
            // options.set_rows(32);
            // options.set_chain_length(3);
            // options.set_parallel(2);
            // options.set_hardware_mapping("vuzic");
            options.set_limit_refresh(0);
            options.set_hardware_pulsing(true);
            // options.set_panel_type("FM6126A");

            let matrix = LedMatrix::new(Some(options), Some(rt_options))
                .expect("failed to create ledmatrix");

            let mut then = std::time::SystemTime::now();
            let mut frame_count = 0;
            let mut canvas = matrix.offscreen_canvas();

            loop {
                match recv_frame.recv() {
                    Ok(frame) => {
                        for (x, y, c) in frame.enumerate_pixels() {
                            let (red, green, blue) = (c[0], c[1], c[2]);
                            canvas.set(x as i32, y as i32, &LedColor { red, green, blue });
                        }
                        canvas = matrix.swap(canvas);
                        frame_count += 1;
                        if verbose > 0 && frame_count % 256 == 0 {
                            let e = then.elapsed().unwrap().as_secs_f32();
                            log::debug!("FPS: {:.2}", 256. / e);
                            then = std::time::SystemTime::now();
                        }
                    }
                    Err(e) => {
                        log::error!("failed to recv frame: {}", e);
                        break;
                    }
                };
            }
        });

        Self { send_frame_ }
    }

    pub fn send_frame(&self, frame: RgbImage) -> Result<()> {
        self.send_frame_.send(frame)?;
        Ok(())
    }
}
