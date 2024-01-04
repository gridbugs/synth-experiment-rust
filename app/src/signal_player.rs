use cpal_sample_player::SamplePlayer;
use std::mem;
use synth_language::{BufferedSignal, SignalCtx};

pub struct SignalPlayer {
    sample_player: SamplePlayer<f32>,
    sample_index: u64,
    recent_samples: Vec<f32>,
}

impl SignalPlayer {
    pub fn new(downsample: u32) -> anyhow::Result<Self> {
        Ok(Self {
            sample_player: SamplePlayer::new_with_downsample(downsample)?,
            sample_index: 0,
            recent_samples: Default::default(),
        })
    }

    pub fn send_signal(&mut self, buffered_signal: &mut BufferedSignal<f32>) {
        self.recent_samples.clear();
        let sample_rate = self.sample_player.sample_rate();
        self.sample_player.play_stream(|| {
            let ctx = SignalCtx {
                sample_index: self.sample_index,
                sample_rate,
            };
            let sample = buffered_signal.sample(&ctx);
            self.recent_samples.push(sample);
            self.sample_index += 1;
            sample
        });
    }

    pub fn swap_recent_samples(&mut self, buffer: &mut Vec<f32>) {
        buffer.clear();
        mem::swap(&mut self.recent_samples, buffer)
    }
}
