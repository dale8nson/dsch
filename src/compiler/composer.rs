#![allow(unused, const_item_mutation)]
#![forbid(
    clippy::infinite_loop,
    clippy::maybe_infinite_iter,
    unconditional_recursion
)]
use std::iter::Iterator;

use crate::compiler::{
    ast::*,
    codegen::{
        Scope,
        state::State,
        utils::{TextStyle::*, *},
        *,
    },
    functional::*,
};

use colonnade::{self, Alignment, Colonnade};
use crossterm::{
    cursor::position,
    terminal::{self, size},
};
use rust_sugiyama::{
    configure::{Config, RankingType},
    from_edges,
};

const QUARTER_NOTE_RATIO: f64 = 4.0;

pub fn compose_program(mut program: Program) -> State {
    let mut state = State::default();
    program.exps.push(Exp::EOI);
    let ctx = Ctx::Root;

    state.set_exps(vec![Exp::Compound(Compound::Parens(program.exps))], 0);

    let mut m = Monad::ret((state.next().unwrap_or_default(), Ctx::Root));
    let mut should_break = false;
    while state.len() > 0 {
        m = m.bind(Box::new(|(exp, ctx)| {
            // dbg!(&exp, ctx);
            if matches!(exp, Exp::EOI) {
                should_break = true;
            }
            compose_exp(exp, ctx, &mut state)
        }));
        if should_break {
            break;
        }
    }

    m = m
        .bind(Box::new(|(exp, ctx)| compose_exp(exp, ctx, &mut state)))
        .bind(Box::new(|(_, _)| {
            Monad::ret((Exp::Noop, ctx))
        }));

    state.sequence(ctx);

    state.children(ctx).iter().cloned().for_each(|ctx| print_state(&state, ctx));

    state.timeline().clone().iter().cloned().enumerate().for_each(|(idx, (t, ctx))| {
        let lengths = state.lengths(ctx).iter().cloned().map(|length| format!("{}", length.as_u64())).collect::<Vec<String>>().join(", ");
        eprintln!("{IntenseYellow}{idx}: T:{t} ID:{} DUR: {lengths}{ResetColor}", ctx.to_usize());
    });

    state
}

fn compose_exp(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    eprintln!("{IntensePurple}{exp}{ResetColor}");

    let m = match exp {
        Exp::Compound(compound) => compose_compound(compound, ctx, state),
        Exp::Simple(simple) => compose_simple(simple, ctx, state),
        Exp::Noop => compose_exp(state.next().unwrap_or_default(), ctx, state),
        Exp::EOI => Monad::ret((Exp::EOI, ctx)),
    };
    // graph(state, ctx);
    // print_state(state, ctx);

    m
}

fn compose_compound(compound: Compound, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    dbg!(ctx);
    let ctx_ = state.append_child(ctx);
    state.set_scope(ctx_, compound.scope());

    let program = std::mem::take(&mut state.programs(ctx).last().cloned().unwrap_or_default());
    let tempo = std::mem::take(&mut state.tempos(ctx).last().cloned().unwrap_or_default());
    let length = state.lengths(ctx).last().cloned().unwrap_or_default();
    let register = state.registers(ctx).last().cloned().unwrap_or_default();
    let velocity = state.velocities(ctx).last().cloned().unwrap_or_default();
    let bindings = state.bindings(ctx);

    state.add_program(ctx_, program);
    state.add_tempo(ctx_, tempo);
    state.add_length(ctx_, length);
    state.add_register(ctx_, register);
    state.add_velocity(ctx_, velocity);
    state.add_bindings(ctx_, bindings);

    let exps = compound.to_vec();

    let (exps_, index_) = state.set_exps(exps, 0);

    let mut m = Monad::ret((state.next().unwrap_or_default(), ctx_));

    while state.len() > 0 {
        m = m.bind(Box::new(|(exp, ctx)| compose_exp(exp, ctx_, state)));
    }

    m = m
        .bind(Box::new(|(exp, ctx)| compose_exp(exp, ctx_, state)))
        .bind(Box::new(|(_, ctx)| {
            // dbg!();
            state.set_exps(exps_, index_);
            // print_state(state, ctx);
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }));
    m
}

fn compose_simple(simple: Simple, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match simple {
        Simple::Prefix(prefix) => compose_prefix(prefix, ctx, state),
        Simple::Infix(infix) => compose_infix(infix, ctx, state),
        Simple::Suffix(suffix) => compose_suffix(suffix, ctx, state),
        Simple::Decl(decl) => compose_decl(decl, ctx, state),
        Simple::Scalar(scalar) => compose_scalar(scalar, ctx, state),
        Simple::Ident(ident) => compose_ident(ident, ctx, state),
    }
}

fn compose_decl(decl: Decl, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let Decl { ident, binding } = decl;
    state.add_binding(ctx, ident, *binding);
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_prefix(prefix: Prefix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match prefix {
        Prefix::Dur => {
            let rhs = state.next().unwrap_or_default();
            match rhs {
                Exp::Compound(compound) => todo!(),
                Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs)))) => {
                    compose_duration(Duration::Fractional(Fractional::Absolute(abs)), ctx, state)
                }
                Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
                    Fractional::Tuplet(tuplet),
                )))) => compose_fractional_duration(Fractional::Tuplet(tuplet), ctx, state),
                Exp::Noop => todo!(),
                Exp::EOI => todo!(),
                _ => {
                    // dbg!(&rhs);
                    todo!()
                }
            }
        }
        Prefix::Pc => compose_pc(state.next().unwrap_or_default(), ctx, state),
        Prefix::Reg => compose_register(state.next().unwrap_or_default(), ctx, state),
    }
}

