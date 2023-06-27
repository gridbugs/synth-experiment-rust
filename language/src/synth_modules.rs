pub mod oscillator {
    use crate::{signal::*, Waveform};

    pub struct Props {
        pub waveform: BufferedSignal<Waveform>,
        pub frequency_hz: Sf64,
        pub reset_trigger: Sbool,
        pub reset_offset_01: Sf64,
        pub square_wave_pulse_width_01: Sf64,
    }

    struct Signal {
        props: Props,
        state: Option<f64>,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self { props, state: None }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            if self.state.is_none() {
                self.state = Some(self.props.reset_offset_01.sample(ctx));
            }
            let state = self.state.as_mut().unwrap();
            if self.props.reset_trigger.sample(ctx) {
                *state = self.props.reset_offset_01.sample(ctx);
            } else {
                *state = (*state + (self.props.frequency_hz.sample(ctx) / ctx.sample_rate as f64))
                    .rem_euclid(1.0);
            }
            let state = *state;
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

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(Signal::new(props))
    }
}

pub mod sum {
    use crate::signal::*;
    pub struct Props {
        signals: Vec<Sf64>,
    }

    impl Props {
        pub fn new(signals: Vec<Sf64>) -> Self {
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

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(props)
    }
}

pub mod weighted_sum {
    use crate::signal::*;

    pub struct WeightedSignal {
        pub weight: Sf64,
        pub signal: Sf64,
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

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(props)
    }
}

pub mod amplify {
    use crate::signal::*;

    pub struct Props {
        pub signal: Sf64,
        pub by: Sf64,
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

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(props)
    }
}

pub mod asr_envelope_lin_01 {
    use crate::signal::*;

    pub struct Props {
        pub gate: Sbool,
        pub attack_seconds: Sf64,
        pub release_seconds: Sf64,
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

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(Signal::new(props))
    }
}

pub mod adsr_envelope_lin_01 {
    use crate::signal::*;

    pub struct Props {
        pub gate: Sbool,
        pub attack_seconds: Sf64,
        pub decay_seconds: Sf64,
        pub sustain_01: Sf64,
        pub release_seconds: Sf64,
    }

