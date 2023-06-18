use crate::{
    signal::{
        self, AdsrConfiguration, Amplifier, Const, LinearAdsrEnvelopeGenerator01Builder, Mixer,
        MixerVec, MovingAverageFilterBuilder, SawWaveOsillatorBuilder, Signal,
        SineWaveOsillatorBuilder, SquareWaveOscillatorBuilder, Variable,
    },
    signal_player::SignalPlayer,
};
use chargrid::{control_flow::*, core::*, prelude::*};
use rgb_int::Rgb24;
use std::collections::{BTreeMap, HashMap};

struct AppData {
    mouse_coord: Option<Coord>,
    signal_player: SignalPlayer,
    lit_coords: HashMap<Coord, u8>,
    signal: Box<dyn Signal<f32>>,
    frequency_hz: Variable<f64>,
    pulse_width_01: Variable<f64>,
    mouse_click_gate: Variable<bool>,
    octave_range: u32,
    keyboard: BTreeMap<char, Note>,
}

fn make_key_synth(sample_rate: u32, frequency: f64, gate: Variable<bool>) -> impl Signal<f64> {
    let osc = Mixer {
        a: Mixer {
            a: SawWaveOsillatorBuilder {
                frequency_hz_signal: Const::new(frequency),
                sample_rate,
            }
            .build(),
            b: SawWaveOsillatorBuilder {
                frequency_hz_signal: Const::new(frequency * 2.0),
                sample_rate,
            }
            .build(),
        },
        b: SineWaveOsillatorBuilder {
            frequency_hz_signal: Const::new(frequency * 1.5),
            sample_rate,
        }
        .build(),
    };
    let configuration = AdsrConfiguration {
        attack_seconds: 2.0,
        decay_seconds: 4.0,
        sustain_level_01: 0.9,
        release_seconds: 1.0,
    };
    let filter_envelope = MovingAverageFilterBuilder {
        signal: LinearAdsrEnvelopeGenerator01Builder {
            gate: gate.shallow_clone(),
            sample_rate,
            configuration: configuration.clone(),
        }
        .build(),
        width: Const::new(10),
    }
    .build();
    let filter_envelope = signal::map(filter_envelope, |s| s * 0.8);
    let filter_max = 120;
    let moving_average_filter_width = signal::map(filter_envelope, move |e| {
        1 + filter_max - (filter_max as f64 * e) as u32
    });
    let filtered_osc = MovingAverageFilterBuilder {
        signal: osc,
        width: moving_average_filter_width,
    }
    .build();
    let amplifier_envelope = LinearAdsrEnvelopeGenerator01Builder {
        gate,
        sample_rate,
        configuration,
    }
    .build();
    let x = Amplifier {
        signal: filtered_osc,
        volume: amplifier_envelope,
    };
    x
}

struct Note {
    frequency: f64,
    gate_variable: Variable<bool>,
}

impl Note {
    fn new(frequency: f64) -> Self {
        Self {
            frequency,
            gate_variable: Variable::new(false),
        }
    }
}

impl AppData {
    fn new() -> anyhow::Result<Self> {
        let signal_player = SignalPlayer::new()?;
        let keyboard = maplit::btreemap! {
            'a' => Note::new(261.63), // C
            'o' => Note::new(293.66), // D
            'e' => Note::new(329.63), // E
            'u' => Note::new(349.23), // F
            'i' => Note::new(392.00), // G
            'd' => Note::new(440.00), // A
            'h' => Note::new(493.88), // B
            't' => Note::new(523.25), // C
            ',' => Note::new(277.18), // C sharp
            '.' => Note::new(311.13), // D sharp
            'y' => Note::new(369.99), // F sharp
            'f' => Note::new(415.30), // G sharp
            'g' => Note::new(466.16), // A sharp
        };
        let mut key_synths: Vec<Box<dyn Signal<f64>>> = Vec::new();
        for note in keyboard.values() {
            let x: Box<dyn Signal<f64>> = Box::new(make_key_synth(
                signal_player.sample_rate(),
                note.frequency,
                note.gate_variable.shallow_clone(),
            ));
            key_synths.push(x);
        }
        let keyboard_synth = MixerVec(key_synths);

        let frequency_hz = Variable::new(100_f64);

        let pulse_width_01 = Variable::new(0.5_f64);
        let _osc = SquareWaveOscillatorBuilder {
            high: 1_f32,
            low: -1_f32,
            frequency_hz_signal: frequency_hz.shallow_clone(),
            pulse_width_01_signal: pulse_width_01.shallow_clone(),
            sample_rate: signal_player.sample_rate(),
        }
        .build();
        let osc = Mixer {
            a: Mixer {
                a: SawWaveOsillatorBuilder {
                    frequency_hz_signal: signal::map(frequency_hz.shallow_clone(), |x| x * 1f64),
                    sample_rate: signal_player.sample_rate(),
                }
                .build(),
                b: SawWaveOsillatorBuilder {
                    frequency_hz_signal: signal::map(frequency_hz.shallow_clone(), |x| x * 1.5f64),
                    sample_rate: signal_player.sample_rate(),
                }
                .build(),
            },
            b: SineWaveOsillatorBuilder {
                frequency_hz_signal: frequency_hz.shallow_clone(),
                sample_rate: signal_player.sample_rate(),
            }
            .build(),
        };
        let _osc = SineWaveOsillatorBuilder {
            frequency_hz_signal: frequency_hz.shallow_clone(),
            sample_rate: signal_player.sample_rate(),
        }
        .build();
        let mouse_click_gate = Variable::new(false);
        let configuration = AdsrConfiguration {
            attack_seconds: 0.2,
            decay_seconds: 0.4,
            sustain_level_01: 0.75,
            release_seconds: 0.5,
        };
        let filter_envelope = MovingAverageFilterBuilder {
            signal: LinearAdsrEnvelopeGenerator01Builder {
                gate: mouse_click_gate.shallow_clone(),
                sample_rate: signal_player.sample_rate(),
                configuration: configuration.clone(),
            }
            .build(),
            width: Const::new(10),
        }
        .build();
        let filter_max = 120;
        let moving_average_filter_width = signal::map(filter_envelope, move |e| {
            1 + filter_max - (filter_max as f64 * e) as u32
        });
        let filtered_osc = MovingAverageFilterBuilder {
            signal: osc,
            width: moving_average_filter_width,
        }
        .build();
        let amplifier_envelope = LinearAdsrEnvelopeGenerator01Builder {
            gate: mouse_click_gate.shallow_clone(),
            sample_rate: signal_player.sample_rate(),
            configuration,
        }
        .build();
        let x = Amplifier {
            signal: filtered_osc,
            volume: amplifier_envelope,
        };
        Ok(Self {
            mouse_coord: None,
            signal_player,
            lit_coords: HashMap::new(),
            signal: Box::new(signal::map(keyboard_synth, |s| s as f32)),
            frequency_hz,
            pulse_width_01,
            octave_range: 24,
            mouse_click_gate,
            keyboard,
        })
    }
}

