use std::{io::Read, net::TcpListener};

use clap::Clap;
use image::RgbImage;

use panel_driver::{Options, Panel};

/// LED Panel Video Streamer
#[derive(Clap)]
#[clap(version = "0.1", author = "Steven Cohen <peragwin@gmail.com>")]
struct Opts {
    /// Verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,

    /// Host/port to listen for stream. e.g. udp://localhost:1234
    listen: String,

    #[clap(long)]
    led_pwm_lsb_nano: Option<u32>,
}

fn main() -> std::io::Result<()> {
    let opts = Opts::parse();

    let mut panel_opts = Options::default();
    panel_opts.pwm_lsb_nanoseconds = opts.led_pwm_lsb_nano;
    let (width, height) = panel_opts.frame_size();

    let panel = Panel::new(opts.verbose, panel_opts);

    let buf_size = (width * height * 3) as usize;
    let mut buf = vec![0u8; buf_size];

    let listener = TcpListener::bind(opts.listen)?;

    // accept connections and process them, spawning a new thread for each one
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                /* connection succeeded */
                while let Ok(()) = stream.read_exact(&mut buf) {
                    let frame = RgbImage::from_raw(width, height, buf.clone()).unwrap();
                    panel.send_frame(frame).unwrap();
                }
                let frame = RgbImage::from_raw(width, height, vec![0u8; buf_size]).unwrap();
                panel.send_frame(frame).unwrap();
            }
            Err(err) => {
                /* connection failed */
                log::info!("listen stream error: {}", err);
            }
        }
    }
    Ok(())
}
