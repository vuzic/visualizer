use audio::frequency_sensor::Features as AudioFeatures;
use specs::{Read, ReadStorage, System};

use super::component::AudioIntensity;

pub struct AudioSystem;

impl<'s> System<'s> for AudioSystem {
    type SystemData = (Read<'s, AudioFeatures>, ReadStorage<'s, AudioIntensity>);

    fn run(&mut self, (features, audio_intensity): Self::SystemData) {
        use specs::Join;
        for audio_intensity in audio_intensity.join() {
            let &AudioIntensity { frame, bucket } = audio_intensity;
            let amp = features.get_amplitudes(frame)[bucket];
            println!(
                "Audio Intensity (Frame={}, Bucket={:2}): {:.2}",
                frame, bucket, amp
            );
        }
    }
}
