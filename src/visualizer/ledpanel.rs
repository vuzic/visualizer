use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::thread;

use amethyst::{core::dispatcher::ThreadLocalSystem, ecs::*};
use image::RgbImage;
use rpi_led_matrix::{LedColor, LedMatrix, LedMatrixOptions, LedRuntimeOptions};
use serde::{Deserialize, Serialize};

use super::cpurender::{Params, Visualizer};
use crate::audiosys::AudioFeatures;

pub struct RenderToPanel {
    vis: Visualizer,
    send_frame: SyncSender<RgbImage>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Options {
    // pub cols: u32,
    // pub rows: u32,
    // pub chain_length: u32,
    // pub parallel: u32,
    // pub hardware_mapping: String,
    pub pwm_dither_bits: Option<u32>,
    pub pwm_lsb_nanoseconds: Option<u32>,
}

impl Options {
    pub fn into_matrix_options(&self) -> LedMatrixOptions {
        let mut opts = LedMatrixOptions::new();
        if let Some(v) = self.pwm_dither_bits {
            opts.set_pwm_dither_bits(v);
        }
        if let Some(v) = self.pwm_lsb_nanoseconds {
            opts.set_pwm_lsb_nanoseconds(v);
        }
        opts
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            pwm_dither_bits: Some(0),
            pwm_lsb_nanoseconds: Some(120),
        }
    }
}

impl RenderToPanel {
    pub fn new(verbose: i32, options: Options) -> Self {
        let (send_frame, recv_frame) = sync_channel::<RgbImage>(1);
        // let (send_params, recv_params) = sync_channel(1);

        thread::spawn(move || {
            let mut options = options.into_matrix_options();
            options.set_cols(64);
            options.set_rows(32);
            options.set_chain_length(3);
            options.set_parallel(2);
            options.set_hardware_mapping("vuzic");
            options.set_limit_refresh(0);
            options.set_hardware_pulsing(true);
            // options.set_panel_type("FM6126A");
            let mut rt_options = LedRuntimeOptions::new();
            rt_options.set_gpio_slowdown(3);

            let matrix = LedMatrix::new(Some(options), Some(rt_options))
                .expect("failed to create ledmatrix");
            println!("@@@ created ledmatrix!");

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
                            println!("FPS: {:.2}", 256. / e);
                            then = std::time::SystemTime::now();
                        }
                    }
                    Err(e) => {
                        println!("failed to recv frame: {}", e);
                        break;
                    }
                };
            }
        });

        let vis = Visualizer::new(192, 64, verbose);

        Self { vis, send_frame }
    }
}

impl ThreadLocalSystem<'static> for RenderToPanel {
    fn build(self) -> Box<dyn Runnable> {
        Box::new(
            SystemBuilder::new("led panel renderer")
                .read_resource::<Params>()
                .read_resource::<AudioFeatures>()
                .build(move |_commands, _world, (params, features), _query| {
                    let image = self.vis.render(params, features);
                    if let Err(e) = self.send_frame.send(image) {
                        match e {
                            // TrySendError::Full(_) => {
                            //     println!("send frame full")
                            // }
                            e => println!("failed to send frame: {}", e),
                        }
                    }
                }),
        )
    }
}

/*
pub struct LedPanelBundle {
    options: Option<LedMatrixOptions>,
    rt_options: Option<LedRuntimeOptions>,
    verbose: i32,
}

impl LedPanelBundle {
    pub fn new(
        options: Option<LedMatrixOptions>,
        rt_options: Option<LedRuntimeOptions>,
        verbose: i32,
    ) -> Self {
        Self {
            options,
            rt_options,
            verbose,
        }
    }
}

impl SystemBundle for LedPanelBundle {
    fn load(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
        builder: &mut DispatcherBuilder,
    ) -> Result<(), Error> {
        println!("ADD RNDERE@@@@@@@@@@@");

        let mut options = LedMatrixOptions::default();
        options.set_cols(64);
        options.set_rows(32);
        options.set_chain_length(3);
        options.set_parallel(2);
        options.set_hardware_mapping("vuzic");
        options.set_limit_refresh(240);
        let mut rt_options = LedRuntimeOptions::default();
        rt_options.set_gpio_slowdown(3);
        let matrix =
            LedMatrix::new(Some(options), Some(rt_options)).expect("failed to create matrix");

        println!("@@@ inserted ledmatrix!");

        let params = Params::defaults();
        let vis = Visualizer::new(192, 64, params, self.verbose);
        let render = RenderToPanel::new(0);
        builder.add_thread_local(Box::new(render));

        Ok(())
    }
}
*/
