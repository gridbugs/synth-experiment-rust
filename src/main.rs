mod app;
mod synth;

fn main() {
    use chargrid_sdl2::*;
    env_logger::init();
    let context = Context::new(Config {
        font_bytes: FontBytes {
            normal: include_bytes!("./fonts/PxPlus_IBM_CGAthin-with-quadrant-blocks.ttf").to_vec(),
            bold: include_bytes!("./fonts/PxPlus_IBM_CGA-with-quadrant-blocks.ttf").to_vec(),
        },
        title: "Synth Experiment".to_string(),
        window_dimensions_px: Dimensions {
            width: 960.,
            height: 720.,
        },
        cell_dimensions_px: Dimensions {
            width: 12.,
            height: 12.,
        },
        font_point_size: 12,
        character_cell_offset: Dimensions {
            width: 0.,
            height: -1.,
        },
        underline_width_cell_ratio: 0.1,
        underline_top_offset_cell_ratio: 0.8,
        resizable: false,
    });
    context.run(app::app());
}
