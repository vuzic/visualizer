use actix::*;
#[cfg(feature = "gpu")]
use amethyst::renderer::{
    plugins::RenderToWindow, rendy::hal::command::ClearColor, types::DefaultBackend,
    RenderingBundle,
};
use amethyst::{
    core::{dispatcher::ThreadLocalSystem, frame_limiter::FrameRateLimitStrategy},
    ecs::*,
    prelude::*,
};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};

use crate::audiosys::{
    analysis::{AudioAnalysis, Opts as AudioOpts},
    AnalyzerParams,
};
use crate::config::{Config, OptionalConfig};
use crate::visualizer::cpurender::Params as RenderParams;
#[cfg(feature = "ledpanel")]
use crate::visualizer::ledpanel::{Options as LedPanelOptions, RenderToPanel};
#[cfg(feature = "gpu")]
use visualizer::warpgrid::WarpGridRender;

struct Init {
    audio_opts: AudioOpts,
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

/// actor which contains the game engine allowing it to comminicate with other actors
pub struct App {
    config: Config,
    config_update: SyncSender<OptionalConfig>,
}

impl App {
    pub(crate) fn new(config: Config, audio_opts: AudioOpts, verbose: i32) -> Self {
        let (config_update, config_mailbox) = sync_channel(1);

        let app_system = AppSystem { config_mailbox };

        // let config = config.clone();
        std::thread::spawn(move || {
            amethyst::start_logger(Default::default());

            let audio = AudioAnalysis::new(audio_opts.clone(), Default::default(), verbose);

            let mut dispatcher = DispatcherBuilder::default();
            dispatcher.add_thread_local(audio);
            dispatcher.add_thread_local(app_system);

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
                let render = RenderToPanel::new(verbose, config.panel.clone());
                dispatcher.add_thread_local(render);
            }

            let app_root = std::path::Path::new(".");
            let game = Application::build(app_root, Init { audio_opts, config })
                .expect("failed to create app builder")
                .with_frame_limit(
                    FrameRateLimitStrategy::SleepAndYield(std::time::Duration::from_millis(1)),
                    240,
                )
                .build(dispatcher)
                .expect("failed to build app");
            game.run();

            println!("oh, we done..?");
        });

        Self {
            config,
            config_update,
        }
    }
}

impl Actor for App {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {}
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ConfigMessage(OptionalConfig);

impl Handler<ConfigMessage> for App {
    type Result = ();

    fn handle(&mut self, config: ConfigMessage, _ctx: &mut Self::Context) {
        self.config_update.send(config.0);
    }
}

struct AppSystem {
    config_mailbox: Receiver<OptionalConfig>,
}

impl ThreadLocalSystem<'_> for AppSystem {
    fn build(self) -> Box<dyn Runnable> {
        let builder = SystemBuilder::new("app system")
            .write_resource::<AnalyzerParams>()
            .write_resource::<RenderParams>();

        #[cfg(feature = "ledpanel")]
        builder.write_resource::<LedPanelOptions>();

        Box::new(builder.build(move |_commands, _world, resources, _query| {
            match self.config_mailbox.try_recv() {
                Err(TryRecvError::Empty) => (),
                Ok(config) => {
                    if let Some(ap) = config.audio {
                        println!("updated audio params: {:?}", ap);
                        *resources.0 = ap;
                    }
                    if let Some(rp) = config.render {
                        println!("updated render params: {:?}", rp);
                        *resources.1 = rp;
                    }
                    #[cfg(feature = "ledpanel")]
                    if let Some(lp) = config.panel {
                        *resources.2 = lp;
                    }
                }
                Err(e) => println!("error recv on config_mailbox: {}", e),
            }
        }))
    }
}
