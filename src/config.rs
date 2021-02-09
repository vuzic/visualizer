use serde::{Deserialize, Serialize};

use crate::audiosys::AnalyzerParams;
use crate::visualizer::Params as RenderParams;
#[cfg(feature = "ledpanel")]
use crate::visualizer::ledpanel::Options as LedPanelOptions;

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub(crate) struct Config {
    pub audio: AnalyzerParams,
    pub render: RenderParams,
    #[cfg(feature = "ledpanel")]
    pub panel: LedPanelOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: Default::default(),
            render: Default::default(),
            #[cfg(feature = "ledpanel")]
            panel: Default::default(),
        }
    }
}

// TODO: make a derive macro
#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub(crate) struct OptionalConfig {
    pub audio: Option<AnalyzerParams>,
    pub render: Option<RenderParams>,
    #[cfg(feature = "ledpanel")]
    pub panel: Option<LedPanelOptions>,
}
