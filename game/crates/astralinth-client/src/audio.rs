use std::sync::mpsc::{channel, Sender};
use std::thread;

#[derive(Debug, Clone, Copy)]
pub enum SoundEffect {
    Footstep,
    BlockBreak,
    BlockPlace,
    Jump,
    Chime,
}

pub struct AudioEngine {
    sender: Sender<SoundEffect>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (sender, receiver) = channel::<SoundEffect>();

        // Background audio worker thread: receives sound trigger events
        thread::spawn(move || {
            while let Ok(effect) = receiver.recv() {
                // Log and process sound effect triggers
                log::debug!("Playing audio effect: {:?}", effect);
            }
        });

        Self { sender }
    }

    pub fn play(&self, effect: SoundEffect) {
        let _ = self.sender.send(effect);
    }
}

/// Generates raw PCM wave samples for an original procedural sound effect.
pub fn generate_pcm_sound(effect: SoundEffect, sample_rate: u32) -> Vec<i16> {
    let mut samples = Vec::new();
    match effect {
        SoundEffect::Footstep => {
            let duration = (sample_rate as f32 * 0.08) as usize; // 80 ms
            for i in 0..duration {
                let t = i as f32 / sample_rate as f32;
                let env = 1.0 - (i as f32 / duration as f32);
                let tone = (t * 140.0 * std::f32::consts::TAU).sin();
                let noise = ((i * 1103515245 + 12345) % 32767) as f32 / 32767.0 - 0.5;
                let s = ((tone * 0.4 + noise * 0.6) * env * 16000.0) as i16;
                samples.push(s);
            }
        }
        SoundEffect::BlockBreak => {
            let duration = (sample_rate as f32 * 0.14) as usize; // 140 ms
            for i in 0..duration {
                let env = (1.0 - (i as f32 / duration as f32)).powf(1.8);
                let noise = ((i * 214013 + 2531011) % 32767) as f32 / 32767.0 - 0.5;
                let s = (noise * env * 24000.0) as i16;
                samples.push(s);
            }
        }
        SoundEffect::BlockPlace => {
            let duration = (sample_rate as f32 * 0.06) as usize; // 60 ms
            for i in 0..duration {
                let t = i as f32 / sample_rate as f32;
                let env = 1.0 - (i as f32 / duration as f32);
                let freq = 160.0 - (i as f32 / duration as f32) * 80.0;
                let tone = (t * freq * std::f32::consts::TAU).sin();
                let s = (tone * env * 20000.0) as i16;
                samples.push(s);
            }
        }
        SoundEffect::Jump => {
            let duration = (sample_rate as f32 * 0.12) as usize; // 120 ms
            for i in 0..duration {
                let t = i as f32 / sample_rate as f32;
                let env = 1.0 - (i as f32 / duration as f32);
                let freq = 180.0 + (i as f32 / duration as f32) * 160.0;
                let tone = (t * freq * std::f32::consts::TAU).sin();
                let s = (tone * env * 18000.0) as i16;
                samples.push(s);
            }
        }
        SoundEffect::Chime => {
            let duration = (sample_rate as f32 * 0.4) as usize; // 400 ms
            for i in 0..duration {
                let t = i as f32 / sample_rate as f32;
                let env = (1.0 - (i as f32 / duration as f32)).powf(2.0);
                let tone1 = (t * 880.0 * std::f32::consts::TAU).sin();
                let tone2 = (t * 1760.0 * std::f32::consts::TAU).sin();
                let s = ((tone1 * 0.6 + tone2 * 0.4) * env * 18000.0) as i16;
                samples.push(s);
            }
        }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcm_audio_generation() {
        let step = generate_pcm_sound(SoundEffect::Footstep, 44100);
        let brk = generate_pcm_sound(SoundEffect::BlockBreak, 44100);
        let place = generate_pcm_sound(SoundEffect::BlockPlace, 44100);
        let jump = generate_pcm_sound(SoundEffect::Jump, 44100);
        let chime = generate_pcm_sound(SoundEffect::Chime, 44100);

        assert!(!step.is_empty());
        assert!(!brk.is_empty());
        assert!(!place.is_empty());
        assert!(!jump.is_empty());
        assert!(!chime.is_empty());
    }
}