fn compose_prog(prog: Prog, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    state.add_program(ctx, prog);
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_infix(infix: Infix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match infix {
        Infix::Colon => todo!(),
        Infix::Intercalate => todo!(),
        Infix::Range => todo!(),
        Infix::Interpolation(interpolation) => todo!(),
        Infix::Plus => todo!(),
        Infix::Minus => todo!(),
        Infix::Mul => todo!(),
        Infix::Div => todo!(),
    }
}

fn compose_suffix(suffix: Suffix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match suffix {
        Suffix::Amp => todo!(),
        Suffix::Bpm => todo!(),
        Suffix::Freq => todo!(),
    }
}

fn compose_scalar(scalar: Scalar, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match scalar {
        Scalar::Duration(duration) => compose_duration(duration, ctx, state),
        Scalar::Frequency(absolute) => compose_absolute(absolute, ctx, state),
        Scalar::Pure(pure) => compose_pure(pure, ctx, state),
        Scalar::Dynamic(dynamic) => compose_dynamic(dynamic, ctx, state),
        Scalar::Rest => {
            state.add_pc(ctx, Pc::None);
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
        Scalar::Tuplet(tuplet) => {
            compose_fractional_duration(Fractional::Tuplet(tuplet), ctx, state)
        }
        Scalar::Prog(prog) => compose_prog(prog, ctx, state),
    }
}

fn compose_absolute(absolute: Absolute, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match absolute {
        Absolute::UInt(uint) => {
            state.add_pc(ctx, Pc::Class(uint as u8));
            state.pad(ctx, state.pcs(ctx).len());
            Monad::ret((Exp::Noop, ctx))
        }
        Absolute::Float(_) => todo!(),
    }
}

fn compose_pure(pure: Pure, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let next = state.next().unwrap_or_default();
    // dbg!(&next);
    match next {
        Exp::Simple(Simple::Suffix(suffix)) => match suffix {
            Suffix::Amp => todo!(),
            Suffix::Bpm => match pure {
                Pure::Relative(relative) => todo!(),
                Pure::Absolute(absolute) => {
                    let microseconds_per_quarter_note =
                        f64::round(60_000_000 as f64 / absolute.as_f64()) as u64;
                    state.add_tempo(ctx, Mpb(microseconds_per_quarter_note));
                    Monad::ret((state.next().unwrap_or_default(), ctx))
                }
            },
            Suffix::Freq => todo!(),
        },
        Exp::Simple(Simple::Infix(infix)) => compose_infix(infix, ctx, state),
        next => match pure {
            Pure::Relative(relative) => compose_relative(relative, ctx, state)
                .bind(Box::new(|(_, ctx)| Monad::ret((next, ctx)))),
            Pure::Absolute(absolute) => compose_absolute(absolute, ctx, state)
                .bind(Box::new(|(_, ctx)| Monad::ret((next, ctx)))),
        },
    }
}

fn compose_relative(relative: Relative, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_dynamic(dynamic: String, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let velocity: u8 = match dynamic.as_str() {
        "ppp" => 20,
        "pp" => 39,
        "p" => 57,
        "mf" => 84,
        "f" => 98,
        "ff" => 113,
        "fff" => 127,
        _ => 80,
    };
    state.add_velocity(ctx, Velocity(velocity));
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_ident(ident: Ident, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    compose_exp(state.binding(ctx, ident), ctx, state)
}

fn compose_duration(duration: Duration, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match duration {
        Duration::Fixed(fixed) => composed_fixed_duration(fixed, ctx, state),
        Duration::Fractional(fractional) => compose_fractional_duration(fractional, ctx, state),
    }
}

fn composed_fixed_duration(fixed: Fixed, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_fractional_duration(
    fractional: Fractional,
    ctx: Ctx,
    state: &mut State,
) -> Monad<(Exp, Ctx)> {
    match fractional {
        Fractional::Absolute(absolute) => {
            let microseconds = f64::round(
                QUARTER_NOTE_RATIO / absolute.as_f64()
                    * state.tempos(ctx).last().cloned().unwrap_or_default().0 as f64,
            ) as u64;
            state.add_length(ctx, Length::MicroSeconds(microseconds));

            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
        Fractional::Tuplet(tuplet) => {
            let Tuplet { lhs, rhs } = tuplet;
            let microseconds = f64::round(
                QUARTER_NOTE_RATIO / rhs.as_f64() / lhs.as_f64()
                    * state.tempos(ctx).last().cloned().unwrap_or_default().0 as f64,
            ) as u64;
            state.add_length(ctx, Length::MicroSeconds(microseconds));
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
        Fractional::Rational(rational) => {
            let Rational { num, den } = rational;
            let microseconds = f64::ceil(
                num.as_f64() / den.as_f64()
                    * QUARTER_NOTE_RATIO
                    * state.tempos(ctx).last().cloned().unwrap_or_default().0 as f64,
            ) as u64;
            // dbg!(microseconds);
            state.add_length(ctx, Length::MicroSeconds(microseconds));
            // print_state(state, ctx);
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
    }
}

fn compose_pc(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match exp {
        Exp::Compound(compound) => todo!(),
        Exp::Simple(simple) => todo!(),
        Exp::Noop => todo!(),
        Exp::EOI => todo!(),
    }
}

fn compose_register(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // dbg!(&exp);
    match exp {
        Exp::Compound(compound) => todo!(),
        Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(Absolute::UInt(int))))) => {
            state.add_register(ctx, Register::Reg(int as i8));
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
        Exp::Noop => todo!(),
        Exp::EOI => todo!(),
        _ => {
            // dbg!(&exp);
            todo!()
        }
    }
}

fn compose_rest(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    todo!()
}