struct GuiComponent;

fn coord_to_rgba32(coord: Coord, size: Size) -> Rgba32 {
    let x = coord.x as u32;
    let y = coord.y as u32;
    let r = 255 - ((x * 255) / size.width());
    let g = (x * 510) / size.width();
    let g = if g > 255 { 510 - g } else { g };
    let b = (x * 255) / size.width();
    let mul = 255 - ((y * 255) / size.height());
    Rgb24::new(r as u8, g as u8, b as u8).to_rgba32(mul as u8)
}

fn render_coord(coord: Coord, brightness: u8, size: Size, ctx: Ctx, fb: &mut FrameBuffer) {
    let cursor_rgba32 = coord_to_rgba32(coord, size).normalised_scalar_mul(brightness);
    let cell = RenderCell::default()
        .with_character(' ')
        .with_background(cursor_rgba32);
    fb.set_cell_relative_to_ctx(ctx, coord, 0, cell);
}

fn offset_to_freq_exp(offset: f64, base_freq: f64, octave_range: f64) -> f64 {
    base_freq * 2_f64.powf(offset / octave_range)
}

impl Component for GuiComponent {
    type Output = ();
    type State = AppData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        let size = self.size(state, ctx);
        for coord in size.coord_iter_row_major() {
            if coord.x as u32 % state.octave_range == 0 {
                let cell = RenderCell::default()
                    .with_character(' ')
                    .with_background(Rgba32::new_grey(63));
                fb.set_cell_relative_to_ctx(ctx, coord, 0, cell);
            }
        }
        for (coord, brightness) in state.lit_coords.iter() {
            render_coord(*coord, *brightness, size, ctx, fb);
        }
        if let Some(mouse_coord) = state.mouse_coord {
            render_coord(mouse_coord, 255, size, ctx, fb);
        }
    }

    fn update(&mut self, state: &mut Self::State, _ctx: Ctx, event: Event) -> Self::Output {
        if let Some(mouse_input) = event.mouse_input() {
            match mouse_input {
                MouseInput::MouseMove { coord, .. } => {
                    if let Some(mouse_coord) = state.mouse_coord.as_mut() {
                        for coord in line_2d::coords_between(*mouse_coord, coord) {
                            state.lit_coords.insert(coord, 255);
                        }
                        *mouse_coord = coord;
                    } else {
                        state.mouse_coord = Some(coord);
                        state.lit_coords.insert(coord, 255);
                    }
                }
                MouseInput::MousePress { .. } => {
                    state.mouse_click_gate.set(true);
                }
                MouseInput::MouseRelease { .. } => {
                    state.mouse_click_gate.set(false);
                }
                _ => (),
            }
        }
        if let Some(keyboard_input) = event.keyboard_input() {
            match keyboard_input {
                KeyboardInput {
                    key: Key::Char(ref ch),
                    event: KeyboardEvent::KeyDown,
                } => {
                    if let Some(note) = state.keyboard.get(ch) {
                        note.gate_variable.set(true);
                    }
                }
                KeyboardInput {
                    key: Key::Char(ref ch),
                    event: KeyboardEvent::KeyUp,
                } => {
                    if let Some(note) = state.keyboard.get(ch) {
                        note.gate_variable.set(false);
                    }
                }
                _ => (),
            }
        }
        if event.tick().is_some() {
            if let Some(mouse_coord) = state.mouse_coord {
                let freq = offset_to_freq_exp(
                    (mouse_coord.x + 1) as f64,
                    27.5_f64,
                    state.octave_range as f64,
                );
                //state.frequency_hz.set(freq);
                /*
                state.pulse_width_01.set(
                    0.5_f64
                        - (mouse_coord.y as f64 / (2 * ctx.bounding_box.size().height()) as f64),
                );*/
                /*
                state
                    .moving_average_filter_width
                    .set(mouse_coord.y as u32 + 1); */
            }
            state.lit_coords.retain(|_, brightness| {
                *brightness = brightness.saturating_sub(20);
                *brightness != 0
            });
            state.signal_player.send_signal(state.signal.as_mut());
        }
    }

    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        ctx.bounding_box.size()
    }
}

pub fn app() -> anyhow::Result<App> {
    let app_data = AppData::new()?;
    Ok(cf(GuiComponent)
        .with_state(app_data)
        .clear_each_frame()
        .ignore_output()
        .exit_on_close())
}
