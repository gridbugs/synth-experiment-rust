use crate::{
    signal::{BufferedSignal, Const, Sbool, Sf64, Var},
    synth_modules::{
        adsr_envelope_lin_01, amplify, asr_envelope_lin_01, biquad_filter, clock, oscillator,
        random_uniform, sample_and_hold, sum, synth_sequencer, weighted_sum,
    },
    Waveform,
};

pub fn const_<T: Clone + 'static>(value: T) -> BufferedSignal<T> {
    Const::new(value).into_buffered_signal()
}

pub fn var<T: Clone + 'static>(value: T) -> (BufferedSignal<T>, Var<T>) {
    let var = Var::new(value);
    (var.clone_ref().into_buffered_signal(), var)
}

pub fn lfo(
    waveform: BufferedSignal<Waveform>,
    frequency_hz: Sf64,
    reset_trigger: Sbool,
    square_wave_pulse_width_01: Sf64,
) -> Sf64 {
    use oscillator::*;
    create(Props {
        frequency_hz,
        waveform,
        reset_trigger,
        square_wave_pulse_width_01,
    })
}

pub fn lfo_01(
    waveform: BufferedSignal<Waveform>,
    frequency_hz: Sf64,
    reset_trigger: Sbool,
    square_wave_pulse_width_01: Sf64,
) -> Sf64 {
    ((lfo(
        waveform,
        frequency_hz,
        reset_trigger,
        square_wave_pulse_width_01,
    ) + 1.0)
        * 0.5)
        .map(|x| x)
}

pub fn oscillator(
    waveform: BufferedSignal<Waveform>,
    frequency_hz: Sf64,
    square_wave_pulse_width_01: Sf64,
) -> Sf64 {
    use oscillator::*;
    create(Props {
        frequency_hz,
        waveform,
        reset_trigger: const_(false),
        square_wave_pulse_width_01,
    })
}

pub fn sine_oscillator(frequency_hz: Sf64) -> Sf64 {
    oscillator(const_(Waveform::Sine), frequency_hz, const_(0.0))
}

pub fn square_oscillator(frequency_hz: Sf64, pulse_width_01: Sf64) -> Sf64 {
    oscillator(const_(Waveform::Sine), frequency_hz, pulse_width_01)
}

pub fn saw_oscillator(frequency_hz: Sf64) -> Sf64 {
    oscillator(const_(Waveform::Saw), frequency_hz, const_(0.0))
}

pub fn triangle_oscillator(frequency_hz: Sf64) -> Sf64 {
    oscillator(const_(Waveform::Triangle), frequency_hz, const_(0.0))
}

pub fn sum(values: Vec<Sf64>) -> Sf64 {
    use sum::*;
    create(Props::new(values))
}

pub fn weighted_sum_pair(left_weight: Sf64, left: Sf64, right: Sf64) -> Sf64 {
    use weighted_sum::*;
    create(Props::new(vec![
        WeightedSignal {
            weight: left_weight.clone_ref(),
            signal: left,
        },
        WeightedSignal {
            weight: left_weight.map(|x| 1.0 - x),
            signal: right,
        },
    ]))
}

pub fn weighted_sum_const_pair(left_weight: f64, left: Sf64, right: Sf64) -> Sf64 {
    weighted_sum_pair(const_(left_weight), left, right)
}

pub fn amplify(signal: Sf64, by: Sf64) -> Sf64 {
    use amplify::*;
    create(Props { signal, by })
}

pub fn asr_envelope_lin_01(gate: Sbool, attack_seconds: Sf64, release_seconds: Sf64) -> Sf64 {
    use asr_envelope_lin_01::*;
    create(Props {
        gate,
        attack_seconds,
        release_seconds,
    })
}

pub fn adsr_envelope_lin_01(
    gate: Sbool,
    attack_seconds: Sf64,
    decay_seconds: Sf64,
    sustain_01: Sf64,
    release_seconds: Sf64,
) -> Sf64 {
    use adsr_envelope_lin_01::*;
    create(Props {
        gate,
        attack_seconds,
        decay_seconds,
        sustain_01,
        release_seconds,
    })
}

pub fn butterworth_low_pass_filter(signal: Sf64, half_power_frequency_hz: Sf64) -> Sf64 {
    use biquad_filter::butterworth::low_pass::*;
    create(
        Props {
            signal,
            half_power_frequency_hz,
        },
        1,
    )
}

pub fn butterworth_high_pass_filter(signal: Sf64, half_power_frequency_hz: Sf64) -> Sf64 {
    use biquad_filter::butterworth::high_pass::*;
    create(
        Props {
            signal,
            half_power_frequency_hz,
        },
        1,
    )
}

pub fn chebyshev_low_pass_filter(signal: Sf64, cutoff_hz: Sf64, epsilon: Sf64) -> Sf64 {
    use biquad_filter::chebyshev::low_pass::*;
    create(
        Props {
            signal,
            cutoff_hz,
            epsilon,
        },
        1,
    )
}

pub fn chebyshev_high_pass_filter(signal: Sf64, cutoff_hz: Sf64, epsilon: Sf64) -> Sf64 {
    use biquad_filter::chebyshev::high_pass::*;
    create(
        Props {
            signal,
            cutoff_hz,
            epsilon,
        },
        1,
    )
}

pub fn sample_and_hold(signal: Sf64, trigger: Sbool) -> Sf64 {
    use sample_and_hold::*;
    create(Props { signal, trigger })
}

pub fn clock(frequency_hz: Sf64) -> Sbool {
    use clock::*;
    create(Props { frequency_hz })
}

pub fn random_uniform() -> Sf64 {
    use random_uniform::*;
    create()
}

pub use synth_sequencer::{Output as SynthSequencerOutput, Step as SynthSequencerStep};
pub fn synth_sequencer(sequence: Vec<SynthSequencerStep>, clock: Sbool) -> SynthSequencerOutput {
    use synth_sequencer::*;
    create(Props { sequence, clock })
}
