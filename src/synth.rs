use crate::wrap::WrapF64Unit;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host, SupportedStreamConfig,
};

pub struct Synth {
    host: Host,
    device: Device,
    config: SupportedStreamConfig,
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
        Ok(Self {
            host,
            device,
            config,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate().0
    }
}

pub trait Signal<T> {
    fn sample(&mut self, i: u64) -> T;
}

pub struct Variable<T> {
    value: T,
}

impl<T: Copy> Signal<T> for Variable<T> {
    fn sample(&mut self, _: u64) -> T {
        self.value
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
