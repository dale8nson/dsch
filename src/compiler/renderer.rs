use crate::compiler::{codegen::*, composer::State};

pub use midly::{Header, MetaMessage, MidiMessage, Smf, Track, TrackEvent, TrackEventKind, num::*};

#[derive(Debug, Default)]
pub struct Renderer<'a> {
    instructions: Vec<Vec<Instruction<'a>>>,
    events: Vec<Vec<TrackEvent<'a>>>,
}

impl<'a> Renderer<'a> {
    pub fn render(&mut self, state: State) {
        let ctx = Ctx::Root;
    }

    fn instructions_mut(&mut self) -> &mut Vec<Vec<Instruction<'a>>> {
        &mut self.instructions
    }

    fn events_mut(&mut self) -> &mut Vec<Vec<TrackEvent<'a>>> {
        &mut self.events
    }
}
