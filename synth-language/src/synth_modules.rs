pub mod oscillator {
    use crate::{signal::*, Waveform};

    pub struct Props {
        pub waveform: BufferedSignal<Waveform>,
        pub frequency_hz: BufferedSignal<f64>,
        pub reset_trigger: BufferedSignal<bool>,
        pub square_wave_pulse_width_01: BufferedSignal<f64>,
    }

    struct Signal {
        props: Props,
        state: f64,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self { props, state: 0.0 }
        }
    }

    impl SignalTrait<f64> for Signal {
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

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

pub mod sum {
    use crate::signal::*;
    pub struct Props {
        signals: Vec<BufferedSignal<f64>>,
    }

    impl Props {
        pub fn new(signals: Vec<BufferedSignal<f64>>) -> Self {
            Self { signals }
        }
    }

    impl SignalTrait<f64> for Props {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            self.signals
                .iter_mut()
                .map(|signal| signal.sample(ctx))
                .sum()
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(value)
        }
    }
}

pub mod weighted_sum {
    use crate::signal::*;

    pub struct WeightedSignal {
        pub weight: BufferedSignal<f64>,
        pub signal: BufferedSignal<f64>,
    }

    pub struct Props {
        weighted_signals: Vec<WeightedSignal>,
    }

    impl Props {
        pub fn new(weighted_signals: Vec<WeightedSignal>) -> Self {
            Self { weighted_signals }
        }
    }

    impl SignalTrait<f64> for Props {
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

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(value)
        }
    }
}

pub mod amplify {
    use crate::signal::*;

    pub struct Props {
        pub signal: BufferedSignal<f64>,
        pub by: BufferedSignal<f64>,
    }

    const THRESHOLD: f64 = 1.0 / 64.0;

    impl SignalTrait<f64> for Props {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let by = self.by.sample(ctx);
            if by.abs() > THRESHOLD {
                self.signal.sample(ctx) * by
            } else {
                0f64
            }
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(value)
        }
    }
}

pub mod adsr_envelope_exp_01 {
    use crate::signal::*;

    pub struct Props {
        pub gate: BufferedSignal<bool>,
        pub attack_seconds: BufferedSignal<f64>,
        pub decay_seconds: BufferedSignal<f64>,
        pub sustain_level_01: BufferedSignal<f64>,
        pub release_seconds: BufferedSignal<f64>,
    }

    struct Signal {
        props: Props,
        in_attack: bool,
        prev_gate: bool,
        current_value: f64,
        attack_gradient_factor_numerator: f64,
        decay_release_gradient_factor_numerator: f64,
    }

