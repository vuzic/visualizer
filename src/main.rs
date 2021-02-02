use actix::*;
use anyhow::Result;
use clap::Clap;
use log::info;

mod api;
mod audiosys;
mod config;
mod visualizer;
use api::ApiServer;
use config::Config;

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

fn get_config(opts: &Opts) -> Result<Config> {
    use std::path::{Path, PathBuf};
    let config_path = opts.config.as_ref().map(|p| PathBuf::from(&p)).unwrap_or(
        Path::new(&std::env::var("HOME").unwrap())
            .join(".config")
            .join("vuzic")
            .join("vuzic.yaml"),
    );
    if opts.verbose > 0 {
        info!("Using config at {:?}", config_path.clone().into_os_string());
    }

    let config = match std::fs::File::open(config_path.clone()) {
        Ok(f) => serde_yaml::from_reader(f)?,
        Err(_) => {
            let config = Config::default();
            if let Command::Init = opts.cmd {
                std::fs::create_dir_all(config_path.parent().unwrap())?;
                let f = std::fs::File::create(config_path)?;
                serde_yaml::to_writer(f, &config)?;
                info!("Wrote config");
            };
            config
        }
    };
    if opts.verbose > 0 {
        info!("{:?}", config)
    }

    Ok(config)
}

fn setup_logging(verbose: i32) {
    use log::{debug, trace, LevelFilter};
    amethyst::start_logger(if verbose > 3 {
        amethyst::LoggerConfig {
            level_filter: LevelFilter::Trace,
            ..Default::default()
        }
    } else if verbose > 0 {
        amethyst::LoggerConfig {
            level_filter: LevelFilter::Debug,
            ..Default::default()
        }
    } else {
        Default::default()
    });

    debug!("log level set to debug");
    trace!("log level set to trace");
}

mod app;
use app::App;
use audiosys::analysis::AudioAnalysis;

fn main() {
    let opts = Opts::parse();

    setup_logging(opts.verbose);

    let config = get_config(&opts).expect("failed to get config");

    let verbose = opts.verbose;
    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            let mut sys = System::new("system");
            sys.block_on(async move {
                let server = ApiServer::create(|ctx| {
                    let server = ctx.address();

                    let (audio, audio_sys) = AudioAnalysis::new(
                        audio_opts.clone(),
                        Default::default(),
                        server.recipient(),
                        verbose,
                    );

                    let audio_addr = audio.start();

                    let app = App::new(config, audio_opts, audio_sys, verbose).start();
                    ApiServer::new(app, audio_addr)
                });
                api::run("127.0.0.1", "8080", server).await
            })
            .expect("failed to create actix system");

            sys.run().expect("actix system runtime error");
        }
    }
}
