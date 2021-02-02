pub mod analysis;
pub mod intensity;

pub use audio::analyzer::{AnalyzerParams, AnalyzerState};
pub use audio::frequency_sensor::{
    Features as AudioFeatures, FrequencySensorParams as AudioParams,
};
pub use audio::gain_control::Params as GainParams;
