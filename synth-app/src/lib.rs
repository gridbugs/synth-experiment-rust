use chargrid::{control_flow::*, core::*, prelude::*};
use rgb_int::Rgb24;
use std::collections::{BTreeMap, HashMap};
use synth_language::*;

pub mod args;
pub mod music;
mod signal_player;

use args::Args;
use signal_player::SignalPlayer;

fn make_key_synth(frequency_hz: f64, gate: Sbool) -> Sf64 {
    let lfo = lfo_01(
        const_(Waveform::Saw),
        const_(2.0),
        gate.trigger(),
        const_(0.5),
    );
    let waveform = Waveform::Saw;
    let osc = sum(vec![
        oscillator(const_(waveform), const_(frequency_hz), const_(0.2)),
        oscillator(const_(waveform), const_(frequency_hz / 2.0), const_(0.2)),
    ]);
    let filter_envelope = asr_envelope_lin_01(gate.clone_ref(), const_(0.2), const_(0.2))
        .map(|x| 2000.0 * (2.0 * (x - 1.0)).exp());
    let amplify_envelope = asr_envelope_lin_01(gate.clone_ref(), const_(0.1), const_(0.2));
    let filtered_osc = osc.clone_ref();
    let filtered_osc = chebyshev_low_pass_filter(
        filtered_osc,
        weighted_sum_const_pair(0.5, filter_envelope, lfo * 2000.0).clamp_nyquist(),
        const_(5.0),
    );
    // let filtered_osc = chebyshev_high_pass_filter(filtered_osc, const_(0.0), const_(0.1));
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
    mouse_x_var: Var<f64>,
    mouse_y_var: Var<f64>,
    signal_player: SignalPlayer,
    lit_coords: HashMap<Coord, u8>,
    signal: BufferedSignal<f32>,
    octave_range: u32,
    keyboard: BTreeMap<char, Note>,
    frame_count: u64,
    recent_samples: Vec<f32>,
}

fn make_notes_even_temp(base_freq: f64, keys: &[char]) -> Vec<(char, Note)> {
    let mut mappings = Vec::new();
    for (i, &ch) in keys.iter().enumerate() {
        let freq = music::note_frequency_even_temperement(base_freq, i as f64);
        mappings.push((ch, Note::new(freq)));
    }
    mappings
}

impl AppData {
    fn new(args: Args) -> anyhow::Result<Self> {
        let signal_player = SignalPlayer::new()?;
        let start_frequency = args.start_note.frequency_in_octave(args.start_octave);
        let keyboard: BTreeMap<char, Note> = vec![
            make_notes_even_temp(
                start_frequency,
                &[
                    'a', ',', 'o', '.', 'e', 'u', 'y', 'i', 'f', 'd', 'g', 'h', 't', 'r', 'n',
                ],
            )
            .into_iter(),
            make_notes_even_temp(
                start_frequency * 2.0,
                &[
                    'A', '<', 'O', '>', 'E', 'U', 'Y', 'I', 'F', 'D', 'G', 'H', 'T', 'R', 'N',
                ],
            )
            .into_iter(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let mut key_synths: Vec<Sf64> = Vec::new();
        for note in keyboard.values() {
            key_synths.push(make_key_synth(note.frequency, note.gate.clone_ref().into()));
        }
        let keyboard_synth = sum(key_synths);
        let (mouse_x_signal, mouse_x_var) = var(0.0_f64);
        let (mouse_y_signal, mouse_y_var) = var(0.0_f64);
        let filtered_synth = chebyshev_low_pass_filter(
            keyboard_synth.clone_ref(),
            butterworth_low_pass_filter(
                mouse_x_signal.map(|x| 5000.0 * (5.0 * (x - 1.0)).exp()),
                const_(10.0),
            ),
            mouse_y_signal * 10.0,
        )
        .map(|x| (1.5 * x).clamp(-2.0, 2.0));
        Ok(Self {
            mouse_coord: None,
            signal_player,
            lit_coords: HashMap::new(),
            signal: filtered_synth.map(|s| s as f32),
            octave_range: 24,
            keyboard,
            mouse_x_var,
            mouse_y_var,
            frame_count: 0,
            recent_samples: Vec::new(),
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
        if state.recent_samples.len() > 0 {
            let width = size.width() as usize;
            let height = size.height();
            let step = state.recent_samples.len() / width;
            let scale = 1.0 / 4.0;
            let mut prev = Coord::new(0, 0);
            for x in 0..width {
                let sample = state.recent_samples[x * step];
                let top = ((height as f32 / 2.0) + (scale * sample * (height as f32 / 2.0))) as u32;
                let coord = Coord::new(x as i32, top as i32);
                if x > 0 {
                    for coord in line_2d::coords_between(prev, coord) {
                        let cell = RenderCell::default()
                            .with_character(' ')
                            .with_background(Rgba32::new_grey(255));
                        fb.set_cell_relative_to_ctx(ctx, coord, 0, cell);
                    }
                }
                prev = coord;
            }
        }
        for (coord, brightness) in state.lit_coords.iter() {
            render_coord(*coord, *brightness, size, ctx, fb);
        }
        if let Some(mouse_coord) = state.mouse_coord {
            render_coord(mouse_coord, 255, size, ctx, fb);
        }
    }

    fn update(&mut self, state: &mut Self::State, ctx: Ctx, event: Event) -> Self::Output {
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
                state
                    .mouse_x_var
                    .set(mouse_coord.x as f64 / ctx.bounding_box.size().width() as f64);
                state
                    .mouse_y_var
                    .set(mouse_coord.y as f64 / ctx.bounding_box.size().height() as f64);
            }
            state.lit_coords.retain(|_, brightness| {
                *brightness = brightness.saturating_sub(20);
                *brightness != 0
            });
            state.signal_player.send_signal(&mut state.signal);
            if state.frame_count % 1 == 0 {
                state
                    .signal_player
                    .swap_recent_samples(&mut state.recent_samples);
            }
            state.frame_count += 1;
        }
    }

    fn size(&self, _state: &Self::State, ctx: Ctx) -> Size {
        ctx.bounding_box.size()
    }
}

pub fn app(args: Args) -> anyhow::Result<App> {
    let app_data = AppData::new(args)?;
    Ok(cf(GuiComponent)
        .with_state(app_data)
        .clear_each_frame()
        .ignore_output()
        .exit_on_close())
}
