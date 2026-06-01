#![allow(unused)]
use std::collections::{BTreeMap, HashSet};

use crate::compiler::{
    ast::NOOP,
    codegen::{utils::length_to_ticks, *},
    composer::{State, print_state},
    functional::*,
};

pub use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
    num::*,
};

pub type Ticks = u64;

#[derive(Debug)]
pub struct Scheduler<'a> {
    prog: u8,
    tempo: Mpb,
    clock: Ticks,
    schedule: BTreeMap<Ticks, Vec<Instruction<'a>>>,
    visited: HashSet<Ctx>,
}

impl<'a> Default for Scheduler<'a> {
    fn default() -> Self {
        Scheduler {
            prog: 0,
            tempo: Mpb(f64::round(60_000_000. / 120.) as u64),
            clock: 0,
            schedule: BTreeMap::<Ticks, Vec<Instruction>>::new(),
            visited: HashSet::<Ctx>::new(),
        }
    }
}

impl<'a> Scheduler<'a> {
    fn forward(&mut self, ticks: u64) {
        self.clock += ticks
    }

    fn rewind(&mut self, ticks: u64) {
        self.clock -= ticks;
    }

    fn add_instruction(&mut self, time: u64, instruction: Instruction<'a>) {
        let instructions = self.schedule_mut().get_mut(&time);
        if instructions.is_some() {
            instructions.unwrap().push(instruction);
        } else {
            let instructions = vec![instruction];
            self.schedule_mut().insert(time, instructions);
        }
    }

    fn set_tempo(&mut self, mpb: Mpb) {
        if mpb.0 != self.tempo.0 {
            let time = self.clock;
            let tempo = u24::new(mpb.0 as u32);
            self.add_instruction(time, Instruction::Meta(MetaMessage::Tempo(tempo)));
        }
    }

    fn schedule_mut(&mut self) -> &mut BTreeMap<Ticks, Vec<Instruction<'a>>> {
        &mut self.schedule
    }
}

pub fn schedule<'a>(mut state: State) -> Smf<'a> {
    dbg!(&state);
    let ctx = Ctx::Id(0);
    let mut scheduler = Scheduler::default();

    let header = Header::new(Format::SingleTrack, Timing::Metrical(PPQ));

    scheduler.add_instruction(
        0 as u64,
        Instruction::Midi(MidiMessage::ProgramChange {
            program: u7::new(0),
        }),
    );

    scheduler.add_instruction(
        0 as u64,
        Instruction::Meta(MetaMessage::Tempo(u24::new(
            f64::round(60_000_000 as f64 / 120 as f64) as u32,
        ))),
    );
    schedule_context(Monad::ret(ctx), &mut state, &mut scheduler);
    // dbg!(&scheduler);
    let tracks = render_tracks(&mut scheduler);
    dbg!(&tracks);
    // pause();

    let smf = Smf { header, tracks };
    smf.to_static()
}

fn schedule_context<'a>(
    mut ctx: Monad<Ctx>,
    state: &mut State,
    scheduler: &mut Scheduler<'a>,
) -> Monad<Ctx> {
    // dbg!(ctx);

    ctx.bind(Box::new(|mut ctx_| {
        let mut context = state.get_context(ctx_);
        let len = context.pcs.len();
        let scope = state.scope_type(ctx_);
        context.children.iter().cloned().for_each(|child| {
            schedule_context(Monad::ret(child), state, scheduler);
        });

        if matches!(scope, ScopeType::Sequence) {
            context
                .pcs
                .into_iter()
                .zip(context.velocities.into_iter().cycle().take(len))
                .zip(context.lengths.into_iter().cycle().take(len))
                .for_each(|((pc, velocity), length)| {
                    let beats = length.as_u64() as f64 / context.tempo.0 as f64;

                    schedule_note(scheduler, vec![pc], context.register, vec![velocity], beats);

                    scheduler.forward(length_to_ticks(length, context.tempo));
                });
        } else {
            let beats = (&context).lengths[0].as_u64() as f64 / (&context).tempo.0 as f64;
            schedule_note(
                scheduler,
                context.pcs.clone(),
                context.register,
                context.velocities,
                beats,
            );

            let parent = state.parent(ctx_);
            // print_state(state, parent);
            print_state(state, ctx_);
            let parent_scope = state.scope_type(parent);
            if matches!(parent_scope, ScopeType::Sequence) {
                scheduler.forward(length_to_ticks(
                    *context.lengths.iter().max().unwrap(),
                    context.tempo,
                ));
            }
        }

        Monad::ret(ctx_)
    }))
}

