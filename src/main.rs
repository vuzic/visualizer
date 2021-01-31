use anyhow::Result;

mod audiosys;
use audiosys::{analysis::AudioAnalysis, AnalyzerParams};

mod visualizer;
use visualizer::cpurender::Params as RenderParams;

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

    /// Path to config file
    #[clap(short, long)]
    config: Option<String>,

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
    audio: AnalyzerParams,
    render: RenderParams,
    #[cfg(feature = "ledpanel")]
    panel: ledpanel::Options,
}

impl Config {
    fn default() -> Self {
        Self {
            audio: Default::default(),
            render: Default::default(),
            panel: Default::default(),
        }
    }
}

struct Init {
    audio_opts: audiosys::analysis::Opts,
    config: Config,
}

impl SimpleState for Init {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        data.resources.insert(Some(self.config.audio));
        data.resources.insert(self.config.render);

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

fn get_config(opts: &Opts) -> Result<Config> {
    use std::path::{Path, PathBuf};
    let config_path = opts.config.as_ref().map(|p| PathBuf::from(&p)).unwrap_or(
        Path::new(&std::env::var("HOME").unwrap())
            .join(".config")
            .join("vuzic")
            .join("vuzic.yaml"),
    );
    if opts.verbose > 0 {
        println!("Using config at {:?}", config_path.clone().into_os_string());
    }

    let config = match std::fs::File::open(config_path.clone()) {
        Ok(f) => serde_yaml::from_reader(f)?,
        Err(_) => {
            let config = Config::default();
            if let Command::Init = opts.cmd {
                std::fs::create_dir_all(config_path.parent().unwrap())?;
                let f = std::fs::File::create(config_path)?;
                serde_yaml::to_writer(f, &config)?;
                println!("Wrote config");
            };
            config
        }
    };
    if opts.verbose > 0 {
        println!("{:?}", config)
    }

    Ok(config)
}

fn main() {
    let opts = Opts::parse();

    amethyst::start_logger(Default::default());
    // let app_root = application_root_dir().expect("failed to get app_root");
    let app_root = std::path::Path::new(".");

    let config = get_config(&opts).expect("failed to get config");

    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            let audio = AudioAnalysis::new(audio_opts.clone(), Default::default(), opts.verbose);

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
                let render = RenderToPanel::new(opts.verbose, config.panel.clone());
                dispatcher.add_thread_local(render);
            }

            let game = Application::build(app_root, Init { audio_opts, config })
                .expect("failed to create app builder")
                .with_frame_limit(FrameRateLimitStrategy::Unlimited, 240)
                .build(dispatcher)
                .expect("failed to build app");
            game.run();
        }
    }

    println!("oh, we done..?");
}
