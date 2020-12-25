use std::sync::mpsc::{channel, Receiver};
use std::thread;

use audio::frequency_sensor::Features as AudioFeatures;
use audio::frequency_sensor::FrequencySensorParams as AudioParams;
use audio::Analyzer;
use clap::Clap;
use specs::{System, World, Write};

#[derive(Clap)]
pub struct Opts {
    #[clap(long, short)]
    device: Option<String>,

    #[clap(long, short = 'r', default_value = "44100")]
    sample_rate: usize,

    #[clap(long, short = 'b', default_value = "256")]
    sample_block_size: usize,

    #[clap(long, short = 'f', default_value = "1024")]
    fft_size: usize,

    #[clap(long, short = 'n', default_value = "16")]
    bins: usize,

    #[clap(long, short = 'l', default_value = "144")]
    length: usize,
}

impl Opts {
    pub fn default_features(&self) -> AudioFeatures {
        AudioFeatures::new(self.bins, self.length)
    }
}

pub struct AudioAnalysis {
    verbose: i32,
    get_features: Receiver<AudioFeatures>,
}

impl AudioAnalysis {
    pub fn new(opts: Opts, params: AudioParams, verbose: i32) -> Self {
        let Opts {
            device,
            sample_rate,
            sample_block_size,
            fft_size,
            bins,
            length,
        } = opts;
        let (audio_data_tx, audio_data_rx) = channel();
        let (send_features, get_features) = channel();
        let now = std::time::SystemTime::now();

        thread::spawn(move || {
            let boost_params = audio::gain_control::Params::defaults();
            let mut analyzer = Analyzer::new(
                fft_size,
                sample_block_size,
                bins,
                length,
                boost_params,
                params,
            );

            let handle_stream = move |data: &[f32]| {
                if verbose >= 4 {
                    println!("tx audio");
                }
                let data = data.iter().map(|&x| x as f64).collect();
                if let Err(e) = audio_data_tx.send(data) {
                    if verbose >= 3 {
                        println!(
                            "[{:08}]: failed to send audio data: {}",
                            now.elapsed().unwrap().as_millis(),
                            e
                        );
                    }
                }
            };
            // random rust thing:
            // https://stackoverflow.com/questions/25649423/sending-trait-objects-between-threads-in-rust
            let handle_stream = Box::new(handle_stream) as Box<dyn Fn(&[f32]) -> () + Send>;

            let s = audio::Source::new(device.as_deref()).expect("failed to get device");
            let _stream = s
                .get_stream(
                    1,
                    sample_rate as u32,
                    sample_block_size as u32,
                    handle_stream,
                )
                .expect("failed to get stream");

            let mut process = |mut data| {
                if let Some(features) = analyzer.process(&mut data) {
                    if verbose >= 2 && features.get_frame_count() % 32 == 0 {
                        let mut out = String::new();
                        analyzer
                            .write_debug(&mut out)
                            .expect("failed to write debug");
                        println!("{}", out);
                    }

                    if let Err(e) = send_features.send(features.clone()) {
                        if verbose >= 3 {
                            println!(
                                "[{:08}]: failed to send features: {}",
                                now.elapsed().unwrap().as_millis(),
                                e
                            );
                        }
                    }
                }
            };

            loop {
                match audio_data_rx.recv() {
                    Ok(data) => {
                        process(data);
                    }
                    Err(e) => {
                        println!("failed to recv audio: {}", e);
                        break;
                    }
                };
                if verbose >= 4 {
                    println!("rx audio");
                };
            }
        });

        Self {
            get_features,
            verbose,
        }
    }
}

impl<'s> System<'s> for AudioAnalysis {
    type SystemData = Write<'s, AudioFeatures>;

    fn run(&mut self, mut features: Self::SystemData) {
        if let Ok(feat) = self.get_features.try_recv() {
            if self.verbose >= 3 {
                println!("AudioAnalysis system received features");
            }
            *features = feat;
        }
    }

    // fn setup(&mut self, world: &mut World) {
    //     world.insert(AudioFeatures::default());
    // }
}