fn get_leaf<'a>(ctx: Monad<Ctx>, state: &mut State, scheduler: &mut Scheduler) -> Monad<Ctx> {
    ctx.bind(Box::new(|mut ctx_| {
        if is_leaf(ctx_, state) {
            if !scheduler.visited.contains(&ctx_) {
                scheduler.visited.insert(ctx_);
                return Monad::ret(ctx_);
            } else {
                ctx_ = state.parent(ctx_);

                return get_leaf(Monad::ret(ctx_), state, scheduler);
            }
        } else {
            let mut unvisited_children: Vec<Ctx> = state
                .children(ctx_)
                .iter()
                .cloned()
                .filter(|ctx| !scheduler.visited.contains(&ctx))
                .collect();

            // dbg!(&unvisited_children);
            while unvisited_children.len() == 0 {
                scheduler.visited.insert(ctx_);
                if scheduler.visited.contains(&Ctx::Id(0)) {
                    return Monad::ret(Ctx::None);
                }
                ctx_ = state.parent(ctx_);
                unvisited_children = state
                    .children(ctx_)
                    .iter()
                    .cloned()
                    .filter(|ctx| !scheduler.visited.contains(&ctx))
                    .collect();
            }

            ctx_ = unvisited_children[0];

            get_leaf(Monad::ret(ctx_), state, scheduler)
        }
    }))
}

fn unvisited_children(ctx: Ctx, state: &mut State, scheduler: &mut Scheduler<'_>) -> Vec<Ctx> {
    let children: Vec<Ctx> = state
        .children(ctx)
        .iter()
        .cloned()
        .filter(|ctx| !scheduler.visited.contains(&ctx))
        .collect();
    children
}

fn is_leaf(ctx: Ctx, state: &mut State) -> bool {
    state.children(ctx).len() == 0
}

fn schedule_note(
    scheduler: &mut Scheduler,
    pcs: Vec<Pc>,
    register: Register,
    velocities: Vec<Velocity>,
    beats: f64,
) {
    let register = if let Register::Reg(register) = register {
        register
    } else {
        4 as i8
    };
    for (pc, velocity) in pcs.into_iter().zip(velocities.into_iter().cycle()) {
        if !matches!(pc, Pc::None) {
            let key = u7::new(((register + 1) * 12 + pc.to_i8()) as u8);
            let vel = u7::new(velocity.0);
            let time = scheduler.clock;
            scheduler.add_instruction(time, Instruction::Midi(MidiMessage::NoteOn { key, vel }));

            let stop = time + f64::round(beats * PPQ.as_int() as f64) as u64;

            // dbg!(&stop);

            // pause();

            scheduler.add_instruction(
                stop,
                Instruction::Midi(MidiMessage::NoteOn { key, vel: 0.into() }),
            );
        }
    }
}

fn render_tracks<'a>(scheduler: &mut Scheduler<'a>) -> Vec<Track<'a>> {
    let mut tracks = Vec::<Track>::new();
    let mut track = Track::new();
    let mut delta = u28::new(0);
    let mut prev_time: u64 = 0;
    for (time, instructions) in scheduler.schedule.iter() {
        // dbg!(time, prev_time);
        delta = u28::new(*time as u32 - prev_time as u32);
        // dbg!(&instructions);
        for instruction in instructions.iter() {
            let kind = match *instruction {
                Instruction::Midi(message) => TrackEventKind::Midi {
                    channel: u4::new(0),
                    message,
                },
                Instruction::Meta(message) => TrackEventKind::Meta(message),
            };
            let event = TrackEvent { delta, kind };
            delta = u28::new(0);
            track.push(event);
        }
        prev_time = *time;
    }

    track.push(TrackEvent {
        delta,
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    tracks.push(track);
    // dbg!(&tracks);
    tracks
}

fn pause() {
    let _ = std::io::stdin().read_line(&mut String::new());
}
