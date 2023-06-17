use crate::wrap::WrapF64Unit;
use std::{cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc};

pub trait Signal<T> {
    fn sample(&mut self, i: u64) -> T;
}

pub struct Map<T, S, F> {
    t: PhantomData<T>,
    signal: S,
    f: F,
}

impl<T, U, S, F> Signal<U> for Map<T, S, F>
where
    S: Signal<T>,
    F: FnMut(T) -> U,
{
    fn sample(&mut self, i: u64) -> U {
        (self.f)(self.signal.sample(i))
    }
}

pub fn map<T, U, S: Signal<T>, F: FnMut(T) -> U>(signal: S, f: F) -> Map<T, S, F> {
    Map {
        t: PhantomData,
        signal,
        f,
    }
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

pub struct SawWaveOsillatorBuilder<FS: Signal<f64>> {
    pub high: f32,
    pub low: f32,
    pub frequency_hz_signal: FS,
    pub sample_rate: u32,
}

impl<FS: Signal<f64>> SawWaveOsillatorBuilder<FS> {
    pub fn build(self) -> SawWaveOsillator<FS> {
        SawWaveOsillator::new(self)
    }
}

pub struct SawWaveOsillator<FS: Signal<f64>> {
    high: f32,
    low: f32,
    frequency_hz_signal: FS,
    sample_rate: u32,
    state: WrapF64Unit,
}

impl<FS: Signal<f64>> SawWaveOsillator<FS> {
    pub fn new(
        SawWaveOsillatorBuilder {
            high,
            low,
            frequency_hz_signal,
            sample_rate,
        }: SawWaveOsillatorBuilder<FS>,
    ) -> Self {
        Self {
            high,
            low,
            frequency_hz_signal,
            sample_rate,
            state: 0f64.into(),
        }
    }
}

impl<FS: Signal<f64>> Signal<f32> for SawWaveOsillator<FS> {
    fn sample(&mut self, i: u64) -> f32 {
        self.state += self.frequency_hz_signal.sample(i) / self.sample_rate as f64;
        self.low + ((self.high - self.low) * self.state.value() as f32)
    }
}

pub struct MovingAverageFilterBuilder<S: Signal<f32>, W: Signal<u32>> {
    pub signal: S,
    pub width: W,
}

impl<S: Signal<f32>, W: Signal<u32>> MovingAverageFilterBuilder<S, W> {
    pub fn build(self) -> MovingAverageFilter<S, W> {
        MovingAverageFilter::new(self)
    }
}

pub struct MovingAverageFilter<S: Signal<f32>, W: Signal<u32>> {
    signal: S,
    width: W,
    buffer: VecDeque<f32>,
}

impl<S: Signal<f32>, W: Signal<u32>> MovingAverageFilter<S, W> {
    pub fn new(
        MovingAverageFilterBuilder { signal, width }: MovingAverageFilterBuilder<S, W>,
    ) -> Self {
        Self {
            signal,
            width,
            buffer: Default::default(),
        }
    }
}

impl<S: Signal<f32>, W: Signal<u32>> Signal<f32> for MovingAverageFilter<S, W> {
    fn sample(&mut self, i: u64) -> f32 {
        let width = self.width.sample(i) as usize;
        let current_sample = self.signal.sample(i);
        while self.buffer.len() >= width {
            self.buffer.pop_front();
        }
        self.buffer.push_back(current_sample);
        let sum = self.buffer.iter().sum::<f32>();
        sum / self.buffer.len() as f32
    }
}

enum AdsrPosition {
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct AdsrConfiguration {
    pub attack_seconds: f64,
    pub decay_seconds: f64,
    pub sustain_level_01: f64,
    pub release_seconds: f64,
}

pub struct LinearAdsrEnvelopeGenerator01<G: Signal<bool>> {
    gate: G,
    gate_prev: bool,
    configuration: AdsrConfiguration,
    position: AdsrPosition,
}
