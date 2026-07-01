#![allow(unused)]
#![forbid(clippy::infinite_loop)]
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::Display,
    io::{stderr, stdout},
    iter::repeat_n,
    ops::Div,
};

use crate::compiler::{
    ast::NOOP,
    codegen::{
        utils::{TextStyle::*, gcd, length_to_ticks, out, progress},
        *,
    },
    composer::{State, print_state},
    functional::*,
    scheduler,
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

    fn to(&mut self, ticks: u64) {
        self.clock = ticks;
    }

    fn ticks(&self) -> u64 {
        self.clock
    }

    fn reset(&mut self) {
        self.clock = 0;
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
    // dbg!(&state);
    let ctx = Ctx::Root;
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
    schedule_context(ctx, &mut state, &mut scheduler);
    // dbg!(&scheduler);
    let tracks = render_tracks(&mut scheduler);
    // dbg!(&tracks);
    // pause();

    let smf = Smf { header, tracks };
    smf.to_static()
}

fn schedule_context<'a>(ctx: Ctx, state: &mut State, scheduler: &mut Scheduler<'a>) -> Length {
    // dbg!();
    // print_state(state, ctx);
    let children = &mut state.children(ctx);
    let has_children = children.len() > 0;

    match state.scope_type(ctx) {
        ScopeType::None => state
            .children(ctx)
            .iter()
            .cloned()
            .fold(Length::default(), |mut length, ctx| {
                length + schedule_context(ctx, state, scheduler)
            }),
        ScopeType::Sequence => {
            // let ticks = get_ticks(ctx, state);
            // let playhead = ticks_to_length(scheduler.ticks(), state.tempo(ctx));

            if has_children {
                let len = children.len();

                children.iter().cloned().enumerate().fold(
                    Length::default_max(),
                    |mut length, (idx, ctx)| {
                        // print_state(state, ctx);
                        let length_ = schedule_context(ctx, state, scheduler);
                        if idx < len - 1 {
                            scheduler
                                .forward(length_to_ticks(length_.min(length), state.tempo(ctx)));
                        }
                        dbg!(&length_, scheduler.ticks());
                        length.min(length_)
                    },
                )
            } else {
                let length = state
                    .pcs(ctx)
                    .iter()
                    .cloned()
                    .zip(state.lengths(ctx).iter().cloned())
                    .zip(state.velocities(ctx).iter().cloned())
                    .fold(
                        Length::default_max(),
                        |mut length_, ((pc, length), velocity)| {
                            schedule_note(
                                scheduler,
                                vec![pc],
                                state.register(ctx),
                                vec![velocity],
                                vec![length_to_beats(length, state.tempo(ctx))],
                            );
                            scheduler
                                .forward(length_to_ticks(length_.min(length), state.tempo(ctx)));
                            length_.min(length)
                            // length
                        },
                    );
                // scheduler.rewind(length_to_ticks(length, state.tempo(ctx)));
                length
            }
        }
        ScopeType::Stack => {
            if has_children {
                children
                    .iter()
                    .cloned()
                    .fold(Length::default_max(), |length, ctx| {
                        // print_state(state, ctx);
                        length.min(schedule_context(ctx, state, scheduler))
                    })
            } else {
                print_state(state, ctx);
                let lengths = state.lengths(ctx);
                let beats: Vec<f64> = lengths
                    .iter()
                    .cloned()
                    .map(|length| length_to_beats(length, state.tempo(ctx)))
                    .collect();
                schedule_note(
                    scheduler,
                    state.pcs(ctx),
                    state.register(ctx),
                    state.velocities(ctx),
                    beats,
                );
                // ticks_to_length(get_ticks(ctx, state), state.tempo(ctx))
                lengths.iter().cloned().min().unwrap()
            }
        }
        _ => todo!(),
    }

    // let scope = state.scope_type(ctx);
    // let children = &mut state.children(ctx);
    // let has_children = children.len() > 0;

    // let ticks = if has_children {
    //     let mut child_iter = children.iter().cloned();
    //     let init = get_ticks(child_iter.next().unwrap(), state);
    //     children
    //         .iter()
    //         .cloned()
    //         .fold(init, |a, b| gcd(a, get_ticks(b, state)))
    // } else {
    //     get_ticks(ctx, state)
    // };

    // dbg!(ticks);

    // match scope {
    //     ScopeType::None => children.iter().for_each(|ctx_| {
    //         // print_state(state, *ctx_);
    //         schedule_context(*ctx_, state, scheduler);
    //     }),
    //     ScopeType::Sequence => {
    //         let tempo = state.tempo(ctx);
    //         let beats: Vec<f64> = state
    //             .lengths(ctx)
    //             .iter()
    //             .map(|length| length.as_f64() / tempo.0 as f64)
    //             .collect();
    //         let register = state.register(ctx);

    //         // dbg!();
    //         if has_children {
    //             let mut children = get_counters(
    //                 state,
    //                 children,
    //                 ticks_to_length(scheduler.ticks(), state.tempo(ctx)),
    //             );
    //             // eprintln!("{IntensePurple}COUNTERS: {:?} {ResetColor}", &children);

    //             let end = ticks_to_length(scheduler.ticks(), tempo)
    //                 + state.lengths(ctx).iter().cloned().sum();

    //             let mut lengths = state.lengths(ctx).into_iter();
    //             let mut playhead = ticks_to_length(scheduler.ticks(), tempo);
    //             // children
    //             //     .iter()
    //             //     .for_each(|(ctx, _)| schedule_context(*ctx, state, scheduler));
    //             while playhead < end {
    //                 out(
    //                     0,
    //                     35,
    //                     format!(
    //                         "{IntensePurple}PLAYHEAD: {} END: {}{ResetColor}",
    //                         playhead.as_u64(),
    //                         end.as_u64()
    //                     ),
    //                 );
    //                 progress(playhead.as_u32(), end.as_u32(), 36);

    //                 // dbg!(&playhead, &end);
    //                 children.iter_mut().for_each(|(ctx, counters)| {
    //                     let lengths = state.lengths(*ctx);
    //                     counters.iter_mut().zip(lengths.into_iter()).for_each(
    //                         |(counter, length)| {
    //                             // eprintln!(
    //                             //     "{IntensePurple}COUNTER: {}{ResetColor}",
    //                             //     counter.as_u64()
    //                             // );
    //                             if *counter == playhead {
    //                                 schedule_context(*ctx, state, scheduler);
    //                                 // *counter += length;

    //                                 // scheduler.forward(length_to_ticks(length, state.tempo(*ctx)));
    //                                 // playhead += length;
    //                                 // eprintln!(
    //                                 //     "{IntensePurple}PLAYHEAD: {}\nTICKS: {}{ResetColor}",
    //                                 //     playhead.as_u64(),
    //                                 //     scheduler.ticks()
    //                                 // );
    //                             }
    //                         },
    //                     );
    //                 });
    //                 scheduler.forward(ticks);
    //                 playhead += ticks_to_length(ticks, state.tempo(ctx));
    //             }
    //         } else {
    //             state
    //                 .pcs(ctx)
    //                 .iter()
    //                 .zip(beats.iter())
    //                 .zip(state.velocities(ctx).iter())
    //                 .zip(state.lengths(ctx).iter())
    //                 .for_each(|(((pc, beat), velocity), length)| {
    //                     // eprintln!("{IntensePurple}TIME: {}{ResetColor}", scheduler.ticks());
    //                     // schedule_note(scheduler, vec![*pc], register, vec![*velocity], vec![*beat]);
    //                     // dbg!();

    //                     scheduler.forward(length_to_ticks(*length, state.tempo(ctx)));
    //                     // scheduler.forward(ticks);

    //                     // playhead = ticks_to_length(scheduler.ticks(), state.tempo(ctx));
    //                 });
    //             // print_state(state, ctx);
    //         }
    //     }
    //     ScopeType::Stack => {
    //         // dbg!();

    //         // dbg!(has_children);
    //         if has_children {
    //             let mut children = get_counters(
    //                 state,
    //                 children,
    //                 ticks_to_length(scheduler.ticks(), state.tempo(ctx)),
    //             );
    //             // eprintln!(
    //             //     "{IntensePurple}TIME (TICKS): {}{ResetColor}",
    //             //     scheduler.ticks()
    //             // );
    //             children.iter_mut().for_each(|(ctx, counters)| {
    //                 // eprintln!("{IntensePurple}COUNTERS: {counters:?}{ResetColor}");
    //                 // schedule_context(*ctx, state, scheduler);
    //                 let lengths = state.lengths(*ctx);
    //                 counters
    //                     .iter_mut()
    //                     .zip(lengths.iter().cloned())
    //                     .for_each(|(counter, length)| *counter += length);
    //                 // eprintln!("{IntensePurple}COUNTERS: {counters:?}{ResetColor}");
    //             });
    //         } else {
    //             let tempo = state.tempo(ctx);
    //             let beats: Vec<f64> = state
    //                 .lengths(ctx)
    //                 .iter()
    //                 .map(|length| length.as_f64() / state.tempo(ctx).0 as f64)
    //                 .collect();
    //             schedule_note(
    //                 scheduler,
    //                 state.pcs(ctx),
    //                 state.register(ctx),
    //                 state.velocities(ctx),
    //                 beats,
    //             );
    //             // print_state(state, ctx);
    //         }
    //     }
    //     _ => todo!(),
    // }
}

