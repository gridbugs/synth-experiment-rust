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

pub mod asr_envelope_lin_01 {
    use crate::signal::*;

    pub struct Props {
        pub gate: BufferedSignal<bool>,
        pub attack_seconds: BufferedSignal<f64>,
        pub release_seconds: BufferedSignal<f64>,
    }

    struct Signal {
        props: Props,
        current_value: f64,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                current_value: 0.0,
            }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            let delta = if self.props.gate.sample(ctx) {
                1.0 / (self.props.attack_seconds.sample(ctx) * ctx.sample_rate as f64)
            } else {
                -1.0 / (self.props.release_seconds.sample(ctx) * ctx.sample_rate as f64)
            };
            self.current_value = (self.current_value + delta).clamp(0.0, 1.0);
            self.current_value
        }
    }

    impl From<Props> for BufferedSignal<f64> {
        fn from(value: Props) -> Self {
            BufferedSignal::new(Signal::new(value))
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

pub mod biquad_filter {
    // This is based on the filter designs at:
    // https://exstrom.com/journal/sigproc/dsigproc.html

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

    impl Buffer {
        fn new(filter_order_half: usize) -> Self {
            let mut entries = Vec::new();
            for _ in 0..filter_order_half {
                entries.push(Default::default());
            }
            Self { entries }
        }

        fn apply_low_pass(&mut self, mut sample: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
                sample = entry.a * (entry.w0 + (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            sample
        }

        fn apply_high_pass(&mut self, mut sample: f64) -> f64 {
            for entry in self.entries.iter_mut() {
                entry.w0 = (entry.d1 * entry.w1) + (entry.d2 * entry.w2) + sample;
                sample = entry.a * (entry.w0 - (2.0 * entry.w1) + entry.w2);
                entry.w2 = entry.w1;
                entry.w1 = entry.w0;
            }
            sample
        }
    }

    trait PassTrait {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64;
    }
    struct LowPass;
    struct HighPass;
    impl PassTrait for LowPass {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64 {
            buffer.apply_low_pass(sample)
        }
    }
    impl PassTrait for HighPass {
        fn apply(buffer: &mut Buffer, sample: f64) -> f64 {
            buffer.apply_high_pass(sample)
        }
    }

    struct SignalGen<P> {
        props: P,
        buffer: Buffer,
    }

    impl<P> SignalGen<P> {
        fn new(props: P, filter_order_half: usize) -> Self {
            Self {
                props,
                buffer: Buffer::new(filter_order_half),
            }
        }
    }

    pub mod butterworth {
        use super::*;
        use crate::signal::*;

        pub struct Props {
            pub signal: BufferedSignal<f64>,
            pub half_power_frequency: BufferedSignal<f64>,
        }

        type Signal = SignalGen<Props>;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, half_power_frequency: f64);
        }

        fn sample<U: UpdateBufferTrait, P: PassTrait>(signal: &mut Signal, ctx: &SignalCtx) -> f64 {
            let sample = signal.props.signal.sample(ctx);
            if signal.buffer.entries.is_empty() {
                return sample;
            }
            let half_power_frequency = signal.props.half_power_frequency.sample(ctx);
            U::update_entries(&mut signal.buffer, half_power_frequency);
            P::apply(&mut signal.buffer, sample)
        }

        pub mod low_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(buffer: &mut Buffer, half_power_frequency: f64) {
                    let a = ((PI * half_power_frequency) / 2.0).tan();
                    let a2 = a * a;
                    let n = buffer.entries.len() as f64;
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let r = ((PI * ((2.0 * i as f64) + 1.0)) / (4.0 * n)).sin();
                        let s = a2 + (2.0 * a * r) + 1.0;
                        entry.a = a2 / s;
                        entry.d1 = (2.0 * (1.0 - a2)) / s;
                        entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                    }
                }
            }

            struct Signal(super::Signal);

            impl SignalTrait<f64> for Signal {
                fn sample(&mut self, ctx: &SignalCtx) -> f64 {
                    sample::<UpdateBuffer, LowPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> BufferedSignal<f64> {
                BufferedSignal::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }

        pub mod high_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(buffer: &mut Buffer, half_power_frequency: f64) {
                    let a = ((PI * half_power_frequency) / 2.0).tan();
                    let a2 = a * a;
                    let n = buffer.entries.len() as f64;
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
                        let r = ((PI * ((2.0 * i as f64) + 1.0)) / (4.0 * n)).sin();
                        let s = a2 + (2.0 * a * r) + 1.0;
                        entry.a = 1.0 / s;
                        entry.d1 = (2.0 * (1.0 - a2)) / s;
                        entry.d2 = -(a2 - (2.0 * a * r) + 1.0) / s;
                    }
                }
            }

            struct Signal(super::Signal);

            impl SignalTrait<f64> for Signal {
                fn sample(&mut self, ctx: &SignalCtx) -> f64 {
                    sample::<UpdateBuffer, LowPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> BufferedSignal<f64> {
                BufferedSignal::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }
    }

    pub mod chebyshev {
        use super::*;
        use crate::signal::*;

        const EPSILON_MIN: f64 = 0.01;

        pub struct Props {
            pub signal: BufferedSignal<f64>,
            pub cutoff_01: BufferedSignal<f64>,
            pub epsilon: BufferedSignal<f64>,
        }

        type Signal = SignalGen<Props>;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, cutoff_01: f64, epsilon: f64);
        }

        fn sample<U: UpdateBufferTrait, P: PassTrait>(signal: &mut Signal, ctx: &SignalCtx) -> f64 {
            let sample = signal.props.signal.sample(ctx);
            if signal.buffer.entries.is_empty() {
                return sample;
            }
            let cutoff_01 = signal.props.cutoff_01.sample(ctx);
            let epsilon = signal.props.epsilon.sample(ctx).max(EPSILON_MIN);
            U::update_entries(&mut signal.buffer, cutoff_01, epsilon);
            let output_scaled = P::apply(&mut signal.buffer, sample);
            let scale_factor = (1.0 - (-epsilon).exp()) / 2.0;
            output_scaled / scale_factor
        }

        pub mod low_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(buffer: &mut Buffer, cutoff_01: f64, epsilon: f64) {
                    let a = ((PI * cutoff_01) / 2.0).tan();
                    let a2 = a * a;
                    let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                    let n = (buffer.entries.len() * 2) as f64;
                    let su = (u / n).sinh();
                    let cu = (u / n).cosh();
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
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
            }

            struct Signal(super::Signal);

            impl SignalTrait<f64> for Signal {
                fn sample(&mut self, ctx: &SignalCtx) -> f64 {
                    sample::<UpdateBuffer, LowPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> BufferedSignal<f64> {
                BufferedSignal::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }

        pub mod high_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(buffer: &mut Buffer, cutoff_01: f64, epsilon: f64) {
                    let a = ((PI * cutoff_01) / 2.0).tan();
                    let a2 = a * a;
                    let u = ((1.0 + (1.0 + (epsilon * epsilon)).sqrt()) / epsilon).ln();
                    let n = (buffer.entries.len() * 2) as f64;
                    let su = (u / n).sinh();
                    let cu = (u / n).cosh();
                    for (i, entry) in buffer.entries.iter_mut().enumerate() {
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
            }

            struct Signal(super::Signal);

            impl SignalTrait<f64> for Signal {
                fn sample(&mut self, ctx: &SignalCtx) -> f64 {
                    sample::<UpdateBuffer, LowPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> BufferedSignal<f64> {
                BufferedSignal::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }
    }
}
