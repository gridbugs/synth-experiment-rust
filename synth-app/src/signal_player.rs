use cpal_sample_player::SamplePlayer;
use synth_language::{BufferedSignal, SignalCtx};

pub struct SignalPlayer {
    sample_player: SamplePlayer<f32>,
    sample_index: u64,
}

impl SignalPlayer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            sample_player: SamplePlayer::new()?,
            sample_index: 0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_player.sample_rate()
    }

    pub fn send_signal(&mut self, buffered_signal: &mut BufferedSignal<f32>) {
        let sample_rate = self.sample_rate();
        self.sample_player.play_stream(|| {
            let ctx = SignalCtx {
                sample_index: self.sample_index,
                sample_rate,
            };
            let sample = buffered_signal.sample(&ctx);
            self.sample_index += 1;
            sample
        });
    }
}
