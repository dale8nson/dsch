#![allow(unused)]
#![forbid(clippy::infinite_loop)]
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    env::temp_dir,
    fmt::Display,
    io::{stderr, stdout},
    iter::repeat_n,
    ops::Div,
    u64,
};

use crate::compiler::{
    ast::NOOP,
    codegen::{
        state::State,
        utils::{TextStyle::*, *},
        *,
    },
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
    event_idx: usize
}

impl<'a> Default for Scheduler<'a> {
    fn default() -> Self {
        Scheduler {
            prog: 0,
            tempo: Mpb(f64::round(60_000_000. / 120.) as u64),
            clock: 0,
            schedule: BTreeMap::<Ticks, Vec<Instruction>>::new(),
            visited: HashSet::<Ctx>::new(),
            event_idx: 0
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

    pub fn event_index(&mut self) -> &mut usize {
        &mut self.event_idx
    }

    fn add_instruction(&mut self, time: u64, instruction: Instruction<'a>) {
        let instructions = self.schedule_mut().get_mut(&time);
        if instructions.is_some() {
            instructions.unwrap().push(instruction);
        } else {
            let instructions = vec![instruction];
            self.schedule_mut().insert(time, instructions);
        }

        self.event_idx += 1;
    }

    pub fn tempo(&self) -> Mpb {
        self.tempo.clone()
    }

    fn set_tempo(&mut self, mpb: Mpb) {
        if mpb != self.tempo {
            let time = self.clock;
            let tempo = u24::new(mpb.0 as u32);
            self.add_instruction(time, Instruction::Meta(MetaMessage::Tempo(tempo)));
            self.tempo = mpb;
        }
    }

    pub fn set_program(&mut self, program: Prog) {
        if self.prog != program.0 {
            let time = self.clock;
            self.add_instruction(
                time,
                Instruction::Midi(MidiMessage::ProgramChange {
                    program: u7::from_int_lossy(program.0),
                }),
            );
            self.prog = program.0;
        }
    }

    fn schedule_mut(&mut self) -> &mut BTreeMap<Ticks, Vec<Instruction<'a>>> {
        &mut self.schedule
    }

    pub fn schedule(&self) -> BTreeMap<Ticks, Vec<Instruction<'a>>> {
        self.schedule.clone()
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

    let timeline = state.timeline().clone();
    let mut delta_ticks: u64 = 0;

    timeline.into_iter().for_each(|(t, ctx)| {
        // print_state(&state, ctx);
        let ticks = scheduler.ticks();

        let t = length_to_ticks(
            Length::MicroSeconds(t),
            state.tempos(ctx).last().cloned().unwrap_or_default(),
        );
        // eprintln!("{IntenseGreen}T: {t} TICKS: {ticks}{ResetColor}");
        // dbg!(t, ticks);

        delta_ticks = t - ticks;
        scheduler.forward(delta_ticks);
        schedule_context(ctx, &mut state, &mut scheduler);
    });

    // dbg!(scheduler.ticks());

    // schedule_context(ctx, &mut state, &mut scheduler);
    // dbg!(&scheduler);
    let tracks = render_tracks(&mut scheduler);
    // dbg!(&tracks);
    // pause();

    let sch = scheduler.schedule();
    let mut iter = sch.iter();
    iter.for_each(|(ticks, instructions)| {
        instructions.iter().cloned().for_each(|instruction| {
            if let Instruction::Midi(MidiMessage::NoteOn { key, vel }) = instruction {
                eprintln!("{IntenseBoldBlue}TICKS: {ticks} KEY: {key} VEL: {vel}{ResetColor}");
            }
        });
    });

    let events = tracks.first().cloned().unwrap_or_default();
    let mut time: usize = 0;

    events.iter().cloned().enumerate().for_each(|(idx, e)| {
    if let TrackEventKind::Midi { message, .. } = e.kind {
        if let MidiMessage::NoteOn { key, vel } = message {
            eprintln!("{IntensePurple}{idx}: T: {time} KEY: {} VEL: {}{ResetColor}", key.as_int(), vel.as_int());
        }
    }
        time += e.delta.as_int() as usize;
    });

    let smf = Smf { header, tracks };
    smf.to_static()
}

fn schedule_context<'a>(ctx: Ctx, state: &mut State, scheduler: &mut Scheduler<'a>) {
    // print_state(state, ctx);

    scheduler.set_program(state.programs(ctx).last().cloned().unwrap_or_default());
    scheduler.set_tempo(state.tempos(ctx).last().cloned().unwrap_or_default());
    eprintln!("{IntensePurple}ID: {}, SCOPE: {:?}{ResetColor}",ctx.to_usize(), state.scope(ctx));
    match state.scope(ctx) {
        Scope::Sequence => {
            let start = scheduler.ticks();
            state
                .pcs(ctx)
                .iter()
                .cloned()
                .zip(state.lengths(ctx).iter().cloned())
                .zip(state.velocities(ctx).iter().cloned())
                .zip(state.registers(ctx).iter().cloned())
                .zip(state.tempos(ctx).iter().cloned())
                .zip(state.programs(ctx).iter().cloned())
                .for_each(|(((((pc, length), velocity), register), tempo), program)| {
                    let time = scheduler.ticks().clone();
                    eprintln!("{IntenseBlue}T: {time} PC: {}, REG: {}, VEL: {}{ResetColor}", pc.as_u8(), register.as_i8(), velocity.0);
                    scheduler.set_program(program);
                    scheduler.set_tempo(tempo);
                    schedule_note(
                        scheduler,
                        vec![pc],
                        vec![register],
                        vec![velocity],
                        vec![length],
                        vec![tempo]
                    );
                    // scheduler.forward(length_to_ticks(
                    //     state
                    //         .lengths(ctx)
                    //         .iter()
                    //         .cloned()
                    //         .fold(Length::None, |step, length| gcd(step, length)),
                    //     tempo,
                    // ));

                    scheduler.forward(length_to_ticks(length, tempo));

                    // length
                });
        }
        Scope::Stack => {
            let tempos = state.tempos(ctx);
            schedule_note(
                scheduler,
                state.pcs(ctx),
                state.registers(ctx),
                state.velocities(ctx),
                state.lengths(ctx),
                state.tempos(ctx)
            );
        }
        _ => {
            // eprintln!("{IntenseBlue}{:?}{ResetColor}", state.lengths(ctx))
            todo!()
        },
    }
}

