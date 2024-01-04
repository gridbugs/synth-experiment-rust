use chargrid::{control_flow::*, core::*, prelude::*};
use rgb_int::Rgb24;
use std::collections::{BTreeMap, HashMap};
use synth_language::*;

pub mod args;
pub mod music;
mod samples;
mod signal_player;

use args::Args;
use signal_player::SignalPlayer;

fn make_key_synth(frequency_hz: Sf64, gate: Sbool, clock: Sbool) -> Sf64 {
    let noise = random_uniform();
    let lfo = lfo_01(
        const_(Waveform::Sine),
        const_(0.5),
        gate.trigger(),
        const_(0.25),
        const_(0.5),
    );
    let sah = butterworth_low_pass_filter(sample_and_hold(noise.clone_ref(), clock), const_(100.0));
    let waveform = Waveform::Saw;
    let osc = sum(vec![oscillator(
        const_(waveform),
        frequency_hz.clone_ref() * 0.5,
        const_(0.2),
    )]);
    let release = const_(0.2);
    let env = butterworth_low_pass_filter(
        adsr_envelope_lin_01(
            gate.clone_ref(),
            const_(0.05),
            const_(0.5),
            const_(1.0),
            release.clone_ref(),
        )
        .exp01(2.0),
        const_(5.0),
    );
    let filtered_osc = chebyshev_low_pass_filter(
        osc,
        env.clone_ref() * 500.0 + 100.0 + lfo * 1000.0 + sah * 500.0,
        const_(10.0),
    );
    amplify(
        filtered_osc,
        asr_envelope_lin_01(gate, const_(0.01), release),
    )
}

fn make_sequencer(sequencer_clock: Sbool, effect_clock: Sbool) -> Sf64 {
    use music::{note, NoteName::*};
    let octave_base = 2;
    let note_sequence = vec![
        (C, 0),
        (C, 0),
        (C, 0),
        (C, 0),
        (C, 0),
        (C, 1),
        (G, 0),
        (C, 1),
    ];
    let note_period_seconds = 0.1;
    let sequence = note_sequence
        .iter()
        .map(|&(note_name, octave_offset)| SynthSequencerStep {
            frequency_hz: const_(note(note_name, octave_base + octave_offset).frequency()),
            period_seconds: const_(note_period_seconds),
        })
        .collect();
    let SynthSequencerOutput { frequency_hz, gate } = synth_sequencer(sequence, sequencer_clock);
    make_key_synth(frequency_hz, gate, effect_clock)
}

fn make_drum_sequencer(sequencer_clock: Sbool) -> Sf64 {
    const SNARE: u8 = 1 << 0;
    const BASS: u8 = 1 << 1;
    const SYMBOL: u8 = 1 << 2;
    let sequence = vec![
        SYMBOL | BASS,
        SYMBOL,
        SYMBOL | SNARE,
        SYMBOL,
        SYMBOL,
        SYMBOL | BASS,
        SYMBOL | SNARE | BASS,
        SYMBOL,
    ]
    .into_iter()
    .map(const_)
    .collect();
    let [snare_trigger, bass_trigger, symbol_trigger, ..] =
        trigger_sequencer_8(sequence, sequencer_clock);
    sum(vec![
        sample_player(samples::sn01(), snare_trigger),
        sample_player(samples::bd01(), bass_trigger),
        sample_player(samples::ch01(), symbol_trigger),
    ])
}

struct NoteKey {
    frequency: f64,
    gate: Var<bool>,
}

impl NoteKey {
    fn new(frequency: f64) -> Self {
        Self {
            frequency,
            gate: Var::new(false),
        }
    }
}

struct AppData {
    args: Args,
    mouse_coord: Option<Coord>,
    mouse_x_var: Var<f64>,
    mouse_y_var: Var<f64>,
    signal_player: SignalPlayer,
    lit_coords: HashMap<Coord, u8>,
    signal: BufferedSignal<f32>,
    octave_range: u32,
    buttons: BTreeMap<char, BoolVar>,
    frame_count: u64,
    recent_samples: Vec<f32>,
}

fn make_notes_even_temp(base_freq: f64, keys: &[char]) -> Vec<(char, NoteKey)> {
    let mut mappings = Vec::new();
    for (i, &ch) in keys.iter().enumerate() {
        let freq = music::note_frequency_even_temperement(base_freq, i as f64 - 1.0);
        mappings.push((ch, NoteKey::new(freq)));
    }
    mappings
}

