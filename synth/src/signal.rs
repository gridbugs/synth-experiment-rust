use crate::wrap::{WrapF64MinusOneToOne, WrapF64Radians, WrapF64Unit};
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

pub struct Amplifier<S: Signal<f64>, V: Signal<f64>> {
    pub signal: S,
    pub volume: V,
}

impl<S: Signal<f64>, V: Signal<f64>> Signal<f64> for Amplifier<S, V> {
    fn sample(&mut self, i: u64) -> f64 {
        self.signal.sample(i) * self.volume.sample(i) as f64
    }
}

pub struct Mixer<A: Signal<f64>, B: Signal<f64>> {
    pub a: A,
    pub b: B,
}

impl<A: Signal<f64>, B: Signal<f64>> Signal<f64> for Mixer<A, B> {
    fn sample(&mut self, i: u64) -> f64 {
        self.a.sample(i) + self.b.sample(i)
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
    pub frequency_hz_signal: FS,
    pub sample_rate: u32,
}

impl<FS: Signal<f64>> SawWaveOsillatorBuilder<FS> {
    pub fn build(self) -> SawWaveOsillator<FS> {
        SawWaveOsillator::new(self)
    }
}

pub struct SawWaveOsillator<FS: Signal<f64>> {
    frequency_hz_signal: FS,
    sample_rate: u32,
    state: WrapF64MinusOneToOne,
}

impl<FS: Signal<f64>> SawWaveOsillator<FS> {
    pub fn new(
        SawWaveOsillatorBuilder {
            frequency_hz_signal,
            sample_rate,
        }: SawWaveOsillatorBuilder<FS>,
    ) -> Self {
        Self {
            frequency_hz_signal,
            sample_rate,
            state: 0f64.into(),
        }
    }
}

impl<FS: Signal<f64>> Signal<f64> for SawWaveOsillator<FS> {
    fn sample(&mut self, i: u64) -> f64 {
        self.state += (self.frequency_hz_signal.sample(i) * WrapF64MinusOneToOne::DELTA)
            / self.sample_rate as f64;
        self.state.value() as f64
    }
}

pub struct SineWaveOsillatorBuilder<FS: Signal<f64>> {
    pub frequency_hz_signal: FS,
    pub sample_rate: u32,
}

impl<FS: Signal<f64>> SineWaveOsillatorBuilder<FS> {
    pub fn build(self) -> SineWaveOsillator<FS> {
        SineWaveOsillator::new(self)
    }
}

pub struct SineWaveOsillator<FS: Signal<f64>> {
    frequency_hz_signal: FS,
    sample_rate: u32,
    state: WrapF64Radians,
}

impl<FS: Signal<f64>> SineWaveOsillator<FS> {
    pub fn new(
        SineWaveOsillatorBuilder {
            frequency_hz_signal,
            sample_rate,
        }: SineWaveOsillatorBuilder<FS>,
    ) -> Self {
        Self {
            frequency_hz_signal,
            sample_rate,
            state: 0f64.into(),
        }
    }
}

impl<FS: Signal<f64>> Signal<f64> for SineWaveOsillator<FS> {
    fn sample(&mut self, i: u64) -> f64 {
        self.state +=
            (self.frequency_hz_signal.sample(i) * WrapF64Radians::DELTA) / self.sample_rate as f64;
        (self.state.value() as f64).sin()
    }
}

pub struct MovingAverageFilterBuilder<S: Signal<f64>, W: Signal<u32>> {
    pub signal: S,
    pub width: W,
}

impl<S: Signal<f64>, W: Signal<u32>> MovingAverageFilterBuilder<S, W> {
    pub fn build(self) -> MovingAverageFilter<S, W> {
        MovingAverageFilter::new(self)
    }
}

pub struct MovingAverageFilter<S: Signal<f64>, W: Signal<u32>> {
    signal: S,
    width: W,
    buffer: VecDeque<f64>,
}

impl<S: Signal<f64>, W: Signal<u32>> MovingAverageFilter<S, W> {
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

impl<S: Signal<f64>, W: Signal<u32>> Signal<f64> for MovingAverageFilter<S, W> {
    fn sample(&mut self, i: u64) -> f64 {
        let width = self.width.sample(i) as usize;
        let current_sample = self.signal.sample(i);
        while self.buffer.len() >= width {
            self.buffer.pop_front();
        }
        self.buffer.push_back(current_sample);
        let sum = self.buffer.iter().sum::<f64>();
        sum / self.buffer.len() as f64
    }
}

#[derive(Debug, Clone, Copy)]
enum AdsrPosition {
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone, Copy)]
pub struct AdsrConfiguration {
    pub attack_seconds: f64,
    pub decay_seconds: f64,
    pub sustain_level_01: f64,
    pub release_seconds: f64,
}

pub struct LinearAdsrEnvelopeGenerator01Builder<G: Signal<bool>> {
    pub gate: G,
    pub configuration: AdsrConfiguration,
    pub sample_rate: u32,
}

impl<G: Signal<bool>> LinearAdsrEnvelopeGenerator01Builder<G> {
    pub fn build(self) -> LinearAdsrEnvelopeGenerator01<G> {
        LinearAdsrEnvelopeGenerator01::new(self)
    }
}

pub struct LinearAdsrEnvelopeGenerator01<G: Signal<bool>> {
    gate: G,
    configuration: AdsrConfiguration,
    position: AdsrPosition,
    sample_rate: u32,
    state: f64,
}

impl<G: Signal<bool>> LinearAdsrEnvelopeGenerator01<G> {
    pub fn new(
        LinearAdsrEnvelopeGenerator01Builder {
            gate,
            configuration,
            sample_rate,
        }: LinearAdsrEnvelopeGenerator01Builder<G>,
    ) -> Self {
        Self {
            gate,
            configuration,
            position: AdsrPosition::Release,
            sample_rate,
            state: 0f64,
        }
    }
}

// iI will take `seconds` seconds to increase a value from 0 to 1 by adding the result of this
// function applied once per sample.
fn seconds_to_step_size_01(seconds: f64, sample_rate: u32) -> f64 {
    1_f64 / (sample_rate as f64 * seconds)
}

impl<G: Signal<bool>> Signal<f64> for LinearAdsrEnvelopeGenerator01<G> {
    fn sample(&mut self, i: u64) -> f64 {
        let gate_current = self.gate.sample(i);
        if !gate_current {
            self.position = AdsrPosition::Release;
        } else if let AdsrPosition::Release = self.position {
            self.position = AdsrPosition::Attack;
        }
        let (position, state) = match self.position {
            AdsrPosition::Attack => {
                let state = self.state
                    + seconds_to_step_size_01(self.configuration.attack_seconds, self.sample_rate);
                if state >= 1f64 {
                    (AdsrPosition::Decay, 1f64)
                } else {
                    (AdsrPosition::Attack, state)
                }
            }
            AdsrPosition::Decay => {
                let state = self.state
                    - seconds_to_step_size_01(self.configuration.decay_seconds, self.sample_rate);
                if state <= self.configuration.sustain_level_01 {
                    (AdsrPosition::Sustain, self.configuration.sustain_level_01)
                } else {
                    (AdsrPosition::Decay, state)
                }
            }
            AdsrPosition::Sustain => (AdsrPosition::Sustain, self.state),
            AdsrPosition::Release => {
                let state = self.state
                    - seconds_to_step_size_01(self.configuration.release_seconds, self.sample_rate);
                if state <= 0f64 {
                    (AdsrPosition::Release, 0f64)
                } else {
                    (AdsrPosition::Release, state)
                }
            }
        };
        self.position = position;
        self.state = state;
        state
    }
}
