use amethyst::{core::dispatcher::ThreadLocalSystem, ecs::*};

use super::{Params, cpurender::Visualizer};
use crate::audiosys::AudioFeatures;
use panel_driver::Panel;
pub use panel_driver::Options;

pub struct RenderToPanel {
    vis: Visualizer,
    panel: Panel,
}

impl RenderToPanel {
    pub fn new(verbose: i32, options: Options) -> Self {
        let panel = Panel::new(verbose, options);
        let vis = Visualizer::new(192, 64, verbose);
        Self { vis, panel }
    }
}

impl ThreadLocalSystem<'static> for RenderToPanel {
    fn build(self) -> Box<dyn Runnable> {
        Box::new(
            SystemBuilder::new("led panel renderer")
                .read_resource::<Params>()
                .read_resource::<AudioFeatures>()
                .build(move |_commands, _world, (params, features), _query| {
                    let image = self.vis.render(params, features);
                    if let Err(e) = self.panel.send_frame(image) {
                        log::error!("failed to send frame: {}", e);
                    }
                }),
        )
    }
}

/*
pub struct LedPanelBundle {
    options: Option<LedMatrixOptions>,
    rt_options: Option<LedRuntimeOptions>,
    verbose: i32,
}

impl LedPanelBundle {
    pub fn new(
        options: Option<LedMatrixOptions>,
        rt_options: Option<LedRuntimeOptions>,
        verbose: i32,
    ) -> Self {
        Self {
            options,
            rt_options,
            verbose,
        }
    }
}

impl SystemBundle for LedPanelBundle {
    fn load(
        &mut self,
        _world: &mut World,
        resources: &mut Resources,
        builder: &mut DispatcherBuilder,
    ) -> Result<(), Error> {
        println!("ADD RNDERE@@@@@@@@@@@");

        let mut options = LedMatrixOptions::default();
        options.set_cols(64);
        options.set_rows(32);
        options.set_chain_length(3);
        options.set_parallel(2);
        options.set_hardware_mapping("vuzic");
        options.set_limit_refresh(240);
        let mut rt_options = LedRuntimeOptions::default();
        rt_options.set_gpio_slowdown(3);
        let matrix =
            LedMatrix::new(Some(options), Some(rt_options)).expect("failed to create matrix");

        println!("@@@ inserted ledmatrix!");

        let params = Params::defaults();
        let vis = Visualizer::new(192, 64, params, self.verbose);
        let render = RenderToPanel::new(0);
        builder.add_thread_local(Box::new(render));

        Ok(())
    }
}
*/