fn length_to_beats(length: Length, tempo: Mpb) -> f64 {
    length.as_f64() / tempo.0 as f64
}

fn ticks_to_length(ticks: u64, tempo: Mpb) -> Length {
    Length::MicroSeconds((ticks as f64 / PPQ.as_int() as f64 * tempo.0 as f64) as u64)
}

fn get_lengths(ctx: Ctx, state: &State) -> Vec<Length> {
    let mut lengths = state.lengths(ctx);
    lengths.append(
        &mut state
            .children(ctx)
            .iter()
            .cloned()
            .flat_map(|ctx| get_lengths(ctx, state))
            .collect(),
    );
    lengths
}

fn get_ctx_length(ctx: Ctx, state: &State) -> Length {
    match state.scope(ctx) {
        Scope::Sequence => {
            if state.children(ctx).is_empty() {
                state.lengths(ctx).iter().cloned().sum()
            } else {
                state
                    .children(ctx)
                    .iter()
                    .cloned()
                    .map(|ctx| get_ctx_length(ctx, state))
                    .sum()
            }
        }
        Scope::Stack => {
            if state.children(ctx).is_empty() {
                state.lengths(ctx).iter().cloned().min().unwrap()
            } else {
                state
                    .children(ctx)
                    .iter()
                    .cloned()
                    .map(|ctx| get_ctx_length(ctx, state))
                    .min()
                    .unwrap()
            }
        }
        _ => todo!(),
    }
}

fn get_ticks(ctx: Ctx, state: &mut State) -> u64 {
    let lengths = get_lengths(ctx, state);
    let tempos = state.tempos(ctx);
    let tempo_iter = tempos.iter().cloned();
    let mut lengths_iter = lengths.iter().cloned().zip(tempo_iter);
    let init = lengths_iter.next().unwrap_or_default();

    let (length, tempo) = lengths_iter.fold(init, |(a, tempo_a), (b, tempo_b)| {
        (a.min(b), if a.min(b) < b { tempo_a } else { tempo_b })
    });

    length_to_ticks(length, tempo)
    // length_to_ticks(lengths_iter.fold(init, |a, b| gcd(a, b)), state.tempo(ctx))
}

fn schedule_note(
    scheduler: &mut Scheduler,
    pcs: Vec<Pc>,
    registers: Vec<Register>,
    velocities: Vec<Velocity>,
    lengths: Vec<Length>,
    tempos: Vec<Mpb>
) {

    let beats: Vec<f64> = lengths
    .iter()
    .cloned()
    .zip(tempos.iter().cloned())
    .map(|(length, tempo)| length_to_beats(length, tempo))
    .collect();

    for (((pc, beat), velocity), register) in pcs
        .into_iter()
        .zip(beats.into_iter())
        .zip(velocities.into_iter().cycle())
        .zip(registers.into_iter().cycle())
    {
        let key = u7::new(((register + 1) as u8 * 12 + pc.as_u8()));
        let vel = if matches!(pc, Pc::None) {
            u7::new(0)
        } else {
            u7::new(velocity.0)
        };
        let time = scheduler.ticks();

        scheduler.add_instruction(time, Instruction::Midi(MidiMessage::NoteOn { key, vel }));

        let length = f64::round(beat * PPQ.as_int() as f64) as u64;
        let stop = time + length;
        let idx = scheduler.event_index().clone();
        eprintln!("{IntenseRed}{idx}: TIME: {time} KEY: {key} VEL: {vel} LEN: {length}{ResetColor}");

        scheduler.add_instruction(
            stop,
            Instruction::Midi(MidiMessage::NoteOn { key, vel: 0.into() }),
        );

        let idx = scheduler.event_index().clone();
        let time = scheduler.ticks();
        eprintln!("{IntenseRed}{idx}: TIME: {stop} KEY: {key} VEL: 0{ResetColor}");
    }
}

fn render_tracks<'a>(scheduler: &mut Scheduler<'a>) -> Vec<Track<'a>> {
    let mut tracks = Vec::<Track>::new();
    let mut track = Track::new();
    let mut delta = u28::new(0);
    let mut prev_time: u64 = 0;
    // dbg!(&scheduler.schedule);
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
