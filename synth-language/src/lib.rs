mod dsl;
mod signal;
mod synth_modules;

#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

pub use dsl::*;
pub use signal::{BufferedSignal, SignalCtx, SignalTrait, Var};
