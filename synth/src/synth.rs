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

pub struct Amplify {
    pub signal: BufferedSignal<f64>,
    pub by: BufferedSignal<f64>,
}

impl SignalTrait<f64> for Amplify {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let by = self.by.sample(ctx);
        if by == 0f64 {
            0f64
        } else {
            self.signal.sample(ctx) * by
        }
    }
}

impl From<Amplify> for BufferedSignal<f64> {
    fn from(value: Amplify) -> Self {
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

pub struct AdsrEnvelopeExp01 {
    pub gate: BufferedSignal<bool>,
    pub attack_seconds: BufferedSignal<f64>,
    pub decay_seconds: BufferedSignal<f64>,
    pub sustain_level_01: BufferedSignal<f64>,
    pub release_seconds: BufferedSignal<f64>,
}

struct AdsrEnvelopeExp01Signal {
    props: AdsrEnvelopeExp01,
    in_attack: bool,
    prev_gate: bool,
    current_value: f64,
    attack_gradient_factor_numerator: f64,
    decay_release_gradient_factor_numerator: f64,
}

impl AdsrEnvelopeExp01Signal {
    const ATTACK_ASYMPTOTE: f64 = 1.5;
    const DECAY_RELEASE_EPSILON: f64 = 1.0 / 64.0;
    fn new(props: AdsrEnvelopeExp01) -> Self {
        Self {
            props,
            in_attack: false,
            prev_gate: false,
            current_value: 0.0,
            attack_gradient_factor_numerator: -(1.0 - (1.0 / Self::ATTACK_ASYMPTOTE)).ln(),
            decay_release_gradient_factor_numerator: -Self::DECAY_RELEASE_EPSILON.ln(),
        }
    }

    fn attack_delta(&mut self, ctx: &SignalCtx) -> f64 {
        let k = self.attack_gradient_factor_numerator / self.props.attack_seconds.sample(ctx);
        let gradient = k * (Self::ATTACK_ASYMPTOTE - self.current_value);
        gradient / ctx.sample_rate as f64
    }

    fn decay_sustain_delta(&mut self, ctx: &SignalCtx) -> f64 {
        let k = self.decay_release_gradient_factor_numerator / self.props.decay_seconds.sample(ctx);
        let sustain_01 = self.props.sustain_level_01.sample(ctx);
        let current_value_above_sustain = (self.current_value - sustain_01).max(0.0);
        let gradient = -k * current_value_above_sustain;
        gradient / ctx.sample_rate as f64
    }

    fn release_delta(&mut self, ctx: &SignalCtx) -> f64 {
        let k =
            self.decay_release_gradient_factor_numerator / self.props.release_seconds.sample(ctx);
        let gradient = -k * self.current_value;
        gradient / ctx.sample_rate as f64
    }
}

impl From<AdsrEnvelopeExp01> for BufferedSignal<f64> {
    fn from(value: AdsrEnvelopeExp01) -> Self {
        BufferedSignal::new(AdsrEnvelopeExp01Signal::new(value))
    }
}

impl SignalTrait<f64> for AdsrEnvelopeExp01Signal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let gate = self.props.gate.sample(ctx);
        self.in_attack = self.current_value != 1.0 && gate && (self.in_attack || !self.prev_gate);
        let delta = if gate {
            if self.in_attack {
                self.attack_delta(ctx)
            } else {
                self.decay_sustain_delta(ctx)
            }
        } else {
            self.release_delta(ctx)
        };
        self.current_value = (self.current_value + delta).min(1.0);
        self.prev_gate = gate;
        self.current_value
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
