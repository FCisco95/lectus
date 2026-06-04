/// Root-mean-square amplitude of a sample buffer — a cheap proxy for loudness.
///
/// Reused both by the VAD below and by the live audio-level meter that drives
/// the recording pill. Returns 0.0 for an empty buffer.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

pub struct EnergyVad {
    threshold: f32,
    silence_frame_count: u32,
    silence_frames_required: u32,
    was_speaking: bool,
}

impl EnergyVad {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            silence_frame_count: 0,
            silence_frames_required: 30,
            was_speaking: false,
        }
    }

    /// Returns true if this frame contains speech
    pub fn is_speech(&mut self, samples: &[f32]) -> bool {
        let speaking = rms(samples) > self.threshold;
        if speaking {
            self.silence_frame_count = 0;
            self.was_speaking = true;
        } else {
            self.silence_frame_count += 1;
        }
        speaking
    }

    /// Returns true once — when speech has just ended (speaking → sustained silence)
    pub fn speech_ended(&mut self, samples: &[f32]) -> bool {
        self.is_speech(samples);
        if self.was_speaking && self.silence_frame_count >= self.silence_frames_required {
            self.was_speaking = false;
            self.silence_frame_count = 0;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence(n: usize) -> Vec<f32> {
        vec![0.0f32; n]
    }

    fn loud(n: usize, amp: f32) -> Vec<f32> {
        (0..n).map(|i| amp * ((i as f32 * 0.1).sin())).collect()
    }

    #[test]
    fn test_silence_not_speech() {
        let mut vad = EnergyVad::new(0.01);
        assert!(!vad.is_speech(&silence(512)));
    }

    #[test]
    fn test_loud_audio_is_speech() {
        let mut vad = EnergyVad::new(0.01);
        assert!(vad.is_speech(&loud(512, 0.5)));
    }

    #[test]
    fn test_speech_end_after_silence_frames() {
        let mut vad = EnergyVad::new(0.01);
        // Prime with speech
        for _ in 0..5 {
            vad.is_speech(&loud(512, 0.5));
        }
        // Then pass silence frames until speech_ended returns true
        let ended = (0..40).any(|_| vad.speech_ended(&silence(512)));
        assert!(ended, "should detect speech end after ~30 silent frames");
    }
}