fn sample_var(sample: Vec<f32>) -> (Sf64, TriggerVar) {
    let (trigger, var) = trigger_var();
    let player = sample_player(sample, trigger);
    (player, var)
}

impl AppData {
    fn new(args: Args) -> anyhow::Result<Self> {
        samples::sn01();
        let signal_player = SignalPlayer::new(args.downsample)?;
        let start_frequency = args.start_note.frequency();
        let keyboard: BTreeMap<char, NoteKey> = vec![make_notes_even_temp(
            start_frequency,
            &[
                'a', 'o', '.', 'e', 'p', 'u', 'i', 'f', 'd', 'g', 'h', 'c', 't', 'n', 'l', 's',
            ],
        )
        .into_iter()]
        .into_iter()
        .flatten()
        .collect();
        let effect_clock = clock(const_(6.0));
        let mut key_synths: Vec<Sf64> = Vec::new();
        for note in keyboard.values() {
            key_synths.push(make_key_synth(
                const_(note.frequency),
                note.gate.buffered_signal(),
                effect_clock.clone_ref(),
            ));
        }
        let drum_machine = maplit::btreemap! {
            ';' => sample_var(samples::sn01()),
            'q' => sample_var(samples::bd01()),
            'j' => sample_var(samples::ch01()),
        };
        let (mouse_x_signal, mouse_x_var) = var(0.0_f64);
        let (mouse_y_signal, mouse_y_var) = var(0.0_f64);
        let sequencer_clock = clock(const_(3.0));
        let sequencers = make_sequencer(sequencer_clock.clone_ref(), const_(false))
            + (make_drum_sequencer(sequencer_clock) * 8.0);
        let keyboard_synth = sum(key_synths);
        let drums = sum(drum_machine.values().map(|(s, _)| s.clone_ref()).collect());
        let manual_synth = sum(vec![keyboard_synth, drums]);
        let combined_synth = sum(vec![manual_synth, sequencers * 0.0]);
        let filtered_synth = chebyshev_low_pass_filter(
            combined_synth.clone_ref(),
            butterworth_low_pass_filter(
                mouse_x_signal.map(|x| 5000.0 * (4.0 * (x - 1.0)).exp()),
                const_(5.0),
            ),
            mouse_y_signal * 10.0,
        )
        .map(|x| (x * 1.0).clamp(-2.0, 2.0));
        let buttons = keyboard
            .into_iter()
            .map(|(key, NoteKey { frequency: _, gate })| (key, gate.bool_var()))
            .chain(
                drum_machine
                    .into_iter()
                    .map(|(ch, (_, var))| (ch, var.bool_var())),
            )
            .collect();
        Ok(Self {
            mouse_coord: None,
            signal_player,
            lit_coords: HashMap::new(),
            signal: filtered_synth.map(move |s| (s * args.volume_scale) as f32),
            octave_range: 24,
            buttons,
            mouse_x_var,
            mouse_y_var,
            frame_count: 0,
            recent_samples: Vec::new(),
            args,
        })
    }
}

struct GuiComponent;

fn coord_to_rgba32(coord: Coord, size: Size) -> Rgba32 {
    let x = coord.x as u32;
    let y = coord.y as u32;
    let r = 255_u32.saturating_sub((x * 255) / size.width());
    let g = (x * 510) / size.width();
    let g = if g > 255 {
        510_u32.saturating_sub(g)
    } else {
        g
    };
    let b = (x * 255) / size.width();
    let mul = 255_u32.saturating_sub((y * 255) / size.height());
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
            let mut prev = Coord::new(0, 0);
            for x in 0..width {
                let sample = state.recent_samples[x * step] / state.args.volume_scale as f32;
                let top = ((height as f32 / 2.0)
                    + (state.args.render_scale as f32 * sample * (height as f32 / 2.0)))
                    as u32;
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
                    if let Some(note) = state.buttons.get(ch) {
                        note.set();
                    }
                }
                KeyboardInput {
                    key: Key::Char(ref ch),
                    event: KeyboardEvent::KeyUp,
                } => {
                    if let Some(note) = state.buttons.get(ch) {
                        note.clear();
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
            state
                .signal_player
                .swap_recent_samples(&mut state.recent_samples);
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
