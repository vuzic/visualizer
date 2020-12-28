mod audiosys;
use audiosys::analysis::{update_audio_features_system, AudioAnalysis, AudioParams};
use audiosys::intensity::{update_audio_intensity_system, AudioIntensity};

// mod visualizer;

use clap::Clap;
use legion::*;

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
            let mut world = World::default();

            let mut resources = Resources::default();
            let default_features = audio_opts.default_features();
            resources.insert(default_features);

            let params = AudioParams::defaults();
            let bins = audio_opts.bins;
            let audio = AudioAnalysis::new(audio_opts, params, opts.verbose);
            let audio_features_system = update_audio_features_system(audio);
            let audio_intensity_system = update_audio_intensity_system();

            let mut schedule = Schedule::builder()
                .add_thread_local(audio_features_system)
                .add_system(audio_intensity_system)
                .build();

            world.extend((0..bins).map(|i| {
                (AudioIntensity {
                    frame: 0,
                    bucket: i,
                },)
            }));

            loop {
                schedule.execute(&mut world, &mut resources);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    println!("oh, we done..?");
}
