mod audiosys;
use audiosys::analysis::AudioAnalysis;
use audiosys::component::AudioIntensity;
use audiosys::system::AudioSystem;

use clap::Clap;
use specs::prelude::*;
use specs::{DispatcherBuilder, World};

/// Vuzic Audio Visualizer
#[derive(Clap)]
#[clap(version = "0.1", author = "Steven Cohen <peragwin@gmail.com>")]
struct Opts {
    /// Verbosity, can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,

    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    Init,
    Run(audiosys::analysis::Opts),
}

// #[derive(Serialize, Deserialize, Copy, Clone, Debug)]
// struct Config {
//     audio: FrequencySensorParams,
//     visualizer: visualizer::Params,
// }

// impl Config {
//     const CONFIG_FILE: &'static str = ".ledconfig.yaml";

//     fn default() -> Self {
//         Self {
//             audio: FrequencySensorParams::defaults(),
//             visualizer: visualizer::Params::defaults(),
//         }
//     }
// }

fn main() {
    let opts = Opts::parse();

    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            let mut world = World::new();
            world.register::<AudioIntensity>();

            let default_features = audio_opts.default_features();
            world.insert(default_features);

            let params = audio::frequency_sensor::FrequencySensorParams::defaults();
            let audio = AudioAnalysis::new(audio_opts, params, opts.verbose);

            let mut dispatcher = DispatcherBuilder::new()
                .with(audio, "audio_analysis", &[])
                .with(AudioSystem, "audio_system", &["audio_analysis"])
                .build();

            for i in 0..4 {
                world
                    .create_entity()
                    .with(AudioIntensity {
                        frame: 0,
                        bucket: i,
                    })
                    .build();
            }

            loop {
                dispatcher.dispatch(&mut world);
                world.maintain();
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    println!("oh, we done..?");
}
