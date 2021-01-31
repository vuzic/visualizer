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
use log::{debug, error};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};

use crate::audiosys::{
    analysis::{AudioAnalysis, Opts as AudioOpts},
    AnalyzerParams, AudioFeatures,
};
use crate::config::{Config, OptionalConfig};
use crate::visualizer::cpurender::Params as RenderParams;
#[cfg(feature = "ledpanel")]
use crate::visualizer::ledpanel::{Options as LedPanelOptions, RenderToPanel};
#[cfg(feature = "gpu")]
use crate::visualizer::warpgrid::WarpGridRender;

struct Init {
    audio_opts: AudioOpts,
    config: Config,
}

impl SimpleState for Init {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        data.resources.insert(AppSystemConfig::new());
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
    selfconfig_update: SyncSender<AppSystemConfig>,
}

use crate::api::{ApiServer, AudioMessage};

impl App {
    pub(crate) fn new(config: Config, audio_opts: AudioOpts, verbose: i32) -> Self {
        let (config_update, config_mailbox) = sync_channel(1);
        let (selfconfig_update, selfconfig_mailbox) = sync_channel(1);
        // let (audio_update, audio_mailbox) = sync_channel(1);

        let app_system = AppSystem {
            config_mailbox,
            selfconfig_mailbox,
        };

        std::thread::spawn(move || {
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
            selfconfig_update,
        }
    }
}

impl Actor for App {
    type Context = Context<Self>;
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

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct AppConfigMessage(pub AppSystemConfig);

impl Handler<AppConfigMessage> for App {
    type Result = ();

    fn handle(&mut self, config: AppConfigMessage, _: &mut Self::Context) {
        self.selfconfig_update.send(config.0);
    }
}

struct AppSystem {
    config_mailbox: Receiver<OptionalConfig>,
    selfconfig_mailbox: Receiver<AppSystemConfig>,
}

pub(crate) struct AppSystemConfig {
    pub subscription_enabled: Option<bool>,
    pub apiserver: Option<Addr<ApiServer>>,
}

impl AppSystemConfig {
    fn new() -> Self {
        Self {
            subscription_enabled: None,
            apiserver: None,
        }
    }
}

impl ThreadLocalSystem<'_> for AppSystem {
    fn build(self) -> Box<dyn Runnable> {
        let builder = SystemBuilder::new("app system")
            .write_resource::<AppSystemConfig>()
            .write_resource::<Option<AnalyzerParams>>()
            .write_resource::<RenderParams>()
            .read_resource::<AudioFeatures>();

        #[cfg(feature = "ledpanel")]
        builder.write_resource::<LedPanelOptions>();

        Box::new(builder.build(move |_commands, _world, resources, _query| {
            match self.config_mailbox.try_recv() {
                Err(TryRecvError::Empty) => (),
                Ok(config) => {
                    if let Some(ap) = config.audio {
                        debug!("updated audio params: {:?}", ap);
                        resources.1.replace(ap);
                    }
                    if let Some(rp) = config.render {
                        debug!("updated render params: {:?}", rp);
                        *resources.2 = rp;
                    }
                    // FIXME: this will be a mess as soon as there are multiple options
                    // maybe options can always be in the struct but have no effect unless enabled
                    #[cfg(feature = "ledpanel")]
                    if let Some(lp) = config.panel {
                        debug!("updated panel config: {:?}", lp);
                        *resources.4 = lp;
                    }
                }
                Err(e) => error!("error recv on config_mailbox: {}", e),
            }

            match self.selfconfig_mailbox.try_recv() {
                Err(TryRecvError::Empty) => (),
                Ok(config) => {
                    if let Some(addr) = config.apiserver {
                        debug!("updated apiserver addr");
                        resources.0.apiserver.replace(addr);
                    }
                    if let Some(en) = config.subscription_enabled {
                        debug!("set audio subs enable: {}", en);
                        resources.0.subscription_enabled.replace(en);
                    }
                }
                Err(e) => error!("error recv on selfconfig_mailbox: {}", e),
            }

            if resources.0.subscription_enabled.unwrap_or(false) {
                if let Some(apiserver) = &resources.0.apiserver {
                    if let Err(e) = apiserver.try_send(AudioMessage(resources.3.clone())) {
                        error!("failed to send back AudioMessage: {}", e);
                    }
                }
            }
        }))
    }
}
