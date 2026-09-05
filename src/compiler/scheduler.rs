#![allow(unused)]
#![forbid(clippy::infinite_loop)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env::temp_dir,
    fmt::Display,
    io::{stderr, stdout},
    iter::repeat_n,
    ops::Div,
    u64,
};

use crate::{
    color,
    compiler::{
        ast::NOOP,
        codegen::{
            state::State,
            utils::{TextStyle::*, *},
            *,
        },
        functional::*,
        scheduler,
    },
};

use midly::PitchBend;
pub use midly::{
    Format, Header, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind, num::*,
};
use num_rational::Ratio;
use num_traits::{FromPrimitive, ToPrimitive};

pub type Ticks = u64;

#[derive(Debug)]
pub struct Scheduler {
    prog: u8,
    tempo: Mpb,
    clock: Ticks,
    times: BTreeSet<Ticks>,
    schedule: BTreeMap<Ticks, Vec<Instruction>>,
    instruments: HashMap<Prog, Vec<u8>>,
    visited: HashSet<Ctx>,
    event_idx: usize,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            prog: 0,
            tempo: Mpb(Ratio::<u64>::from_f64(60_000_000. / 120.).unwrap()),
            clock: 0,
            times: BTreeSet::new(),
            schedule: BTreeMap::<Ticks, Vec<Instruction>>::new(),
            instruments: HashMap::new(),
            visited: HashSet::<Ctx>::new(),
            event_idx: 0,
        }
    }
}

impl Scheduler {
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

    fn add_instruction(&mut self, time: u64, instruction: Instruction) {
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
        if mpb.0 != self.tempo.0 {
            let time = self.clock;
            let tempo = u24::new(mpb.0.to_u32().unwrap());
            self.add_instruction(time, Instruction::Meta(MetaMessage::Tempo(tempo)));
            self.tempo = mpb;
        }
    }

    pub fn set_program(&mut self, program: Prog, state: &State) {
        let prog = &mut self.prog;
        if *prog != program.0 {
            *prog = program.0;
            let time = self.clock;
            self.add_instruction(
                time,
                Instruction::Midi(MidiMessage::ProgramChange {
                    program: u7::from_int_lossy(program.0),
                }),
            );
            if let Some(bytes) = state.instruments().get(&program) {
                info(String::from_utf8(bytes.clone()).unwrap_or_default());
                pause();

                self.add_instruction(0, Instruction::Meta(MetaMessage::InstrumentName(bytes.clone())));
            }
        }
    }

    pub fn set_instrument(&mut self, name: Vec<u8>) {
        self.add_instruction(0, Instruction::Meta(MetaMessage::InstrumentName(name)));
    }

    fn schedule_mut(&mut self) -> &mut BTreeMap<Ticks, Vec<Instruction>> {
        &mut self.schedule
    }

    pub fn schedule(&self) -> BTreeMap<Ticks, Vec<Instruction>> {
        self.schedule.clone()
    }

    pub fn instruments(&self) -> &HashMap<Prog, Vec<u8>> {
        &self.instruments
    }
}

