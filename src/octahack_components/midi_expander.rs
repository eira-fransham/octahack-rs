use crate::{Component, Context, MidiValue};
use arrayvec::ArrayVec;

crate::specs! {
    pub mod input {
        Midi: crate::MidiValue
    }

    // TODO: Have some system of having each output be "multiplexable", i.e. have a
    //       single `Freq` output with some parameter representing the voice to
    //       address at runtime, allowing us to change the number of voices at
    //       runtime.
    pub mod output {
        Freq0: crate::Value,
        Vel0: crate::Value,
        Freq1: crate::Value,
        Vel1: crate::Value,
        Freq2: crate::Value,
        Vel2: crate::Value,
        Freq3: crate::Value,
        Vel3: crate::Value
    }
}

const VOICES: usize = 4;

struct Note {
    freq: f64,
    vel: f64,
}

#[derive(Clone, Default)]
pub struct MidiExpander {
    notes: ArrayVec<[f64; VOICES]>,
}

impl MidiExpander {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for MidiExpander {
    type InputSpecifier = input::Specifier;
    type OutputSpecifier = output::Specifier;
    type ParamSpecifier = !;

    fn update<Ctx>(&self, ctx: &Ctx) -> Self
    where
        Ctx: Context<Self>,
    {
        let out = self.clone();

        if let Some(midi) = ctx.input::<input::Midi>() {
            for msg in midi {
                match msg {
                    MidiValue::NoteOn(_note, _) => unimplemented!(),
                    MidiValue::NoteOff(_note, _) => unimplemented!(),
                    _ => {}
                }
            }
        }

        out
    }
}
