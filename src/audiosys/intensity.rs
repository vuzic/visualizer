use amethyst::{core::math::Vector3, core::transform::Transform, ecs::SystemBuilder, prelude::*};

use super::AudioFeatures;

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

pub struct AudioIntensityDebugSystem;

impl System for AudioIntensityDebugSystem {
    fn build(self) -> Box<dyn ParallelRunnable> {
        Box::new(
            SystemBuilder::new("AudioIntensity")
                .with_query(<(&AudioIntensity,)>::query())
                .read_resource::<Box<AudioFeatures>>()
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

pub struct AudioIntensityScaleSystem;

impl System for AudioIntensityScaleSystem {
    fn build(self) -> Box<dyn ParallelRunnable> {
        Box::new(
            SystemBuilder::new("AudioIntensity")
                .with_query(<(&AudioIntensity, &mut Transform)>::query())
                .read_resource::<Box<AudioFeatures>>()
                .build(move |_commands, world, features, audio_intensity_query| {
                    for (a, trans) in audio_intensity_query.iter_mut(world) {
                        let amp = features.get_amplitudes(a.frame)[a.bucket];
                        trans.set_scale(Vector3::new(amp as f32, amp as f32, amp as f32));
                    }
                }),
        )
    }
}