pub fn schedule<'a>(mut state: State) -> Smf<'a> {
    status(format!("{}SCHEDULING", color!()));
    // dbg!(&state);

    let ctx = Ctx::Root;
    // graph(&state, ctx, 2);
    let mut scheduler = Scheduler::default();
    let sch = scheduler.schedule();

    let header = Header::new(Format::SingleTrack, Timing::Metrical(PPQ));

    let prog = state
        .programs(Ctx::Root)
        .unwrap_or_default()
        .last()
        .cloned()
        .unwrap_or_default()
        .last()
        .cloned()
        .unwrap_or_default();

    scheduler.add_instruction(
        0,
        Instruction::Midi(MidiMessage::ProgramChange {
            program: u7::new(prog.0),
        }),
    );

    if let Some(bytes) = state.instruments().get(&prog) {
        scheduler.add_instruction(0, Instruction::Meta(MetaMessage::InstrumentName(bytes.clone())));
    } else {
        scheduler.add_instruction(0, Instruction::Meta(MetaMessage::InstrumentName(b"Piano".to_vec())));
    }

    scheduler.add_instruction(
        0,
        Instruction::Meta(MetaMessage::Tempo(u24::new(state.tempo().0.to_u32().unwrap()))),
    );

    let timeline = state.timeline();

    let mut delta_time: u64 = 0;
    let mut prev_time = Length::default();
    timeline.iter().for_each(|(time, slices)| {
        delta_time = length_to_ticks(time.clone() - prev_time.clone(), scheduler.tempo());
        scheduler.forward(delta_time);
        time_log(format!("{}T:{}", color!(), time.as_u64()));
        slices.iter().for_each(|slice| {
            let (pcs, registers, lengths, velocities, tempos, programs) = slice.data();
            scheduler.set_program(programs.first().cloned().unwrap_or_default(), &state);
            scheduler.set_tempo(tempos.first().cloned().unwrap_or_default());

            schedule_note(&mut scheduler, pcs.to_vec(), registers.to_vec(), velocities.to_vec(), lengths.to_vec(), tempos.to_vec());

        });
        prev_time = time.clone();


    });

    dbg!(scheduler.ticks(), length_to_ticks(prev_time.clone(), scheduler.tempo()));
    // delta_time = length_to_ticks(prev_time.clone(), scheduler.tempo()) - scheduler.ticks();
    scheduler.forward(delta_time);

    let mut delta_ticks: u64 = 0;

    let ticks = scheduler.ticks();
    scheduler.add_instruction(ticks, Instruction::Meta(MetaMessage::EndOfTrack));

    let mut schedule = BTreeMap::from_iter(scheduler.schedule_mut().iter_mut().map(
        |(ticks, instructions)| {
            (
                *ticks,
                instructions.iter_mut().collect::<Vec<&mut Instruction>>(),
            )
        },
    ));

    let tracks = render_tracks(&mut schedule);

    let mut iter = sch.iter();
    iter.for_each(|(ticks, instructions)| {
        instructions.iter().cloned().for_each(|instruction| {
            if let Instruction::Midi(MidiMessage::NoteOn { key, vel }) = instruction {
                time_log(format!("{}TICKS: {ticks} KEY: {key} VEL: {vel}", color!()));
            }
        });
    });

    let track = tracks.first().cloned().unwrap_or_default();
    let mut time: usize = 0;

    print_track(track, time);
    // pause();
    let smf = Smf { header, tracks };
    smf.to_static()
}

fn print_track(track: Vec<TrackEvent<'_>>, mut time: usize) {
    track.iter().cloned().enumerate().for_each(|(idx, e)| {
        time += e.delta.as_int() as usize;
        if let TrackEventKind::Midi { message, .. } = e.kind {
            if let MidiMessage::NoteOn { key, vel } = message {
                // dbg!(vel.as_int());
                // pause();
                if vel.as_int() > 0 {
                time_log(format!(
                    "{}{idx}: T: {time} Δ: {} KEY: {} VEL: {}",
                    color!(),
                    e.delta.as_int(),
                    key.as_int(),
                    vel.as_int()
                ));}
            }
        }
    });
}

