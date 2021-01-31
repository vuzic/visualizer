use actix::Actor;
use anyhow::Result;
use clap::Clap;

mod api;
mod audiosys;
mod config;
mod visualizer;
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

mod app;
use app::App;

fn main() {
    let opts = Opts::parse();
    let config = get_config(&opts).expect("failed to get config");

    let verbose = opts.verbose;
    match opts.cmd {
        Command::Init => (),
        Command::Run(audio_opts) => {
            actix_web::rt::System::new("apiserver")
                .block_on(async move {
                    let app = App::new(config, audio_opts, verbose).start();
                    api::run("127.0.0.1", "8080", app).await
                })
                .expect("apiserver rt error");
        }
    }
}
