mod audiosys;
use audiosys::{analysis::AudioAnalysis, AudioParams};

mod visualizer;

#[cfg(feature = "gpu")]
use visualizer::warpgrid::WarpGridRender;

#[cfg(feature = "gpu")]
use amethyst::renderer::{
    plugins::RenderToWindow, rendy::hal::command::ClearColor, types::DefaultBackend,
    RenderingBundle,
};

#[cfg(feature = "ledpanel")]
use visualizer::ledpanel;

use amethyst::{
    core::{frame_limiter::FrameRateLimitStrategy, transform::TransformBundle},
    prelude::*,
    // utils::application_root_dir,
};
use clap::Clap;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
struct Config {
    audio: AudioParams,
}

impl Config {
    fn default() -> Self {
        Self {
            audio: AudioParams::default(),
        }
    }
}

struct Init {
    audio_opts: audiosys::analysis::Opts,
}

impl SimpleState for Init {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        let default_features = self.audio_opts.default_features();
        data.resources.insert(default_features);
        println!("@@@ inserted features");
    }

    fn update(&mut self, _data: &mut StateData<'_, GameData>) -> SimpleTrans {
        Trans::Replace(Box::new(Visualizer {
            bins: self.audio_opts.bins,
            length: self.audio_opts.length,
        }))
    }
}

struct Visualizer {
    bins: usize,
    length: usize,
}

impl SimpleState for Visualizer {
    fn on_start(&mut self, _data: StateData<'_, GameData>) {}
}

fn main() {
    let opts = Opts::parse();

    amethyst::start_logger(Default::default());
    // let app_root = application_root_dir().expect("failed to get app_root");
    let app_root = std::path::Path::new(".");
    // let display_config_path = app_root.join("config").join("display.ron");
    let audio_config_path = app_root.join("config").join("audio.yaml");

    let config = match std::fs::File::open(audio_config_path.clone()) {
        Ok(f) => serde_yaml::from_reader(f).unwrap(),
        Err(_) => {
            let config = Config::default();
            if let Command::Init = opts.cmd {
                let f = std::fs::File::create(audio_config_path).unwrap();
                serde_yaml::to_writer(f, &config).unwrap();
            };
            config
        }
    };

    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            let audio = AudioAnalysis::new(audio_opts.clone(), config.audio, opts.verbose);

            let mut dispatcher = DispatcherBuilder::default();
            dispatcher
                .add_thread_local(audio)
                .add_bundle(TransformBundle);

            #[cfg(feature = "gpu")]
            {
                dispatcher.add_bundle(
                    RenderingBundle::<DefaultBackend>::new()
                        .with_plugin(
                            RenderToWindow::from_config_path(display_config_path)
                                .unwrap()
                                .with_clear(ClearColor {
                                    float32: [0.0, 0.0, 0.0, 1.0],
                                }),
                        )
                        .with_plugin(WarpGridRender::default()),
                );
            }

            #[cfg(feature = "ledpanel")]
            {
                use ledpanel::RenderToPanel;
                let render = RenderToPanel::new(opts.verbose);
                dispatcher.add_thread_local(render);
            }

            let game = Application::build(app_root, Init { audio_opts })
                .expect("failed to create app builder")
                .with_frame_limit(FrameRateLimitStrategy::Unlimited, 240)
                .build(dispatcher)
                .expect("failed to build app");
            game.run();
        }
    }

    println!("oh, we done..?");
}