fn schedule_context<'a>(ctx: Ctx, state: &mut State, scheduler: &mut Scheduler) {
    status(format!("{}SCHEDULE CONTEXT {ctx:?}", color!()));
    // eprintln!("{}{}", color!(), state_str(state, ctx));
    // print_state(state, ctx);

    // scheduler.set_tempo(state.tempo());
    // let mut iter = state.lengths(ctx).into_iter();
    // let mut first = iter.next().unwrap_or_default().into_iter();
    // let first_init = first.next().unwrap_or_default();
    // let length = iter.fold(first_init, |step, lengths| {
    //     let mut iter = lengths.iter().cloned();
    //     let init = iter.next().unwrap_or_default();
    //     gcd(step, iter.fold(init, |step, length| gcd(step, length)))
    // });
    // let step = length_to_ticks(length, scheduler.tempo);
    // scheduler.times.insert(0);
    // eprintln!("{IntensePurple}ID: {}, SCOPE: {:?}{ResetColor}", ctx.to_usize(), state.scope(ctx));
    // let total_length = length_to_ticks(state.get_ctx_length(ctx), scheduler.tempo);
    // let mut playhead: u64 = 0;

    // let mut next = scheduler.times.pop_first().unwrap_or_default();
    // let mut ctx_iter = state
    //     .pcs(ctx)
    //     .into_iter()
    //     .zip(state.lengths(ctx).into_iter())
    //     .zip(state.velocities(ctx).into_iter())
    //     .zip(state.registers(ctx).into_iter())
    //     .zip(state.tempos(ctx).into_iter())
    //     .zip(state.programs(ctx).into_iter());


    // while scheduler.clock <= total_length {
        // dbg!(step, total_length, next, scheduler.clock);
        // if scheduler.clock >= next {
        //     if let Some((((((pcs, lengths), velocities), registers), tempos), programs)) =
        //         ctx_iter.next()
        //     {
        //         scheduler.set_program(programs.last().cloned().unwrap_or_default());
        //         scheduler.set_tempo(tempos.iter().cloned().min().unwrap_or_default());
        //         schedule_note(
        //             scheduler,
        //             pcs,
        //             registers,
        //             velocities,
        //             lengths.clone(),
        //             tempos,
        //         );
        //         lengths.into_iter().for_each(|length| {
        //             scheduler
        //                 .times
        //                 .insert(next + length_to_ticks(length, scheduler.tempo));
        //         });
        //     }
        // }
        match state.scope(ctx) {
            Scope::Sequence => {
                let start = scheduler.ticks();
                state
                    .pcs(ctx)
                    .unwrap_or_default()
                    .iter()
                    .cloned()
                    .zip(state.lengths(ctx).unwrap_or_default().iter().cloned())
                    .zip(state.velocities(ctx).unwrap_or_default().iter().cloned())
                    .zip(state.registers(ctx).unwrap_or_default().iter().cloned())
                    .zip(state.tempos(ctx).unwrap_or_default().iter().cloned())
                    .zip(state.programs(ctx).unwrap_or_default().iter().cloned())
                    .for_each(
                        |(((((pcs, lengths), velocities), registers), tempos), programs)| {
                            let time = scheduler.ticks().clone();
                            // eprintln!("{IntenseBlue}T: {time} PC: {}, REG: {}, VEL: {}{ResetColor}", pc.as_u8(), register.as_i8(), velocity.0);
                            let prog = programs.last().cloned().unwrap_or_default();

                            scheduler.set_program(programs.last().cloned().unwrap_or_default(), &state);

                            if let Some(name) = scheduler.instruments().get(&prog) {
                                scheduler.set_instrument(name.clone());
                            }
                            scheduler.set_tempo(tempos.iter().cloned().min().unwrap_or_default());
                            schedule_note(
                                scheduler,
                                pcs,
                                registers,
                                velocities,
                                lengths.clone(),
                                tempos.clone(),
                            );

                            scheduler.forward(length_to_ticks(
                                lengths.iter().cloned().min().unwrap_or_default(),
                                tempos.iter().cloned().min().unwrap_or_default(),
                            ));
                        },
                    );
            }
            Scope::Stack => {
                // todo!();
                let tempos = state.tempos(ctx).unwrap_or_default();
                schedule_note(
                    scheduler,
                    state.pcs(ctx).unwrap_or_default().first().cloned().unwrap_or_default(),
                    state.registers(ctx).unwrap_or_default().first().cloned().unwrap_or_default(),
                    state.velocities(ctx).unwrap_or_default().first().cloned().unwrap_or_default(),
                    state.lengths(ctx).unwrap_or_default().first().cloned().unwrap_or_default(),
                    state.tempos(ctx).unwrap_or_default().first().cloned().unwrap_or_default(),
                );
            }
            _ => {
                // eprintln!("{IntenseBlue}{:?}{ResetColor}", state.lengths(ctx))
                todo!()
            }
        }
        // next += scheduler.times.pop_first().unwrap_or_default();
        // dbg!(next);
        // scheduler.forward(step);
    // }
}

fn length_to_beats(length: Length, tempo: Mpb) -> f64 {
    length.as_f64() / tempo.0.to_f64().unwrap()
}

