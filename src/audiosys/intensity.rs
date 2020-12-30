use amethyst::{ecs::SystemBuilder, prelude::*};

use super::analysis::AudioFeatures;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioIntensity {
    pub frame: usize,
    pub bucket: usize,
}

// #[system(for_each)]
// pub fn update_audio_intensity(a: &AudioIntensity, #[resource] features: &AudioFeatures) {
//     let amp = features.get_amplitudes(a.frame)[a.bucket];
//     println!(
//         "Audio Intensity (Frame={}, Bucket={:2}): {:.2}",
//         a.frame, a.bucket, amp
//     );
// }

pub struct AudioIntensitySystem;

impl System<'_> for AudioIntensitySystem {
    fn build(&'_ mut self) -> Box<dyn ParallelRunnable> {
        Box::new(
            SystemBuilder::new("AudioIntensity")
                .with_query(<(&AudioIntensity,)>::query())
                .read_resource::<AudioFeatures>()
                .build(move |_commands, world, features, audio_intensity_query| {
                    for (a,) in audio_intensity_query.iter(world) {
                        let amp = features.get_amplitudes(a.frame)[a.bucket];
                        println!(
                            "Audio Intensity (Frame={}, Bucket={:2}): {:.2}",
                            a.frame, a.bucket, amp
                        );
                    }
                }),
        )
    }
}
