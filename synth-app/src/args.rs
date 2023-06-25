use crate::music::NoteName;

pub struct Args {
    pub start_note: NoteName,
    pub start_octave: usize,
    pub volume_scale: f64,
}

impl Args {
    fn parser() -> impl meap::Parser<Item = Self> {
        meap::let_map! {
            let {
                start_note = opt_opt_via::<NoteName, _, _>("NOTE", "start-note")
                    .name('n')
                    .with_default(NoteName::C);
                start_octave = opt_opt::<usize, _>("INT", "start-octave")
                    .name('o')
                    .with_default(2);
                volume_scale = opt_opt::<f64, _>("FLOAT", "volume")
                    .with_default(1.0);
            } in {
                Self {
                    start_note,
                    start_octave,
                    volume_scale,
                }
            }
        }
    }
}

pub fn parse() -> Args {
    use meap::Parser;
    Args::parser().with_help_default().parse_env_or_exit()
}
