use crate::signal::*;
use crate::wrap::{WrapF64MinusOneToOne, WrapF64Radians, WrapF64Unit};
use std::{cell::RefCell, collections::VecDeque, ops::DerefMut, rc::Rc};

pub struct Const<T>(T);

impl<T> Const<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Clone> SignalTrait<T> for Const<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.0.clone()
    }
}

impl<T: Clone + 'static> From<Const<T>> for BufferedSignal<T> {
    fn from(value: Const<T>) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct Var<T>(Rc<RefCell<T>>);

impl<T: Clone> Var<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    pub fn clone_ref(&self) -> Self {
        Var(Rc::clone(&self.0))
    }

    pub fn get(&self) -> T {
        self.0.borrow().clone()
    }

    pub fn set(&self, value: T) {
        *self.0.borrow_mut().deref_mut() = value;
    }
}

impl<T: Clone> SignalTrait<T> for Var<T> {
    fn sample(&mut self, _ctx: &SignalCtx) -> T {
        self.get()
    }
}

impl<T: Clone + 'static> From<Var<T>> for BufferedSignal<T> {
    fn from(value: Var<T>) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct SineOscillator {
    pub frequency_hz: BufferedSignal<f64>,
}

struct SineOscillatorSignal {
    props: SineOscillator,
    state: WrapF64Radians,
}

impl SineOscillatorSignal {
    fn new(props: SineOscillator) -> Self {
        Self {
            props,
            state: 0f64.into(),
        }
    }
}

impl SignalTrait<f64> for SineOscillatorSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.state +=
            (self.props.frequency_hz.sample(ctx) * WrapF64Radians::DELTA) / ctx.sample_rate as f64;
        (self.state.value() as f64).sin()
    }
}

impl From<SineOscillator> for BufferedSignal<f64> {
    fn from(value: SineOscillator) -> Self {
        BufferedSignal::new(SineOscillatorSignal::new(value))
    }
}

pub struct SquareOscillator {
    pub frequency_hz: BufferedSignal<f64>,
    pub pulse_width_01: BufferedSignal<f64>,
}

struct SquareOscillatorSignal {
    props: SquareOscillator,
    state: WrapF64Unit,
}

impl SquareOscillatorSignal {
    fn new(props: SquareOscillator) -> Self {
        Self {
            props,
            state: 0f64.into(),
        }
    }
}

impl SignalTrait<f64> for SquareOscillatorSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.state += self.props.frequency_hz.sample(ctx) / ctx.sample_rate as f64;
        if self.state.value() < self.props.pulse_width_01.sample(ctx) {
            -1f64
        } else {
            1f64
        }
    }
}

impl From<SquareOscillator> for BufferedSignal<f64> {
    fn from(value: SquareOscillator) -> Self {
        BufferedSignal::new(SquareOscillatorSignal::new(value))
    }
}

pub struct SawOscillator {
    pub frequency_hz: BufferedSignal<f64>,
}

struct SawOscillatorSignal {
    props: SawOscillator,
    state: WrapF64MinusOneToOne,
}

impl SawOscillatorSignal {
    fn new(props: SawOscillator) -> Self {
        Self {
            props,
            state: 0f64.into(),
        }
    }
}

impl SignalTrait<f64> for SawOscillatorSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.state += (self.props.frequency_hz.sample(ctx) * WrapF64MinusOneToOne::DELTA)
            / ctx.sample_rate as f64;
        self.state.value() as f64
    }
}

impl From<SawOscillator> for BufferedSignal<f64> {
    fn from(value: SawOscillator) -> Self {
        BufferedSignal::new(SawOscillatorSignal::new(value))
    }
}

pub struct Sum {
    signals: Vec<BufferedSignal<f64>>,
}

impl Sum {
    pub fn new(signals: Vec<BufferedSignal<f64>>) -> Self {
        Self { signals }
    }
}

impl SignalTrait<f64> for Sum {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.signals
            .iter_mut()
            .map(|signal| signal.sample(ctx))
            .sum()
    }
}

impl From<Sum> for BufferedSignal<f64> {
    fn from(value: Sum) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct Mul(BufferedSignal<f64>, BufferedSignal<f64>);
impl Mul {
    pub fn new(a: BufferedSignal<f64>, b: BufferedSignal<f64>) -> Self {
        Self(a, b)
    }
}

impl SignalTrait<f64> for Mul {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        self.0.sample(ctx) * self.1.sample(ctx)
    }
}

