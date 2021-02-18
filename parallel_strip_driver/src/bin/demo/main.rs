use clap::Clap;
use image::RgbaImage;
use parallel_strip_driver::{APA102Parallel, Hardware};
use simple_logger::SimpleLogger;

/// LED Strip Parallel Demo
#[derive(Clap)]
struct Opts {
    #[clap(long, default_value = "16")]
    spi_mhz: u8,

    #[clap(long)]
    counter_preset: Option<u8>,

    #[clap(long, default_value = "1")]
    alpha: u8,
}

fn rainbow(l: u32, w: f32, alpha: u8) -> Vec<u8> {
    use std::f32::consts::PI;
    let to_u8 = |x| (127.0 * (1.0 + x)) as u8;
    (0..l)
        .map(|i| 2.0 * PI / l as f32 * i as f32 + w)
        .map(|x| {
            (
                f32::sin(x),
                -1.0,
                -1.0,
                // f32::sin(x + 2.0 * PI / 3.0),
                // f32::sin(x + 4.0 * PI / 3.0),
            )
        })
        .flat_map(|(r, g, b)| vec![to_u8(r), to_u8(g), to_u8(b), alpha].into_iter())
        .collect()
}

fn main() {
    SimpleLogger::new().init().unwrap();
    let opts = Opts::parse();
    let alpha = opts.alpha;
    let spi_clock = opts.spi_mhz as u32 * 1_000_000;

    let hw = Hardware::new(spi_clock, 17, 22, 27, 5, 6, 13, 19, opts.counter_preset)
        .expect("failed to create hardware");
    let leds = APA102Parallel::new(144, 16, hw);

    let mut i = 0;
    loop {
        i += 1;
        let buf = (0..16)
            .flat_map(|_| rainbow(144, i as f32 / 32.0, alpha))
            .collect::<Vec<u8>>();
        let img = RgbaImage::from_raw(144, 16, buf).unwrap();
        leds.display(img);
    }
}
