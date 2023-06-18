use crate::signal::Signal;
use cpal_sample_player::SamplePlayer;

pub struct SignalPlayer {
    sample_player: SamplePlayer<f32>,
    sample_index: u64,
}

impl SignalPlayer {
    pub fn new() -> anyhow::Result<Self> {
        let sample_player = SamplePlayer::new()?;
        Ok(Self {
            sample_player,
            sample_index: 0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_player.sample_rate()
    }

    pub fn send_signal<S: Signal<f32> + ?Sized>(&mut self, signal: &mut S) {
        self.sample_player.play_stream(|| {
            let sample = signal.sample(self.sample_index);
            self.sample_index += 1;
            sample
        });
    }
}
