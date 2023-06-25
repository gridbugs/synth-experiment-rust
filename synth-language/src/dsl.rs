use crate::{
    signal::{BufferedSignal, Const, Var},
    synth_modules::{
        adsr_envelope_exp_01, amplify, asr_envelope_lin_01, biquad_filter, oscillator, sum,
        weighted_sum,
    },
    Waveform,
};

pub fn const_<T: Clone + 'static>(value: T) -> BufferedSignal<T> {
    Const::new(value).into()
}

pub fn var<T: Clone + 'static>(value: T) -> (BufferedSignal<T>, Var<T>) {
    let var = Var::new(value);
    (var.clone_ref().into(), var)
}

pub fn lfo(
    waveform: BufferedSignal<Waveform>,
    frequency_hz: BufferedSignal<f64>,
    reset_trigger: BufferedSignal<bool>,
    square_wave_pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    oscillator::Props {
        frequency_hz,
        waveform,
        reset_trigger,
        square_wave_pulse_width_01,
    }
    .into()
}

pub fn lfo_01(
    waveform: BufferedSignal<Waveform>,
    frequency_hz: BufferedSignal<f64>,
    reset_trigger: BufferedSignal<bool>,
    square_wave_pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
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
    frequency_hz: BufferedSignal<f64>,
    square_wave_pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    oscillator::Props {
        frequency_hz,
        waveform,
        reset_trigger: const_(false),
        square_wave_pulse_width_01,
    }
    .into()
}

pub fn sine_oscillator(frequency_hz: BufferedSignal<f64>) -> BufferedSignal<f64> {
    oscillator(const_(Waveform::Sine), frequency_hz, const_(0.0))
}

pub fn square_oscillator(
    frequency_hz: BufferedSignal<f64>,
    pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    oscillator(const_(Waveform::Sine), frequency_hz, pulse_width_01)
}

pub fn saw_oscillator(frequency_hz: BufferedSignal<f64>) -> BufferedSignal<f64> {
    oscillator(const_(Waveform::Saw), frequency_hz, const_(0.0))
}

pub fn triangle_oscillator(frequency_hz: BufferedSignal<f64>) -> BufferedSignal<f64> {
    oscillator(const_(Waveform::Triangle), frequency_hz, const_(0.0))
}

pub fn sum(values: Vec<BufferedSignal<f64>>) -> BufferedSignal<f64> {
    sum::Props::new(values).into()
}

pub fn weighted_sum_pair(
    left_weight: BufferedSignal<f64>,
    left: BufferedSignal<f64>,
    right: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    use weighted_sum::*;
    Props::new(vec![
        WeightedSignal {
            weight: left_weight.clone_ref(),
            signal: left,
        },
        WeightedSignal {
            weight: left_weight.map(|x| 1.0 - x),
            signal: right,
        },
    ])
    .into()
}

pub fn weighted_sum_const_pair(
    left_weight: f64,
    left: BufferedSignal<f64>,
    right: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    weighted_sum_pair(const_(left_weight), left, right)
}

pub fn amplify(signal: BufferedSignal<f64>, by: BufferedSignal<f64>) -> BufferedSignal<f64> {
    amplify::Props { signal, by }.into()
}

pub fn asr_envelope_lin_01(
    gate: BufferedSignal<bool>,
    attack_seconds: BufferedSignal<f64>,
    release_seconds: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    asr_envelope_lin_01::Props {
        gate,
        attack_seconds,
        release_seconds,
    }
    .into()
}

pub fn adsr_envelope_exp_01(
    gate: BufferedSignal<bool>,
    attack_seconds: BufferedSignal<f64>,
    decay_seconds: BufferedSignal<f64>,
    sustain_level_01: BufferedSignal<f64>,
    release_seconds: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    adsr_envelope_exp_01::Props {
        gate,
        attack_seconds,
        decay_seconds,
        sustain_level_01,
        release_seconds,
    }
    .into()
}

pub fn butterworth_low_pass_filter(
    signal: BufferedSignal<f64>,
    half_power_frequency_hz: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    use biquad_filter::butterworth::low_pass::*;
    create(
        Props {
            signal,
            half_power_frequency_hz,
        },
        1,
    )
}

pub fn butterworth_high_pass_filter(
    signal: BufferedSignal<f64>,
    half_power_frequency_hz: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    use biquad_filter::butterworth::high_pass::*;
    create(
        Props {
            signal,
            half_power_frequency_hz,
        },
        1,
    )
}

pub fn chebyshev_low_pass_filter(
    signal: BufferedSignal<f64>,
    cutoff_hz: BufferedSignal<f64>,
    epsilon: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
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

pub fn chebyshev_high_pass_filter(
    signal: BufferedSignal<f64>,
    cutoff_hz: BufferedSignal<f64>,
    epsilon: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
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
