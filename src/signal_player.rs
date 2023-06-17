use crate::signal::Signal;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    OutputCallbackInfo, Stream, StreamConfig,
};
use std::sync::{mpsc, Arc, RwLock};

pub struct SignalPlayer {
    config: StreamConfig,
    #[allow(unused)]
    stream: Stream,
    sender: mpsc::Sender<f32>,
    sink_cursor: Arc<RwLock<u64>>,
    source_cursor: u64,
    target_padding: u64,
}

impl SignalPlayer {
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        log::info!("cpal host: {}", host.id().name());
        let device = host
            .default_output_device()
            .ok_or(anyhow::anyhow!("no output device"))?;
        if let Ok(name) = device.name() {
            log::info!("cpal device: {}", name);
        } else {
            log::info!("cpal device: (no name)");
        }
        let config = device.default_output_config()?;
        log::info!("sample format: {}", config.sample_format());
        log::info!("sample rate: {}", config.sample_rate().0);
        let config = StreamConfig::from(config);
        let (sender, receiver) = mpsc::channel::<f32>();
        let sink_cursor = Arc::new(RwLock::new(0));
        let sink_cursor_for_cpal_thread = Arc::clone(&sink_cursor);
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                let mut count = 0;
                for output in data.iter_mut() {
                    if let Ok(input) = receiver.try_recv() {
                        *output = input;
                        count += 1;
                    } else {
                        break;
                    }
                }
                *sink_cursor_for_cpal_thread.write().unwrap() += count;
            },
            |err| log::error!("stream error: {}", err),
            None,
        )?;
        // This trades off latency against always having a sample available to send to the
        // dhevice. In the future this could be dynamically chosen as slower machines require a
        // larger amount of padding.
        let target_padding = config.sample_rate.0 as u64 / 20;
        Ok(Self {
            target_padding,
            config,
            stream,
            sender,
            sink_cursor,
            source_cursor: 0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    fn send_single_sample<S: Signal<f32> + ?Sized>(&mut self, signal: &mut S) {
        if let Err(_) = self.sender.send(signal.sample(self.source_cursor)) {
            log::error!("failed to send data to cpal thread");
        }
        self.source_cursor += 1;
    }

    pub fn send_signal<S: Signal<f32> + ?Sized>(&mut self, signal: &mut S) {
        let sink_cursor = *self.sink_cursor.read().unwrap();
        let target_source_cursor = sink_cursor + self.target_padding;
        while self.source_cursor < target_source_cursor {
            self.send_single_sample(signal);
        }
    }
}
