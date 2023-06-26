use crate::music::{Note, NoteName};

pub struct Args {
    pub start_note: Note,
    pub volume_scale: f64,
    pub downsample: u32,
}

impl Args {
    fn parser() -> impl meap::Parser<Item = Self> {
        meap::let_map! {
            let {
                start_note_name = opt_opt_via::<NoteName, _, _>("NOTE", "start-note")
                    .name('n')
                    .with_default(NoteName::C);
                start_note_octave = opt_opt::<usize, _>("INT", "start-octave")
                    .name('o')
                    .with_default(2);
                volume_scale = opt_opt::<f64, _>("FLOAT", "volume")
                    .name('v')
                    .with_default(1.0);
                downsample = opt_opt::<u32, _>("INT", "downsample")
                    .with_default(1);
            } in {
                Self {
                    start_note: Note {
                        name: start_note_name,
                        octave: start_note_octave,
                    },
                    volume_scale,
                    downsample,
                }
            }
        }
    }
}

pub fn parse() -> Args {
    use meap::Parser;
    Args::parser().with_help_default().parse_env_or_exit()
}
