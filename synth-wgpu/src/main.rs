use synth_app::{self, args};

const CELL_SIZE: f64 = 12.;

fn main() -> anyhow::Result<()> {
    use chargrid_wgpu::*;
    let args = args::parse();
    env_logger::init();
    let context = Context::new(Config {
        font_bytes: FontBytes {
            normal: include_bytes!("./fonts/PxPlus_IBM_CGAthin-with-quadrant-blocks.ttf").to_vec(),
            bold: include_bytes!("./fonts/PxPlus_IBM_CGA-with-quadrant-blocks.ttf").to_vec(),
        },
        title: "Synth Experiment".to_string(),
        window_dimensions_px: Dimensions {
            width: 1280.,
            height: 720.,
        },
        cell_dimensions_px: Dimensions {
            width: CELL_SIZE,
            height: CELL_SIZE,
        },
        font_scale: Dimensions {
            width: CELL_SIZE,
            height: CELL_SIZE,
        },
        underline_width_cell_ratio: 0.1,
        underline_top_offset_cell_ratio: 0.8,
        resizable: false,
        force_secondary_adapter: false,
    });
    context.run(synth_app::app(args)?)
}