    struct Signal {
        props: Props,
        current_value: f64,
        crossed_threshold: bool,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                current_value: 0.0,
                crossed_threshold: false,
            }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            if self.props.gate.sample(ctx) {
                if self.crossed_threshold {
                    // decay and sustain
                    self.current_value = (self.current_value
                        - (1.0 / (self.props.decay_seconds.sample(ctx) * ctx.sample_rate as f64)))
                        .max(self.props.sustain_01.sample(ctx));
                } else {
                    // attack
                    self.current_value = (self.current_value
                        + (1.0 / (self.props.attack_seconds.sample(ctx) * ctx.sample_rate as f64)))
                        .min(1.0);
                    if self.current_value == 1.0 {
                        self.crossed_threshold = true;
                    }
                }
            } else {
                // release
                self.crossed_threshold = false;
                self.current_value = (self.current_value
                    - (1.0 / (self.props.release_seconds.sample(ctx) * ctx.sample_rate as f64)))
                    .max(0.0);
            }
            self.current_value
        }
    }

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(Signal::new(props))
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
            pub signal: Sf64,
            pub half_power_frequency_hz: Sf64,
        }

        type Signal = SignalGen<Props>;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, half_power_frequency_hz: f64);
        }

        fn sample<U: UpdateBufferTrait, P: PassTrait>(signal: &mut Signal, ctx: &SignalCtx) -> f64 {
            let sample = signal.props.signal.sample(ctx);
            if signal.buffer.entries.is_empty() {
                return sample;
            }
            let half_power_frequency_hz = signal.props.half_power_frequency_hz.sample(ctx);
            let half_power_frequency_sample_rate_ratio =
                half_power_frequency_hz / ctx.sample_rate as f64;
            U::update_entries(&mut signal.buffer, half_power_frequency_sample_rate_ratio);
            P::apply(&mut signal.buffer, sample)
        }

        pub mod low_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    half_power_frequency_sample_rate_ratio: f64,
                ) {
                    let a = (PI * half_power_frequency_sample_rate_ratio).tan();
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

            pub fn create(props: Props, filter_order_half: usize) -> Sf64 {
                Sf64::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }

        pub mod high_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    half_power_frequency_sample_rate_ratio: f64,
                ) {
                    let a = (PI * half_power_frequency_sample_rate_ratio).tan();
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
                    sample::<UpdateBuffer, HighPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> Sf64 {
                Sf64::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }
    }

    pub mod chebyshev {
        use super::*;
        use crate::signal::*;

        const EPSILON_MIN: f64 = 0.01;

        pub struct Props {
            pub signal: Sf64,
            pub cutoff_hz: Sf64,
            pub epsilon: Sf64,
        }

        type Signal = SignalGen<Props>;

        trait UpdateBufferTrait {
            fn update_entries(buffer: &mut Buffer, cutoff_hz: f64, epsilon: f64);
        }

        fn sample<U: UpdateBufferTrait, P: PassTrait>(signal: &mut Signal, ctx: &SignalCtx) -> f64 {
            let sample = signal.props.signal.sample(ctx);
            if signal.buffer.entries.is_empty() {
                return sample;
            }
            let cutoff_hz = signal.props.cutoff_hz.sample(ctx);
            let cutoff_sample_rate_ratio = cutoff_hz / ctx.sample_rate as f64;
            let epsilon = signal.props.epsilon.sample(ctx).max(EPSILON_MIN);
            U::update_entries(&mut signal.buffer, cutoff_sample_rate_ratio, epsilon);
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
                fn update_entries(
                    buffer: &mut Buffer,
                    cutoff_sample_rate_ratio: f64,
                    epsilon: f64,
                ) {
                    let a = (PI * cutoff_sample_rate_ratio).tan();
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

            pub fn create(props: Props, filter_order_half: usize) -> Sf64 {
                Sf64::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }

        pub mod high_pass {
            pub use super::Props;
            use super::*;
            use std::f64::consts::PI;

            struct UpdateBuffer;
            impl UpdateBufferTrait for UpdateBuffer {
                fn update_entries(
                    buffer: &mut Buffer,
                    cutoff_sample_rate_ratio: f64,
                    epsilon: f64,
                ) {
                    let a = (PI * cutoff_sample_rate_ratio).tan();
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
                    sample::<UpdateBuffer, HighPass>(&mut self.0, ctx)
                }
            }

            pub fn create(props: Props, filter_order_half: usize) -> Sf64 {
                Sf64::new(Signal(SignalGen::new(props, filter_order_half)))
            }
        }
    }
}

pub mod sample_and_hold {
    use crate::signal::*;

    pub struct Props {
        pub signal: Sf64,
        pub trigger: Sbool,
    }

    struct Signal {
        props: Props,
        last_sample: f64,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            Self {
                props,
                last_sample: 0.0,
            }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> f64 {
            if self.props.trigger.sample(ctx) {
                self.last_sample = self.props.signal.sample(ctx);
            }
            self.last_sample
        }
    }

    pub fn create(props: Props) -> Sf64 {
        Sf64::new(Signal::new(props))
    }
}

pub mod clock {
    use crate::signal::*;

    pub struct Props {
        pub frequency_hz: Sf64,
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

    impl SignalTrait<bool> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> bool {
            self.state = (self.state
                + (self.props.frequency_hz.sample(ctx) / ctx.sample_rate as f64))
                .rem_euclid(1.0);
            self.state < 0.5
        }
    }

    pub fn create(props: Props) -> Sbool {
        Sbool::new(Signal::new(props)).trigger()
    }
}

pub mod random_uniform {
    use crate::signal::*;
    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;

    struct Signal {
        rng: XorShiftRng,
    }

    impl Signal {
        fn new() -> Self {
            Self {
                rng: XorShiftRng::from_entropy(),
            }
        }
    }

    impl SignalTrait<f64> for Signal {
        fn sample(&mut self, _ctx: &SignalCtx) -> f64 {
            self.rng.gen()
        }
    }

    pub fn create() -> Sf64 {
        Sf64::new(Signal::new())
    }
}

pub mod synth_sequencer {
    use crate::signal::*;

    pub struct Step {
        pub frequency_hz: Sf64,
        pub period_seconds: Sf64,
    }

    pub struct Props {
        pub sequence: Vec<Step>,
        pub clock: Sbool,
    }

    struct Signal {
        props: Props,
        step_index: usize,
        gate_remain_seconds: f64,
    }

    impl Signal {
        fn new(props: Props) -> Self {
            // the sequence will start on the first clock pulse
            Self {
                gate_remain_seconds: 0.0,
                step_index: props.sequence.len() - 1,
                props,
            }
        }
    }

    #[derive(Clone)]
    struct OutputSample {
        frequency_hz: f64,
        gate: bool,
    }

    pub struct Output {
        pub frequency_hz: Sf64,
        pub gate: Sbool,
    }

    impl SignalTrait<OutputSample> for Signal {
        fn sample(&mut self, ctx: &SignalCtx) -> OutputSample {
            let current_step = if self.props.clock.sample(ctx) {
                self.step_index = (self.step_index + 1) % self.props.sequence.len();
                let current_step = &mut self.props.sequence[self.step_index];
                self.gate_remain_seconds = current_step.period_seconds.sample(ctx);
                current_step
            } else {
                &mut self.props.sequence[self.step_index]
            };
            self.gate_remain_seconds -= 1.0 / ctx.sample_rate as f64;
            let gate = self.gate_remain_seconds >= 0.0;
            let frequency_hz = current_step.frequency_hz.sample(ctx);
            OutputSample { frequency_hz, gate }
        }
    }

    pub fn create(props: Props) -> Output {
        let combined_signal = BufferedSignal::new(Signal::new(props));
        Output {
            frequency_hz: combined_signal.map(|s| s.frequency_hz),
            gate: combined_signal.map(|s| s.gate),
        }
    }
}