fn length_to_beats(length: Length, tempo: Mpb) -> f64 {
    length.as_f64() / tempo.0 as f64
}

fn ticks_to_length(ticks: u64, tempo: Mpb) -> Length {
    Length::MicroSeconds((ticks as f64 / PPQ.as_int() as f64 * tempo.0 as f64) as u64)
}

fn get_counters(
    state: &mut State,
    children: &mut Vec<Ctx>,
    mut offset: Length,
) -> Vec<(Ctx, Vec<Length>)> {
    let mut children: Vec<(Ctx, Vec<Length>)> = children
        .into_iter()
        .map(|ctx| {
            let len = state.lengths(*ctx).len();
            let t = (ctx.clone(), repeat_n(offset, len).collect::<Vec<Length>>());
            offset += ticks_to_length(get_ticks(*ctx, state), state.tempo(*ctx));
            t
        })
        .collect();
    // eprintln!("{IntenseGreen}CHILDREN: {children:?}{ResetColor}");
    children
}

fn get_ticks(ctx: Ctx, state: &mut State) -> u64 {
    let lengths = state.lengths(ctx);
    // dbg!(ctx, &lengths);
    let mut lengths_iter = lengths.iter().cloned();
    let init = lengths_iter.next().unwrap();
    length_to_ticks(lengths_iter.fold(init, |a, b| gcd(a, b)), state.tempo(ctx))
}

fn schedule_note(
    scheduler: &mut Scheduler,
    pcs: Vec<Pc>,
    register: Register,
    velocities: Vec<Velocity>,
    beats: Vec<f64>,
) {
    let register = if let Register::Reg(register) = register {
        register
    } else {
        4 as i8
    };
    for ((pc, beat), velocity) in pcs
        .into_iter()
        .zip(beats.into_iter())
        .zip(velocities.into_iter().cycle())
    {
        if !matches!(pc, Pc::None) {
            let key = u7::new(((register + 1) * 12 + pc.to_i8()) as u8);
            let vel = u7::new(velocity.0);
            let time = scheduler.clock;
            scheduler.add_instruction(time, Instruction::Midi(MidiMessage::NoteOn { key, vel }));
            // dbg!(beats);
            let stop = time + f64::round(beat * PPQ.as_int() as f64) as u64;

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
    dbg!(&scheduler.schedule);
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
