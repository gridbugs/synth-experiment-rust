use crate::synth::{Const, SquareWaveOscillatorBuilder, Synth};
use chargrid::{control_flow::*, core::*, prelude::*};
use std::collections::HashMap;

struct AppData {
    mouse_coord: Option<Coord>,
    synth: Synth,
    lit_coords: HashMap<Coord, u8>,
}

impl AppData {
    fn new() -> anyhow::Result<Self> {
        let synth = Synth::new()?;
        let x = SquareWaveOscillatorBuilder {
            high: 1_f64,
            low: -1_f64,
            frequency_hz_signal: Const::new(440_f64),
            pulse_width_01_signal: Const::new(0.5_f64),
            sample_rate: synth.sample_rate(),
        }
        .build();
        Ok(Self {
            mouse_coord: None,
            synth,
            lit_coords: HashMap::new(),
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
    Rgba32::new_rgb(r as u8, g as u8, b as u8).normalised_scalar_mul(mul as u8)
}

fn render_coord(coord: Coord, brightness: u8, size: Size, ctx: Ctx, fb: &mut FrameBuffer) {
    let cursor_rgba32 = coord_to_rgba32(coord, size).normalised_scalar_mul(brightness);
    let cell = RenderCell::default()
        .with_character(' ')
        .with_background(cursor_rgba32);
    fb.set_cell_relative_to_ctx(ctx, coord, 0, cell);
}

impl Component for GuiComponent {
    type Output = ();
    type State = AppData;

    fn render(&self, state: &Self::State, ctx: Ctx, fb: &mut FrameBuffer) {
        let size = self.size(state, ctx);
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
                _ => (),
            }
        }
        if event.tick().is_some() {
            state.lit_coords.retain(|_, brightness| {
                *brightness = brightness.saturating_sub(10);
                *brightness != 0
            })
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