    const ATTACK_ASYMPTOTE: f64 = 1.5;
    const DECAY_RELEASE_EPSILON: f64 = 1.0 / 64.0;

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                in_attack: false,
                prev_gate: false,
                current_value: 0.0,
                attack_gradient_factor_numerator: -(1.0 - (1.0 / ATTACK_ASYMPTOTE)).ln(),
                decay_release_gradient_factor_numerator: -DECAY_RELEASE_EPSILON.ln(),
            }
        }

        fn attack_delta(&mut self, ctx: &SignalCtx) -> f64 {
            let k = self.attack_gradient_factor_numerator / self.props.attack_seconds.sample(ctx);
            let gradient = k * (ATTACK_ASYMPTOTE - self.current_value);
            gradient / ctx.sample_rate as f64
        }

        fn decay_sustain_delta(&mut self, ctx: &SignalCtx) -> f64 {
            let k =
                self.decay_release_gradient_factor_numerator / self.props.decay_seconds.sample(ctx);
            let sustain_01 = self.props.sustain_level_01.sample(ctx);
            let current_value_above_sustain = (self.current_value - sustain_01).max(0.0);
            let gradient = -k * current_value_above_sustain;
            gradient / ctx.sample_rate as f64
        }

        fn release_delta(&mut self, ctx: &SignalCtx) -> f64 {
            let k = self.decay_release_gradient_factor_numerator
                / self.props.release_seconds.sample(ctx);
            let gradient = -k * self.current_value;
            gradient / ctx.sample_rate as f64
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let gate = self.props.gate.sample(ctx);
            self.in_attack =
                self.current_value != 1.0 && gate && (self.in_attack || !self.prev_gate);
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

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

mod moving_average_low_pass_filter_ {
    use crate::signal::*;
    use std::collections::VecDeque;

    pub struct Props {
        pub signal: BufferedSignal<f64>,
        pub width: BufferedSignal<u32>,
    }

    pub struct Signal {
        pub props: Props,
        buffer: VecDeque<f64>,
    }

    impl Signal {
        pub fn new(props: Props) -> Self {
            Self {
                props,
                buffer: Default::default(),
            }
        }
    }

    impl SignalTrait<f64> for Signal {
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

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

pub mod moving_average_low_pass_filter {
    pub use super::moving_average_low_pass_filter_::Props;
}

pub mod moving_average_high_pass_filter {
    use super::moving_average_low_pass_filter_;
    use crate::signal::*;

    pub struct Props {
        pub signal: BufferedSignal<f64>,
        pub width: BufferedSignal<u32>,
    }

    struct Signal {
        low_pass_filter: moving_average_low_pass_filter_::Signal,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            let low_pass_filter = moving_average_low_pass_filter_::Signal::new(
                moving_average_low_pass_filter_::Props {
                    signal: props.signal,
                    width: props.width,
                },
            );
            Self { low_pass_filter }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let low_pass_sample = self.low_pass_filter.sample(ctx);
            let sample = self.low_pass_filter.props.signal.sample(ctx);
            sample - low_pass_sample
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

mod state_variable_filter {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct Parts<T> {
        pub low_pass: T,
        pub band_pass: T,
        pub high_pass: T,
    }
}

pub mod state_variable_filter_first_order {
    pub use super::state_variable_filter::Parts;
    use crate::signal::*;

    pub struct PropsGen<T> {
        pub signal: T,
        pub cutoff_01: T,
        pub resonance_01: T,
    }

    pub type Props = PropsGen<BufferedSignal<f64>>;

    fn next_parts(parts: &Parts<f64>, props: &PropsGen<f64>) -> Parts<f64> {
        let low_pass = parts.low_pass + (props.cutoff_01 * parts.band_pass);
        let band_pass = props.signal - parts.low_pass - (props.resonance_01 * parts.band_pass);
        let high_pass = props.signal - low_pass - band_pass;
        Parts {
            low_pass,
            band_pass,
            high_pass,
        }
    }

    struct Signal {
        props: Props,
        state: Parts<f64>,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                state: Default::default(),
            }
        }
    }

    impl SignalTrait<Parts<f64>> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> Parts<f64> {
            let props = PropsGen {
                signal: self.props.signal.sample(ctx),
                cutoff_01: self.props.cutoff_01.sample(ctx),
                resonance_01: self.props.resonance_01.sample(ctx),
            };
            self.state = next_parts(&self.state, &props);
            self.state
        }
    }

    pub type Output = Parts<BufferedSignal<f64>>;

    impl From<Props> for Output {
        fn from(value: Props) -> Self {
            let parts = BufferedSignal::new(Signal::new(value));
            Self {
                low_pass: parts.map(|p| p.low_pass),
                band_pass: parts.map(|p| p.band_pass),
                high_pass: parts.map(|p| p.high_pass),
            }
        }
    }
}

mod biquad_filter {
    use crate::signal::*;

    /// Param names are taken from wikipedia. The a0 parameter is missing because it is a
    /// normalization parameter and is assumed to be 1.
    pub struct Params<T> {
        pub a1: T,
        pub a2: T,
        pub b0: T,
        pub b1: T,
        pub b2: T,
    }

    pub struct PropsGen<T> {
        pub signal: T,
        pub params: Params<T>,
    }

    pub type Props = PropsGen<BufferedSignal<f64>>;

    #[derive(Default)]
    struct Buffers {
        input: [f64; 2],
        output: [f64; 2],
    }

    impl Buffers {
        fn next_output(&self, props: &PropsGen<f64>) -> f64 {
            (props.params.b0 * props.signal)
                + (props.params.b1 * self.input[0])
                + (props.params.b2 * self.input[1])
                - (props.params.a1 * self.output[0])
                - (props.params.a2 * self.output[1])
        }

        fn next(&mut self, props: &PropsGen<f64>) -> f64 {
            let next_output = self.next_output(props);
            self.input[1] = self.input[0];
            self.input[0] = props.signal;
            self.output[1] = self.output[0];
            self.output[0] = next_output;
            next_output
        }
    }

    struct Signal {
        props: Props,
        buffers: Buffers,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                buffers: Default::default(),
            }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let props = PropsGen {
                signal: self.props.signal.sample(ctx),
                params: Params {
                    a1: self.props.params.a1.sample(ctx),
                    a2: self.props.params.a2.sample(ctx),
                    b0: self.props.params.b0.sample(ctx),
                    b1: self.props.params.b1.sample(ctx),
                    b2: self.props.params.b2.sample(ctx),
                },
            };
            self.buffers.next(&props)
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

pub mod chebyshev_low_pass_filter {
    use crate::signal::*;
    use std::f64::consts::PI;

    pub struct Props {
        pub signal: BufferedSignal<f64>,
        pub cutoff_01: BufferedSignal<f64>,
        pub epsilon: BufferedSignal<f64>,
        pub num_chained_filters: usize,
    }

    #[derive(Default)]
    struct BufferEntry {
        a: f64,
        d1: f64,
        d2: f64,
        w0: f64,
        w1: f64,
        w2: f64,
    }

    struct Buffer {
        entries: Vec<BufferEntry>,
    }

    const CUTOFF_MIN: f64 = 0.001;
    const EPSILON_MIN: f64 = 0.1;

    impl Buffer {
        fn update_entries(&mut self, cutoff_01: f64, epsilon: f64) {
            // This is based on the chebyshev lowpass filter at:
            // https://exstrom.com/journal/sigproc/dsigproc.html
            let a = ((PI * cutoff_01) / 2.0).tan();
            let a2 = a * a;
            let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
            let n = (self.entries.len() * 2) as f64;
            let su = (u / n).sinh();
            let cu = (u / n).cosh();
            for (i, entry) in self.entries.iter_mut().enumerate() {
                let theta = (PI * ((2.0 * i as f64) + 1.0)) / (2.0 * n);
                let b = theta.sin() * su;
                let c = theta.cos() * cu;
                let c = (b * b) + (c * c);
                let s = (a2 * c) + (2.0 * a * b) + 1.0;
                entry.a = a2 / (4.0 * s);
                entry.d1 = (2.0 * (1.0 - (a2 * c))) / s;
                entry.d2 = -((a2 * c) - (2.0 * a * b) + 1.0) / s;
            }
        }

        fn filter_sample(&mut self, mut x: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + x;
                x = entry.a * (entry.w0 + (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            x
        }
    }

    struct Signal {
        props: Props,
        buffer: Buffer,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            let mut buffer = Buffer {
                entries: Vec::new(),
            };
            for _ in 0..props.num_chained_filters {
                buffer.entries.push(Default::default());
            }
            Self { props, buffer }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let x = self.props.signal.sample(ctx);
            if self.buffer.entries.is_empty() {
                return x;
            }
            let cutoff_01 = self.props.cutoff_01.sample(ctx).max(CUTOFF_MIN);
            let epsilon = self.props.epsilon.sample(ctx).max(EPSILON_MIN);
            self.buffer.update_entries(cutoff_01, epsilon);
            let output_scaled = self.buffer.filter_sample(x);
            let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
            output_scaled / scale_factor
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}

pub mod chebyshev_high_pass_filter {
    use crate::signal::*;
    use std::f64::consts::PI;

    pub struct Props {
        pub signal: BufferedSignal<f64>,
        pub cutoff_01: BufferedSignal<f64>,
        pub epsilon: BufferedSignal<f64>,
        pub num_chained_filters: usize,
    }

    #[derive(Default)]
    struct BufferEntry {
        a: f64,
        d1: f64,
        d2: f64,
        w0: f64,
        w1: f64,
        w2: f64,
    }

    struct Buffer {
        entries: Vec<BufferEntry>,
    }

    const CUTOFF_MIN: f64 = 0.001;
    const EPSILON_MIN: f64 = 0.1;

    impl Buffer {
        fn update_entries(&mut self, cutoff_01: f64, epsilon: f64) {
            // This is based on the chebyshev lowpass filter at:
            // https://exstrom.com/journal/sigproc/dsigproc.html
            let a = ((PI * cutoff_01) / 2.0).tan();
            let a2 = a * a;
            let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
            let n = (self.entries.len() * 2) as f64;
            let su = (u / n).sinh();
            let cu = (u / n).cosh();
            for (i, entry) in self.entries.iter_mut().enumerate() {
                let theta = (PI * ((2.0 * i as f64) + 1.0)) / (2.0 * n);
                let b = theta.sin() * su;
                let c = theta.cos() * cu;
                let c = (b * b) + (c * c);
                let s = a2 + (2.0 * a * b) + c;
                entry.a = 1.0 / (4.0 * s);
                entry.d1 = (2.0 * (c - a2)) / s;
                entry.d2 = -(a2 - (2.0 * a * b) + c) / s;
            }
        }

        fn filter_sample(&mut self, mut x: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + x;
                x = entry.a * (entry.w0 - (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            x
        }
    }

    struct Signal {
        props: Props,
        buffer: Buffer,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            let mut buffer = Buffer {
                entries: Vec::new(),
            };
            for _ in 0..props.num_chained_filters {
                buffer.entries.push(Default::default());
            }
            Self { props, buffer }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let x = self.props.signal.sample(ctx);
            if self.buffer.entries.is_empty() {
                return x;
            }
            let cutoff_01 = self.props.cutoff_01.sample(ctx).max(CUTOFF_MIN);
            let epsilon = self.props.epsilon.sample(ctx).max(EPSILON_MIN);
            self.buffer.update_entries(cutoff_01, epsilon);
            let output_scaled = self.buffer.filter_sample(x);
            let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
            output_scaled / scale_factor
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
        }
    }
}
