use crate::wrap::WrapF64Unit;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host, OutputCallbackInfo, Stream, StreamConfig,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{atomic::AtomicU64, mpsc, Arc, RwLock},
};

pub struct Synth {
    host: Host,
    device: Device,
    config: StreamConfig,
    stream: Stream,
    sender: mpsc::Sender<f32>,
    sink_cursor: Arc<RwLock<u64>>,
    source_cursor: u64,
    target_padding: u64,
}

impl Synth {
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
        Ok(Self {
            target_padding: (config.sample_rate.0 as u64) / 20,
            host,
            device,
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

pub trait Signal<T> {
    fn sample(&mut self, i: u64) -> T;
}

pub struct Variable<T> {
    value: Rc<RefCell<T>>,
}

impl<T> Variable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
        }
    }

    pub fn set(&self, value: T) {
        *self.value.borrow_mut() = value;
    }

    pub fn shallow_clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
        }
    }
}

impl<T: Copy> Signal<T> for Variable<T> {
    fn sample(&mut self, _: u64) -> T {
        *self.value.borrow()
    }
}

pub struct Const<T> {
    value: T,
}

impl<T> Const<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Copy> Signal<T> for Const<T> {
    fn sample(&mut self, _: u64) -> T {
        self.value
    }
}

pub struct SquareWaveOscillatorBuilder<T, FS: Signal<f64>, PWS: Signal<f64>> {
    pub high: T,
    pub low: T,
    pub frequency_hz_signal: FS,
    pub pulse_width_01_signal: PWS,
    pub sample_rate: u32,
}

impl<T, FS: Signal<f64>, PWS: Signal<f64>> SquareWaveOscillatorBuilder<T, FS, PWS> {
    pub fn build(self) -> SquareWaveOscillator<T, FS, PWS> {
        SquareWaveOscillator::new(self)
    }
}

pub struct SquareWaveOscillator<T, FS: Signal<f64>, PWS: Signal<f64>> {
    high: T,
    low: T,
    frequency_hz_signal: FS,
    pulse_width_01_signal: PWS,
    sample_rate: u32,
    state: WrapF64Unit,
}

impl<T, FS: Signal<f64>, PWS: Signal<f64>> SquareWaveOscillator<T, FS, PWS> {
    pub fn new(
        SquareWaveOscillatorBuilder {
            high,
            low,
            frequency_hz_signal,
            pulse_width_01_signal,
            sample_rate,
        }: SquareWaveOscillatorBuilder<T, FS, PWS>,
    ) -> Self {
        Self {
            high,
            low,
            frequency_hz_signal,
            pulse_width_01_signal,
            sample_rate,
            state: 0f64.into(),
        }
    }
}

impl<T: Copy, FS: Signal<f64>, PWS: Signal<f64>> Signal<T> for SquareWaveOscillator<T, FS, PWS> {
    fn sample(&mut self, i: u64) -> T {
        self.state += self.frequency_hz_signal.sample(i) / self.sample_rate as f64;
        if self.state.value() < self.pulse_width_01_signal.sample(i) {
            self.high
        } else {
            self.low
        }
    }
}
