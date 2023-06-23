use crate::{dsl::*, signal::BufferedSignal, signal_player::SignalPlayer};
use chargrid::{control_flow::*, core::*, prelude::*};
use rgb_int::Rgb24;
use std::collections::{BTreeMap, HashMap};

fn make_key_synth(frequency_hz: f64, gate: BufferedSignal<bool>) -> BufferedSignal<f64> {
    let waveform = Waveform::Triangle;
    let osc = sum(vec![
        waveform.oscillator(const_(frequency_hz / 8.0), const_(0.2)),
        waveform
            .oscillator(const_(frequency_hz / 16.0), const_(0.2))
            .map(|x| x / 1.5),
    ]);
    let filter_envelope = adsr_envelope_exp_01(
        gate.clone_ref(),
        const_(0.5),
        const_(3.0),
        const_(1.0),
        const_(4.0),
    );
    let filter_max = 50;
    let filtered_osc = moving_average_low_pass_filter(
        osc.clone_ref(),
        filter_envelope.map(move |e| filter_max - (filter_max as f64 * e) as u32),
    );
    let amplify_envelope = filter_envelope;
    //        adsr_envelope_linear_01(gate, const_(0.1), const_(0.0), const_(1.0), const_(1.0));
    amplify(filtered_osc, amplify_envelope)
}

struct Note {
    frequency: f64,
    gate: Var<bool>,
}

impl Note {
    fn new(frequency: f64) -> Self {
        Self {
            frequency,
            gate: Var::new(false),
        }
    }
}

struct AppData {
    mouse_coord: Option<Coord>,
    mouse_x_var: Var<u32>,
    mouse_y_var: Var<u32>,
    signal_player: SignalPlayer,
    lit_coords: HashMap<Coord, u8>,
    signal: BufferedSignal<f32>,
    octave_range: u32,
    keyboard: BTreeMap<char, Note>,
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
        let mut key_synths: Vec<BufferedSignal<f64>> = Vec::new();
        for note in keyboard.values() {
            key_synths.push(make_key_synth(note.frequency, note.gate.clone_ref().into()));
        }
        let keyboard_synth = sum(key_synths);
        let (mouse_x_signal, mouse_x_var) = var(0);
        let (mouse_y_signal, mouse_y_var) = var(0);
        let filtered_synth = moving_average_high_pass_filter(
            moving_average_low_pass_filter(keyboard_synth, mouse_x_signal.map(|x| x)),
            mouse_y_signal.map(|x| 60 - x),
        );
        Ok(Self {
            mouse_coord: None,
            signal_player,
            lit_coords: HashMap::new(),
            signal: filtered_synth.map(|s| s as f32 * 0.5),
            octave_range: 24,
            keyboard,
            mouse_x_var,
            mouse_y_var,
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
                MouseInput::MousePress { .. } => {}
                MouseInput::MouseRelease { .. } => {}
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
                        note.gate.set(true);
                    }
                }
                KeyboardInput {
                    key: Key::Char(ref ch),
                    event: KeyboardEvent::KeyUp,
                } => {
                    if let Some(note) = state.keyboard.get(ch) {
                        note.gate.set(false);
                    }
                }
                _ => (),
            }
        }
        if event.tick().is_some() {
            if let Some(mouse_coord) = state.mouse_coord {
                let _freq = offset_to_freq_exp(
                    (mouse_coord.x + 1) as f64,
                    27.5_f64,
                    state.octave_range as f64,
                );
                state.mouse_x_var.set(mouse_coord.x as u32);
                state.mouse_y_var.set(mouse_coord.y as u32);
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
            state.signal_player.send_signal(&mut state.signal);
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
