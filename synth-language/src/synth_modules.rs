use crate::signal::*;
use std::collections::VecDeque;

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

pub struct Oscillator {
    pub waveform: BufferedSignal<Waveform>,
    pub frequency_hz: BufferedSignal<f64>,
    pub reset_trigger: BufferedSignal<bool>,
    pub square_wave_pulse_width_01: BufferedSignal<f64>,
}

struct OscillatorSignal {
    props: Oscillator,
    state: f64,
}

impl OscillatorSignal {
    fn new(props: Oscillator) -> Self {
        Self { props, state: 0.0 }
    }
}

impl SignalTrait<f64> for OscillatorSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        if self.props.reset_trigger.sample(ctx) {
            self.state = 0f64.into();
        } else {
            self.state = (self.state
                + (self.props.frequency_hz.sample(ctx) / ctx.sample_rate as f64))
                .rem_euclid(1.0);
        }
        let state: f64 = self.state.into();
        let x = match self.props.waveform.sample(ctx) {
            Waveform::Saw => (state * 2.0) - 1.0,
            Waveform::Square => {
                if state < self.props.square_wave_pulse_width_01.sample(ctx) {
                    -1.0
                } else {
                    1.0
                }
            }
            Waveform::Triangle => (((state * 2.0) - 1.0).abs() * 2.0) - 1.0,
            Waveform::Sine => (state * std::f64::consts::PI * 2.0).sin(),
        };
        x
    }
}

impl From<Oscillator> for BufferedSignal<f64> {
    fn from(value: Oscillator) -> Self {
        BufferedSignal::new(OscillatorSignal::new(value))
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

pub struct WeightedSignal {
    pub weight: BufferedSignal<f64>,
    pub signal: BufferedSignal<f64>,
}
pub struct WeightedSum {
    weighted_signals: Vec<WeightedSignal>,
}
impl WeightedSum {
    pub fn new(weighted_signals: Vec<WeightedSignal>) -> Self {
        Self { weighted_signals }
    }
}

impl SignalTrait<f64> for WeightedSum {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let weights_sum = self
            .weighted_signals
            .iter_mut()
            .map(|ws| ws.weight.sample(ctx))
            .sum::<f64>();
        if weights_sum == 0.0 {
            0.0
        } else {
            self.weighted_signals
                .iter_mut()
                .map(|ws| ws.weight.sample(ctx) * ws.signal.sample(ctx))
                .sum::<f64>()
                / weights_sum
        }
    }
}

impl From<WeightedSum> for BufferedSignal<f64> {
    fn from(value: WeightedSum) -> Self {
        BufferedSignal::new(value)
    }
}

pub struct Amplify {
    pub signal: BufferedSignal<f64>,
    pub by: BufferedSignal<f64>,
}

impl Amplify {
    const THRESHOLD: f64 = 1.0 / 64.0;
}

impl SignalTrait<f64> for Amplify {
    fn sample(&mut self, ctx: &SignalCtx) -> f64 {
        let by = self.by.sample(ctx);
        if by.abs() > Self::THRESHOLD {
            self.signal.sample(ctx) * by
        } else {
            0f64
        }
    }
}

impl From<Amplify> for BufferedSignal<f64> {
    fn from(value: Amplify) -> Self {
        BufferedSignal::new(value)
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

pub struct StateVariableFilterFirstOrder {
    pub signal: BufferedSignal<f64>,
    pub cutoff_01: BufferedSignal<f64>,
    pub resonance_01: BufferedSignal<f64>,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StateVariableFilterFirstOrderParts<T> {
    pub low_pass: T,
    pub band_pass: T,
    pub high_pass: T,
}

impl StateVariableFilterFirstOrderParts<f64> {
    fn next(&self, signal: f64, cutoff_01: f64, resonance_01: f64) -> Self {
        let low_pass = self.low_pass + (cutoff_01 * self.band_pass);
        let band_pass = signal - self.low_pass - (resonance_01 * self.band_pass);
        let high_pass = signal - low_pass - band_pass;
        Self {
            low_pass,
            band_pass,
            high_pass,
        }
    }
}

struct StateVariableFilterFirstOrderSignal {
    props: StateVariableFilterFirstOrder,
    state: StateVariableFilterFirstOrderParts<f64>,
}

impl StateVariableFilterFirstOrderSignal {
    fn new(props: StateVariableFilterFirstOrder) -> Self {
        Self {
            props,
            state: Default::default(),
        }
    }
}

impl SignalTrait<StateVariableFilterFirstOrderParts<f64>> for StateVariableFilterFirstOrderSignal {
    fn sample(&mut self, ctx: &SignalCtx) -> StateVariableFilterFirstOrderParts<f64> {
        self.state = self.state.next(
            self.props.signal.sample(ctx),
            self.props.cutoff_01.sample(ctx),
            self.props.resonance_01.sample(ctx),
        );
        self.state
    }
}

pub type StateVariableFilterFirstOrderOutput =
    StateVariableFilterFirstOrderParts<BufferedSignal<f64>>;

impl From<StateVariableFilterFirstOrder> for StateVariableFilterFirstOrderOutput {
    fn from(value: StateVariableFilterFirstOrder) -> Self {
        let parts = BufferedSignal::new(StateVariableFilterFirstOrderSignal::new(value));
        Self {
            low_pass: parts.map(|p| p.low_pass),
            band_pass: parts.map(|p| p.band_pass),
            high_pass: parts.map(|p| p.high_pass),
        }
    }
}