fn ticks_to_length(ticks: u64, tempo: Mpb) -> Length {
    Length::MicroSeconds(Ratio::<u64>::from_f64(ticks as f64 / PPQ.as_int() as f64 * tempo.0.to_f64().unwrap()).unwrap())
}

// fn get_lengths(ctx: Ctx, state: &State) -> Vec<Length> {
//     let mut lengths = state.lengths(ctx);
//     lengths.append(
//         &mut state
//             .children(ctx)
//             .iter()
//             .cloned()
//             .flat_map(|ctx| get_lengths(ctx, state))
//             .collect(),
//     );
//     lengths
// }

// fn get_ctx_length(ctx: Ctx, state: &State) -> Length {
//     match state.scope(ctx) {
//         Scope::Sequence => {
//             if state.children(ctx).is_empty() {
//                 state.lengths(ctx).iter().cloned().sum()
//             } else {
//                 state
//                     .children(ctx)
//                     .iter()
//                     .cloned()
//                     .map(|ctx| get_ctx_length(ctx, state))
//                     .sum()
//             }
//         }
//         Scope::Stack => {
//             if state.children(ctx).is_empty() {
//                 state.lengths(ctx).iter().cloned().min().unwrap()
//             } else {
//                 state
//                     .children(ctx)
//                     .iter()
//                     .cloned()
//                     .map(|ctx| get_ctx_length(ctx, state))
//                     .min()
//                     .unwrap()
//             }
//         }
//         _ => todo!(),
//     }
// }

