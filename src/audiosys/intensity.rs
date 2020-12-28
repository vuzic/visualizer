use legion::*;

use super::analysis::AudioFeatures;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioIntensity {
    pub frame: usize,
    pub bucket: usize,
}

#[system(for_each)]
pub fn update_audio_intensity(a: &AudioIntensity, #[resource] features: &AudioFeatures) {
    let amp = features.get_amplitudes(a.frame)[a.bucket];
    println!(
        "Audio Intensity (Frame={}, Bucket={:2}): {:.2}",
        a.frame, a.bucket, amp
    );
}