impl From<Mul> for BufferedSignal<f64> {
    fn from(value: Mul) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct AdsrEnvelopeLinear01 {
    pub gate: BufferedSignal<bool>,
    pub attack_seconds: BufferedSignal<f64>,
    pub decay_seconds: BufferedSignal<f64>,
    pub sustain_level_01: BufferedSignal<f64>,
    pub release_seconds: BufferedSignal<f64>,
}

#[derive(Debug, Clone, Copy)]
enum AdsrPosition {
    Attack,
    Decay,
    Sustain,
    Release,
}

struct AdsrEnvelopeLinear01Signal {
    props: AdsrEnvelopeLinear01,
    position: AdsrPosition,
    state: f64,
}

impl AdsrEnvelopeLinear01Signal {
    fn new(props: AdsrEnvelopeLinear01) -> Self {
        Self {
            props,
            position: AdsrPosition::Release,
            state: 0f64,
        }
    }
}

// iI will take `seconds` seconds to increase a value from 0 to 1 by adding the result of this
// function applied once per sample.
fn seconds_to_step_size_01(seconds: f64, sample_rate: u32) -> f64 {
    1_f64 / (sample_rate as f64 * seconds)
}

impl SignalTrait<f64> for AdsrEnvelopeLinear01Signal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let gate_current = self.props.gate.sample(ctx);
        if !gate_current {
            self.position = AdsrPosition::Release;
        } else if let AdsrPosition::Release = self.position {
            self.position = AdsrPosition::Attack;
        }
        let (position, state) = match self.position {
            AdsrPosition::Attack => {
                let state = self.state
                    + seconds_to_step_size_01(
                        self.props.attack_seconds.sample(ctx),
                        ctx.sample_rate,
                    );
                if state >= 1f64 {
                    (AdsrPosition::Decay, 1f64)
                } else {
                    (AdsrPosition::Attack, state)
                }
            }
            AdsrPosition::Decay => {
                let state = self.state
                    - seconds_to_step_size_01(
                        self.props.decay_seconds.sample(ctx),
                        ctx.sample_rate,
                    );
                let sustain_level_01 = self.props.sustain_level_01.sample(ctx);
                if state <= sustain_level_01 {
                    (AdsrPosition::Sustain, sustain_level_01)
                } else {
                    (AdsrPosition::Decay, state)
                }
            }
            AdsrPosition::Sustain => (AdsrPosition::Sustain, self.state),
            AdsrPosition::Release => {
                let state = self.state
                    - seconds_to_step_size_01(
                        self.props.release_seconds.sample(ctx),
                        ctx.sample_rate,
                    );
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

impl From<AdsrEnvelopeLinear01> for BufferedSignal<f64> {
    fn from(value: AdsrEnvelopeLinear01) -> Self {
        BufferedSignal::new(AdsrEnvelopeLinear01Signal::new(value))
    }
}

pub struct MovingAverageLowPassFilter {
    pub signal: BufferedSignal<f64>,
    pub width: BufferedSignal<u32>,
}

struct MovingAverageLowPassFilterSignal {
    props: MovingAverageLowPassFilter,
    buffer: VecDeque<f64>,
}

impl MovingAverageLowPassFilterSignal {
    fn new(props: MovingAverageLowPassFilter) -> Self {
        Self {
            props,
            buffer: Default::default(),
        }
    }
}

impl SignalTrait<f64> for MovingAverageLowPassFilterSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let width = self.props.width.sample(ctx) as usize + 1;
        let current_sample = self.props.signal.sample(ctx);
        while self.buffer.len() >= width {
            self.buffer.pop_front();
        }
        self.buffer.push_back(current_sample);
        let sum = self.buffer.iter().sum::<f64>();
        sum / self.buffer.len() as f64
    }
}

impl From<MovingAverageLowPassFilter> for BufferedSignal<f64> {
    fn from(value: MovingAverageLowPassFilter) -> Self {
        BufferedSignal::new(MovingAverageLowPassFilterSignal::new(value))
    }
}

pub struct MovingAverageHighPassFilter {
    pub signal: BufferedSignal<f64>,
    pub width: BufferedSignal<u32>,
}

struct MovingAverageHighPassFilterSignal {
    low_pass_filter: MovingAverageLowPassFilterSignal,
}

impl MovingAverageHighPassFilterSignal {
    fn new(props: MovingAverageHighPassFilter) -> Self {
        let low_pass_filter = MovingAverageLowPassFilterSignal::new(MovingAverageLowPassFilter {
            signal: props.signal,
            width: props.width,
        });
        Self { low_pass_filter }
    }
}

impl SignalTrait<f64> for MovingAverageHighPassFilterSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let low_pass_sample = self.low_pass_filter.sample(ctx);
        let sample = self.low_pass_filter.props.signal.sample(ctx);
        sample - low_pass_sample
    }
}

impl From<MovingAverageHighPassFilter> for BufferedSignal<f64> {
    fn from(value: MovingAverageHighPassFilter) -> Self {
        BufferedSignal::new(MovingAverageHighPassFilterSignal::new(value))
    }
}
