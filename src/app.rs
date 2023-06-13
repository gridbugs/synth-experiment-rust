use crate::synth::{Const, Signal, SquareWaveOscillatorBuilder, Synth, Variable};
use chargrid::{control_flow::*, core::*, prelude::*};
use rgb_int::Rgb24;
use std::collections::HashMap;

struct AppData {
    mouse_coord: Option<Coord>,
    synth: Synth,
    lit_coords: HashMap<Coord, u8>,
    signal: Box<dyn Signal<f32>>,
    frequency_hz: Variable<f64>,
    pulse_width_01: Variable<f64>,
    octave_range: u32,
}

impl AppData {
    fn new() -> anyhow::Result<Self> {
        let synth = Synth::new()?;
        let frequency_hz = Variable::new(100_f64);
        let pulse_width_01 = Variable::new(0.5_f64);
        let x = SquareWaveOscillatorBuilder {
            high: 0.1_f32,
            low: -0.1_f32,
            frequency_hz_signal: frequency_hz.shallow_clone(),
            pulse_width_01_signal: pulse_width_01.shallow_clone(),
            sample_rate: synth.sample_rate(),
        }
        .build();
        Ok(Self {
            mouse_coord: None,
            synth,
            lit_coords: HashMap::new(),
            signal: Box::new(x),
            frequency_hz,
            pulse_width_01,
            octave_range: 24,
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
                _ => (),
            }
        }
        if event.tick().is_some() {
            if let Some(mouse_coord) = state.mouse_coord {
                let freq =
                    offset_to_freq_exp(mouse_coord.x as f64, 55_f64, state.octave_range as f64);
                state.frequency_hz.set(freq);
                state.pulse_width_01.set(
                    0.5_f64
                        - (mouse_coord.y as f64 / (2 * ctx.bounding_box.size().height()) as f64),
                );
            }
            state.lit_coords.retain(|_, brightness| {
                *brightness = brightness.saturating_sub(20);
                *brightness != 0
            });
            state.synth.send_signal(state.signal.as_mut());
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
