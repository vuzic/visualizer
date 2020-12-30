mod audiosys;
use audiosys::analysis::{AudioAnalysis, AudioParams};
use audiosys::intensity::{AudioIntensity, AudioIntensityScaleSystem};

use amethyst::{
    assets::{AssetStorage, Loader},
    core::{
        frame_limiter::FrameRateLimitStrategy,
        transform::{Transform, TransformBundle},
    },
    prelude::*,
    renderer::{
        camera::Camera,
        light::{Light, PointLight},
        mtl::{Material, MaterialDefaults},
        palette::{LinSrgba, Srgb},
        plugins::{RenderShaded3D, RenderToWindow},
        rendy::{
            hal::command::ClearColor,
            mesh::{Normal, Position, Tangent, TexCoord},
            texture::palette::load_from_linear_rgba,
        },
        shape::Shape,
        types::DefaultBackend,
        Mesh, RenderingBundle, Texture,
    },
    utils::application_root_dir,
    window::ScreenDimensions,
};
use clap::Clap;
use itertools::iproduct;
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
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        let StateData {
            world, resources, ..
        } = data;

        let mat_defaults = resources.get::<MaterialDefaults>().unwrap().0.clone();
        let loader = resources.get::<Loader>().unwrap();
        let mesh_storage = resources.get::<AssetStorage<Mesh>>().unwrap();
        let tex_storage = resources.get::<AssetStorage<Texture>>().unwrap();
        let mtl_storage = resources.get::<AssetStorage<Material>>().unwrap();

        let (mesh, albedo) = {
            let mesh = loader.load_from_data(
                Shape::Sphere(32, 32)
                    .generate::<(Vec<Position>, Vec<Normal>, Vec<Tangent>, Vec<TexCoord>)>(None)
                    .into(),
                (),
                &mesh_storage,
            );

            let albedo = loader.load_from_data(
                load_from_linear_rgba(LinSrgba::new(1.0, 1.0, 1.0, 0.5)).into(),
                (),
                &tex_storage,
            );

            (mesh, albedo)
        };

        let spheres = iproduct!(0..self.bins, 0..self.length / 8).map(|(n, m)| {
            let mut pos = Transform::default();
            pos.set_translation_xyz(
                -12.0f32 * (n + 1) as f32 / (self.bins + 2) as f32 + 6.0,
                -12.0f32 * (m + 1) as f32 / (self.length / 8 + 2) as f32 + 6.0,
                0.0,
            );

            let mtl = {
                let metallic_roughness = loader.load_from_data(
                    load_from_linear_rgba(LinSrgba::new(0.0, 0.5, 0.5, 0.0)).into(),
                    (),
                    &tex_storage,
                );
                loader.load_from_data(
                    Material {
                        albedo: albedo.clone(),
                        metallic_roughness,
                        ..mat_defaults.clone()
                    },
                    (),
                    &mtl_storage,
                )
            };

            (
                pos,
                mesh.clone(),
                mtl,
                AudioIntensity {
                    frame: m * 8,
                    bucket: n,
                },
            )
        });

        world.extend(spheres);

        let light1: Light = PointLight {
            intensity: 6.0,
            color: Srgb::new(0.8, 0.0, 0.8),
            ..PointLight::default()
        }
        .into();

        let mut light1_transform = Transform::default();
        light1_transform.set_translation_xyz(6.0, 6.0, -6.0);

        world.push((light1, light1_transform));

        let mut transform = Transform::default();
        transform.set_translation_xyz(0.0, 0.0, -12.0);
        transform.prepend_rotation_y_axis(std::f32::consts::PI);

        let (width, height) = {
            let dim = resources.get::<ScreenDimensions>().unwrap();
            (dim.width(), dim.height())
        };

        world.extend(vec![(Camera::standard_3d(width, height), transform)]);
    }
}

fn main() {
    let opts = Opts::parse();

    amethyst::start_logger(Default::default());
    let app_root = application_root_dir().expect("failed to get app_root");
    let display_config_path = app_root.join("config").join("display.ron");
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
                .add_thread_local(Box::new(audio))
                .add_bundle(TransformBundle)
                .add_system(Box::new(AudioIntensityScaleSystem))
                .add_bundle(
                    RenderingBundle::<DefaultBackend>::new()
                        .with_plugin(
                            RenderToWindow::from_config_path(display_config_path)
                                .unwrap()
                                .with_clear(ClearColor {
                                    float32: [0.34, 0.36, 0.52, 1.0],
                                }),
                        )
                        .with_plugin(RenderShaded3D::default()),
                );

            let game = Application::build(app_root, Init { audio_opts })
                .expect("failed to create app builder")
                .with_frame_limit(FrameRateLimitStrategy::Yield, 120)
                .build(dispatcher)
                .expect("failed to build app");
            game.run();
        }
    }

    println!("oh, we done..?");
}
