use std::thread;
use std::{
    io::Read,
    os::linux::raw,
    sync::mpsc::{sync_channel, SyncSender},
};

use anyhow::Result;
use image::{Pixel, RgbaImage};
use rppal::{
    gpio::{Gpio, OutputPin},
    spi::{Bus, Mode, Polarity, SlaveSelect, Spi},
};

/// APA102 strips parallel driver
pub struct APA102Parallel {
    length: u32,
    rows: u32,
    buffer: Vec<u16>,
    send_frame: SyncSender<RgbaImage>,
}

pub struct Hardware {
    spi: Spi,
    lclk_en_l: OutputPin,
    reset_l: OutputPin,
    // cnt_en: OutputPin,
    cnt_load_l: OutputPin,
    cnt_p3: OutputPin,
    cnt_p2: OutputPin,
    cnt_p1: OutputPin,
    cnt_p0: OutputPin,
    preset: u8,
}

impl Hardware {
    pub fn new(
        spi_clock: u32,
        lclk_en: u8,
        reset: u8,
        // cnt_en: u8,
        cnt_load: u8,
        cnt_p3: u8,
        cnt_p2: u8,
        cnt_p1: u8,
        cnt_p0: u8,
        preset: Option<u8>,
    ) -> Result<Self> {
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, spi_clock, Mode::Mode0)?;
        log::debug!("created spi");
        let gpio = Gpio::new()?;
        log::debug!("created gpio");

        let mut lclk_en_l = gpio.get(lclk_en)?.into_output();
        lclk_en_l.set_high();

        let mut reset_l = gpio.get(reset)?.into_output();
        reset_l.set_high();

        let mut cnt_load_l = gpio.get(cnt_load)?.into_output();
        cnt_load_l.set_high();

        let cnt_p3 = gpio.get(cnt_p3)?.into_output();
        let cnt_p2 = gpio.get(cnt_p2)?.into_output();
        let cnt_p1 = gpio.get(cnt_p1)?.into_output();
        let cnt_p0 = gpio.get(cnt_p0)?.into_output();
        let preset = preset.unwrap_or(15);

        Ok(Self {
            spi,
            lclk_en_l,
            reset_l,
            cnt_load_l,
            cnt_p3,
            cnt_p2,
            cnt_p1,
            cnt_p0,
            preset,
        })
    }

    fn load_preset(&mut self) -> Result<()> {
        let p = self.preset;
        for (i, po) in [
            &mut self.cnt_p0,
            &mut self.cnt_p1,
            &mut self.cnt_p2,
            &mut self.cnt_p3,
        ]
        .iter_mut()
        .enumerate()
        {
            if p & (1 << i) > 0 {
                po.set_high();
            } else {
                po.set_low();
            }
        }

        self.cnt_load_l.set_low();
        self.spi.write(&[0])?;
        self.cnt_load_l.set_high();
        Ok(())
    }

    fn reset(&mut self) {
        let slp = std::time::Duration::from_micros(100);
        self.reset_l.set_low();
        thread::sleep(slp);
        self.reset_l.set_high();
        thread::sleep(slp);
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.reset();
        self.load_preset()?;
        self.lclk_en_l.set_low();

        let mut rem = data.len();
        let mut data = data;
        loop {
            let bs = &data[..4096.min(rem)];
            let w = self.spi.write(bs)?;
            rem -= w;
            if rem <= 0 {
                break;
            }
            data = &data[rem..];
            // hw.write(&bs).expect("failed to write frame");
        }
        self.spi.write(&[0xff; 16])?;
        self.spi.write(&[0; 128])?;

        self.lclk_en_l.set_high();
        Ok(())
    }
}

fn to_bytes(input: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 * input.len());
    for value in input {
        bytes.extend(&value.to_be_bytes());
    }
    bytes
}

impl APA102Parallel {
    /// Create a APA102Parallel driver with grid dimensions
    pub fn new(length: u32, rows: u32, hardware: Hardware) -> Self {
        let end_frame = (6 + length / 16) as usize;
        let led_frame = (4 * (length + 1)) as usize;
        let buffer_size = led_frame + end_frame;
        let mut b = vec![0u8; buffer_size];
        b[led_frame] = 0xff;
        let buffer: Vec<u8> = (0..rows).map(|_| b.clone()).flatten().collect();
        let buffer = RgbaImage::from_raw(buffer_size as u32 / 4, rows, buffer).unwrap();
        let buffer = Self::to_output_buffer(buffer);

        let (send_frame, recv_frame) = sync_channel(1);

        thread::spawn(move || {
            let mut frame_count = 0;
            let mut then = std::time::SystemTime::now();
            let mut hw = hardware;

            hw.spi
                .set_ss_polarity(Polarity::ActiveHigh)
                .expect("failed to set spi polarity");

            while let Ok(image) = recv_frame.recv() {
                let buffer = Self::to_output_buffer(image);
                let bs = to_bytes(buffer.as_slice());
                hw.write(&bs).expect("failed to write frame");

                frame_count += 1;
                if frame_count % 256 == 0 {
                    let now = std::time::SystemTime::now();
                    if let Ok(e) = now.duration_since(then) {
                        then = now;
                        log::debug!("fps: {:.02}", 256.0 / e.as_secs_f32());
                    }
                }
            }

            panic!("uh oh");
        });

        Self {
            length,
            rows,
            buffer,
            send_frame,
        }
    }

    pub fn display(&self, image: RgbaImage) {
        if let Err(e) = self.send_frame.send(image) {
            log::error!("failed to send frame: {}", e);
        }
    }

    fn to_output_buffer(image: RgbaImage) -> Vec<u16> {
        let (cols, rows) = image.dimensions();

        let mut img = vec![0u32; (cols * rows) as usize];
        for col in 0..cols {
            for row in 0..rows {
                let ix = (row * cols + col) as usize;
                let p = &image.as_raw()[4 * ix..4 * ix + 4];
                let ix = (col * rows + row) as usize; // transpose maybe improves the inner loop below?
                img[ix] = (((p[3] as u32).min(31) | 0xe0) << 24)
                    | ((p[2] as u32) << 16)
                    | ((p[1] as u32) << 8)
                    | p[0] as u32;
            }
        }

        let mut buf = vec![0u16; 32 * cols as usize];

        for col in 0..cols {
            for k in 0..32 {
                let nk = 31 - k;
                let sel = 1 << nk;
                let mut v = 0;
                for row in 0..rows {
                    // let ix = (row * cols + col) as usize;
                    let ix = (col * rows + row) as usize;
                    let s = img[ix] & sel;
                    let rnk = row as i32 - nk as i32;
                    v |= if rnk < 0 { s >> -rnk } else { s << rnk };
                }
                buf[(col * 32 + k) as usize] = v as u16;
            }
        }

        for i in 0..32 {
            // print!("{:x},", (buf[i] >> 8) & 1);
            print!("{:x},", buf[i] & 1);
        }
        println!("");

        buf
    }
}

#[cfg(test)]
mod test {
    use super::APA102Parallel;
    use image::RgbaImage;
    #[test]
    pub fn to_output_buffer() {
        for i in 0..16 {
            let raw: Vec<u8> = (0..16)
                .map(|j| {
                    if i == j {
                        [vec![0xa5; 4], vec![0x5a; 4]].concat()
                    } else {
                        vec![0; 8]
                    }
                })
                .flatten()
                .collect();
            let image = RgbaImage::from_raw(2, 16, raw).unwrap();
            let buffer = APA102Parallel::to_output_buffer(image);
            println!("{:?}", buffer);
        }
    }
}
