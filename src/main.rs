mod audiosys;
use audiosys::analysis::{AudioAnalysis, AudioParams};
use audiosys::intensity::{AudioIntensity, AudioIntensitySystem};

use amethyst::{prelude::*, utils::application_root_dir};
use clap::Clap;

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
//     audio: AudioParams,
//     visualizer: visualizer::Params,
// }

// impl Config {
//     const CONFIG_FILE: &'static str = ".ledconfig.yaml";

//     fn default() -> Self {
//         Self {
//             audio: AudioParams::default(),
//             visualizer: visualizer::Params::default(),
//         }
//     }
// }

struct Init {
    audio_opts: audiosys::analysis::Opts,
}

impl SimpleState for Init {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        let world = data.world;
        let resources = data.resources;

        let default_features = self.audio_opts.default_features();
        resources.insert(default_features);

        let bins = self.audio_opts.bins;
        world.extend((0..bins).map(|i| {
            (AudioIntensity {
                frame: 0,
                bucket: i,
            },)
        }));
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData>) -> SimpleTrans {
        Trans::Replace(Box::new(Visualizer))
    }
}

struct Visualizer;
impl SimpleState for Visualizer {}

fn main() {
    let opts = Opts::parse();

    amethyst::start_logger(Default::default());
    let app_root = application_root_dir().expect("failed to get app_root");

    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            let params = AudioParams::default();
            let audio = AudioAnalysis::new(audio_opts.clone(), params, opts.verbose);

            let mut dispatcher = DispatcherBuilder::default();
            dispatcher
                .add_thread_local(Box::new(audio))
                .add_system(Box::new(AudioIntensitySystem));

            let game = Application::new(app_root, Init { audio_opts }, dispatcher)
                .expect("failed to create app");
            game.run();
        }
    }

    println!("oh, we done..?");
}
