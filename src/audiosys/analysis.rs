use std::sync::mpsc::{channel, sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread;

use amethyst::{core::dispatcher::ThreadLocalSystem, prelude::*};
use audio::Analyzer;
use clap::Clap;

use super::{AnalyzerParams, AudioFeatures};

#[derive(Clap, Clone)]
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
    pub bins: usize,

    #[clap(long, short = 'l', default_value = "144")]
    pub length: usize,
}

impl Opts {
    pub fn default_features(&self) -> AudioFeatures {
        AudioFeatures::new(self.bins, self.length)
    }
}

pub struct AudioAnalysis {
    verbose: i32,
    get_features: Receiver<AudioFeatures>,
    send_params: SyncSender<AnalyzerParams>,
}

impl AudioAnalysis {
    pub fn new(opts: Opts, params: AnalyzerParams, verbose: i32) -> Self {
        let Opts {
            device,
            sample_rate,
            sample_block_size,
            fft_size,
            bins,
            length,
        } = opts;
        let (audio_data_tx, audio_data_rx) = channel();
        let (send_features, get_features) = sync_channel(1);
        let (send_params, recv_params) = sync_channel(1);
        let now = std::time::SystemTime::now();

        thread::spawn(move || {
            #[cfg(feature = "ledpanel")]
            {
                // This is an ugly hack to work around that we might be starting this with root
                // privileges to initialize the ledpanel display driver on the pi, but then the
                // driver will drop privileges. Jack will only let us create a client as the same
                // user that is running the daemon.
                thread::sleep(std::time::Duration::from_secs(2));
            }

            let mut analyzer = Analyzer::new(fft_size, sample_block_size, bins, length);
            let mut params = params;

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

            audio::Source::print_devices(false).expect("failed to print devices");

            let s = audio::Source::new(device.as_deref()).expect("failed to get device");
            let _stream = s
                .get_stream(
                    1,
                    sample_rate as u32,
                    sample_block_size as u32,
                    handle_stream,
                )
                .expect("failed to get stream");

            loop {
                match recv_params.try_recv() {
                    Ok(new_params) => params = new_params,
                    Err(TryRecvError::Empty) => (),
                    Err(e) => {
                        println!("failed to recv params: {}", e);
                        break;
                    }
                };

                match audio_data_rx.recv() {
                    Ok(mut data) => {
                        if let Some(features) = analyzer.process(&mut data, &params) {
                            if verbose >= 2 && features.get_frame_count() % 32 == 0 {
                                let mut out = String::new();
                                analyzer
                                    .write_debug(&mut out)
                                    .expect("failed to write debug");
                                println!("{}", out);
                            }
                            if let Err(e) = send_features.try_send(features.clone()) {
                                match e {
                                    TrySendError::Full(_) => (),
                                    e => {
                                        if verbose >= 3 {
                                            println!(
                                                "[{:08}]: failed to send features: {}",
                                                now.elapsed().unwrap().as_millis(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
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
            send_params,
            verbose,
        }
    }
}

impl ThreadLocalSystem<'static> for AudioAnalysis {
    fn build(self) -> Box<dyn Runnable> {
        let mut now = std::time::SystemTime::now();
        Box::new(
            SystemBuilder::new("AudioAnalysis")
                .write_resource::<AudioFeatures>()
                .write_resource::<Option<AnalyzerParams>>()
                .build(move |_commands, _world, (features, params), _queries| {
                    if let Ok(feat) = self.get_features.try_recv() {
                        if self.verbose >= 3 {
                            println!(
                                "[{:?}] AudioAnalysis system received features #{}",
                                now.elapsed(),
                                feat.get_frame_count(),
                            );
                            now = std::time::SystemTime::now();
                        }
                        **features = feat;
                    }
                    if let Some(params) = params.take() {
                        if let Err(e) = self.send_params.send(params) {
                            println!("failed to send params: {}", e);
                        }
                    }
                }),
        )
    }
}

// pub struct AudioParamSystem;

// impl ThreadLocalSystem<'_> for AudioParamSystem {
//     fn build(self) -> Box<dyn Runnable> {
//         Box::new(SystemBuilder::new("audio param system")
//     .read
// )
//     }
// }

// #[system]
// pub fn update_audio_features(
//     #[state] audio: &AudioAnalysis,
//     #[resource] features: &mut AudioFeatures,
// ) {
//     if let Ok(feat) = audio.get_features.try_recv() {
//         if audio.verbose >= 3 {
//             println!("update_audio_features_system received features");
//         }
//         *features = feat;
//     }
// }
