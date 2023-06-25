use std::{fmt, str::FromStr};

const SCALE_SIZE: usize = 12;
const A4_FREQUENCY_HZ: f64 = 440.0;
const A0_FREQUENCY_HZ: f64 = A4_FREQUENCY_HZ / 16.0;

pub fn note_frequency_even_temperement(base_freq: f64, key_offset: f64) -> f64 {
    base_freq * (2_f64.powf(key_offset / 12_f64))
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum NoteName {
    A,
    ASharp,
    B,
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
}

impl NoteName {
    const ALL: [Self; SCALE_SIZE] = [
        Self::A,
        Self::ASharp,
        Self::B,
        Self::C,
        Self::CSharp,
        Self::D,
        Self::DSharp,
        Self::E,
        Self::F,
        Self::FSharp,
        Self::G,
        Self::GSharp,
    ];

    fn base_index(self) -> usize {
        use NoteName::*;
        match self {
            A => 0,
            ASharp => 1,
            B => 2,
            C => 3,
            CSharp => 4,
            D => 5,
            DSharp => 6,
            E => 7,
            F => 8,
            FSharp => 9,
            G => 10,
            GSharp => 11,
        }
    }

    fn index_in_octave(self, octave: usize) -> usize {
        self.base_index() + (octave * SCALE_SIZE)
    }

    pub fn frequency_in_octave(self, octave: usize) -> f64 {
        note_frequency_even_temperement(A0_FREQUENCY_HZ, self.index_in_octave(octave) as f64)
    }
}

impl fmt::Display for NoteName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use NoteName::*;
        let str = match self {
            A => "a",
            ASharp => "a-sharp",
            B => "b",
            C => "c",
            CSharp => "c-sharp",
            D => "d",
            DSharp => "d-sharp",
            E => "e",
            F => "f",
            FSharp => "f-sharp",
            G => "g",
            GSharp => "g-sharp",
        };
        write!(f, "{}", str)
    }
}

impl FromStr for NoteName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for note in Self::ALL {
            if note.to_string() == s {
                return Ok(note);
            }
        }
        anyhow::bail!("note a note: {}", s)
    }
}