fn get_ticks(ctx: Ctx, state: &mut State) -> u64 {
    let lengths = state.lengths(ctx).unwrap_or_default();
    let tempos = state.tempos(ctx).unwrap_or_default();
    let tempo_iter = tempos.iter().cloned();
    let mut iter = lengths.iter().cloned().zip(tempo_iter);
    let first = iter.next().unwrap_or_default();
    let mut iter_ = first.0.into_iter().zip(first.1.into_iter());
    let init_ = iter_.next().unwrap_or_default();
    let init = iter_.fold(init_, |(a, tempo_a), (b, tempo_b)| {
        (a.clone().min(b.clone()), if a.clone().min(b.clone()) < b { tempo_a } else { tempo_b })
    });

    let (length, tempo) = iter.fold(init, |(length, tempo), (lengths, tempos)| {
        let mut iter = lengths.iter().cloned().zip(tempos.iter().cloned());
        let init = iter.next().unwrap_or_default();
        iter.fold(init, |(a, tempo_a), (b, tempo_b)| {
            (a.clone().min(b.clone()), if a.min(b.clone()) < b { tempo_a } else { tempo_b })
        })
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
    tempos: Vec<Mpb>,
) {

    // dbg!(&lengths, &tempos);

    let beats: Vec<f64> = lengths
        .iter()
        .cloned()
        .zip(tempos.iter().cloned().cycle())
        .map(|(length, tempo)| length_to_beats(length, tempo))
        .collect();

    for (((pc, beat), velocity), register) in pcs
        .into_iter()
        .zip(beats.into_iter().cycle())
        .zip(velocities.into_iter().cycle())
        .zip(registers.into_iter().cycle())
    {
        let key = u7::new(((register + 1) as u8 * 12 + f64::floor(pc.as_f64()) as u8));
        let vel = if matches!(pc, Pc::None) {
            u7::new(0)
        } else {
            u7::new(velocity.0)
        };
        let time = scheduler.ticks();

        scheduler.add_instruction(time, Instruction::Midi(MidiMessage::NoteOn { key, vel }));

        let rem = f64::ceil(pc.as_f64() - f64::floor(pc.as_f64()) * 127.) as u16;

        if rem > 0 {
            let bend = u14::new(0x2000 + rem);
            scheduler.add_instruction(
                time,
                Instruction::Midi(MidiMessage::PitchBend {
                    bend: PitchBend(bend),
                }),
            );
        }

        let length = f64::floor(beat * PPQ.as_int() as f64) as u64;
        let stop = time + length;
        let idx = scheduler.event_index().clone();
        eprintln!("{IntenseRed}{idx}: TIME: {time} KEY: {key} VEL: {vel} LEN: {length}{ResetColor}");

        scheduler.add_instruction(
            stop,
            Instruction::Midi(MidiMessage::NoteOn { key, vel: 0.into() }),
        );

        let idx = scheduler.event_index().clone();
        let time = scheduler.ticks();
        eprintln!("{}{idx}: TIME: {stop} KEY: {key} VEL: 0", color!());
    }
}

fn render_tracks<'a>(
    schedule: &'a mut BTreeMap<u64, Vec<&'a mut Instruction>>,
) -> Vec<Vec<TrackEvent<'a>>> {
    let mut tracks = Vec::<Track>::new();
    let mut track = Track::new();
    let mut delta = u28::new(0);
    let mut prev_time: u64 = 0;
    // dbg!(&scheduler.schedule);
    for (time, instructions) in schedule {
        // eprintln!("{IntenseYellow}TIME: {time}{ResetColor}");
        // dbg!(time, prev_time);
        delta = u28::new(*time as u32 - (prev_time as u32).min(*time as u32));
        // dbg!(&instructions);

        for instruction in instructions {
            let kind = match *instruction {
                Instruction::Midi(message) => TrackEventKind::Midi {
                    channel: u4::new(0),
                    message: message.clone(),
                },
                Instruction::Meta(message) => TrackEventKind::Meta(match message {
                    MetaMessage::TrackNumber(num) => midly::MetaMessage::TrackNumber(*num),
                    MetaMessage::Text(items) => midly::MetaMessage::Text(items.as_slice()),
                    MetaMessage::Copyright(items) => {
                        midly::MetaMessage::Copyright(items.as_slice())
                    }
                    MetaMessage::TrackName(items) => {
                        midly::MetaMessage::TrackName(items.as_slice())
                    }
                    MetaMessage::InstrumentName(items) => {
                        midly::MetaMessage::InstrumentName(items.as_slice())
                    }
                    MetaMessage::Lyric(items) => midly::MetaMessage::Lyric(items.as_slice()),
                    MetaMessage::Marker(items) => midly::MetaMessage::Marker(items.as_slice()),
                    MetaMessage::CuePoint(items) => midly::MetaMessage::CuePoint(items.as_slice()),
                    MetaMessage::ProgramName(items) => {
                        midly::MetaMessage::ProgramName(items.as_slice())
                    }
                    MetaMessage::DeviceName(items) => {
                        midly::MetaMessage::DeviceName(items.as_slice())
                    }
                    MetaMessage::MidiChannel(ch) => midly::MetaMessage::MidiChannel(*ch),
                    MetaMessage::MidiPort(p) => midly::MetaMessage::MidiPort(*p),
                    MetaMessage::EndOfTrack => midly::MetaMessage::EndOfTrack,
                    MetaMessage::Tempo(tempo) => midly::MetaMessage::Tempo(*tempo),
                    MetaMessage::SmpteOffset(smpte_time) => {
                        midly::MetaMessage::SmpteOffset(*smpte_time)
                    }
                    MetaMessage::TimeSignature(num, den, clicks, notes) => {
                        midly::MetaMessage::TimeSignature(*num, *den, *clicks, *notes)
                    }
                    MetaMessage::KeySignature(accidentals, mode) => {
                        midly::MetaMessage::KeySignature(*accidentals, *mode)
                    }
                    MetaMessage::SequencerSpecific(items) => {
                        midly::MetaMessage::SequencerSpecific(items.as_slice())
                    }
                    MetaMessage::Unknown(byte, items) => {
                        midly::MetaMessage::Unknown(*byte, items.as_slice())
                    }
                }),
            };
            eprintln!("{}T: {time} Δ: {delta} {kind:?}", color!());
            let event = TrackEvent { delta, kind };
            delta = u28::new(0);
            // eprintln!("{Cyan}{event:?}{ResetColor}");
            track.push(event);
        }
        prev_time = *time;
    }

    tracks.push(track);
    // dbg!(&tracks);
    tracks
}

fn pause() {
    let _ = std::io::stdin().read_line(&mut String::new());
}
