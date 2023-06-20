pub use crate::synth::Var;
use crate::{
    signal_::BufferedSignal,
    synth::{
        AdsrEnvelopeLinear01, Const, MovingAverageHighPassFilter, MovingAverageLowPassFilter, Mul,
        SawOscillator, SineOscillator, SquareOscillator, Sum,
    },
};

pub fn const_<T: Clone + 'static>(value: T) -> BufferedSignal<T> {
    Const::new(value).into()
}

pub fn var<T: Clone + 'static>(value: T) -> (BufferedSignal<T>, Var<T>) {
    let var = Var::new(value);
    (var.clone_ref().into(), var)
}

pub fn sine_oscillator(frequency_hz: BufferedSignal<f64>) -> BufferedSignal<f64> {
    SineOscillator { frequency_hz }.into()
}

pub fn square_oscillator(
    frequency_hz: BufferedSignal<f64>,
    pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    SquareOscillator {
        frequency_hz,
        pulse_width_01,
    }
    .into()
}

pub fn saw_oscillator(frequency_hz: BufferedSignal<f64>) -> BufferedSignal<f64> {
    SawOscillator { frequency_hz }.into()
}

#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
}

fn oscillator(
    waveform: Waveform,
    frequency_hz: BufferedSignal<f64>,
    pulse_width_01: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    match waveform {
        Waveform::Saw => saw_oscillator(frequency_hz),
        Waveform::Sine => sine_oscillator(frequency_hz),
        Waveform::Square => square_oscillator(frequency_hz, pulse_width_01),
    }
}

impl Waveform {
    pub fn oscillator(
        self,
        frequency_hz: BufferedSignal<f64>,
        pulse_width_01: BufferedSignal<f64>,
    ) -> BufferedSignal<f64> {
        oscillator(self, frequency_hz, pulse_width_01)
    }
}

pub fn sum(values: Vec<BufferedSignal<f64>>) -> BufferedSignal<f64> {
    Sum::new(values).into()
}

pub fn mul(lhs: BufferedSignal<f64>, rhs: BufferedSignal<f64>) -> BufferedSignal<f64> {
    Mul::new(lhs, rhs).into()
}

pub fn adsr_envelope_linear_01(
    gate: BufferedSignal<bool>,
    attack_seconds: BufferedSignal<f64>,
    decay_seconds: BufferedSignal<f64>,
    sustain_level_01: BufferedSignal<f64>,
    release_seconds: BufferedSignal<f64>,
) -> BufferedSignal<f64> {
    AdsrEnvelopeLinear01 {
        gate,
        attack_seconds,
        decay_seconds,
        sustain_level_01,
        release_seconds,
    }
    .into()
}

pub fn moving_average_low_pass_filter(
    signal: BufferedSignal<f64>,
    width: BufferedSignal<u32>,
) -> BufferedSignal<f64> {
    MovingAverageLowPassFilter { signal, width }.into()
}

pub fn moving_average_high_pass_filter(
    signal: BufferedSignal<f64>,
    width: BufferedSignal<u32>,
) -> BufferedSignal<f64> {
    MovingAverageHighPassFilter { signal, width }.into()
}
