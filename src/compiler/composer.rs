// #![allow(unused, const_item_mutation)]
#![forbid(
    clippy::infinite_loop,
    clippy::maybe_infinite_iter,
    unconditional_recursion
)]
use std::{
    cell::{LazyCell, OnceCell, RefCell},
    clone,
    fmt::Pointer,
    io::stderr,
    iter::{Chain, Enumerate, Iterator, Map, from_fn, repeat_n, zip},
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    os::unix::process::ExitStatusExt,
    path::absolute,
    process::exit,
    rc::Rc,
    slice::IterMut,
    vec::{self, IntoIter},
};

use crossterm::{cursor::*, execute, style::*, terminal::*};
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Ratio};
use num_traits::{
    ConstZero, FromPrimitive, ToPrimitive,
    float::{Float, FloatCore},
};

use crate::{
    c, color,
    compiler::{
        ast::*,
        codegen::{
            Register::Reg,
            Scope,
            state::State,
            utils::{ColorIter as Color, TextStyle::*, *},
            *,
        },
        error,
        functional::*,
    },
};

pub fn compose_program(mut program: Program) -> State {
    let mut state = State::default();
    let ctx = Ctx::None;

    eprintln!("{}COMPOSING...", color!());
    compose(
        ctx,
        vec![Exp::Compound(Compound::Parens(program.exps))],
        &mut state,
    );

    // let children = state.children(Ctx::Root);

    eprintln!("{}SEQUENCING...", color!());
    sequence(&mut state, vec![Ctx::Root]);
    // exit(0);
    // print_states(&state, ctx);
    // exit(0);
    // pause();

    // print_state(&state, Ctx::Root);
    // pause();

    // print_timeline(&state);
    // pause();

    // exit(0);

    // eprintln!(
    //     "{}",
    //     format!("{:?}", state.timeline())
    //         .split_inclusive("): ")
    //         .collect::<Vec<_>>()
    //         .join("\n  ")
    //         .split_inclusive("Slice { ")
    //         .collect::<Vec<_>>()
    //         .join("\n    ")
    //         .split_inclusive("}, ")
    //         .collect::<Vec<_>>()
    //         .join("\n  ")
    //         .split_inclusive("}], ")
    //         .collect::<Vec<_>>()
    //         .join("\n")
    //         .split_inclusive("], ")
    //         .collect::<Vec<_>>()
    //         .join("\n    ")
    // );
    // exit(0);
    // pause();

    state
}

fn compose(ctx: Ctx, mut exps: Vec<Exp>, state: &mut State) -> Monad<(Exp, Ctx)> {
    // let exps = if exps.len() > 1 {
    //     exps.into_iter().fold(Vec::<Exp>::new(), |mut exps, rhs| {
    //         if let Some(lhs) = exps.pop() {
    //             c!{{
    //                 eprintln!("{lhs}\t{}{rhs}", color!());
    //                 pause();
    //             }}
    //             match (lhs, rhs) {
    //                 // (Exp::Simple(Simple::Prefix(prefix)), rhs @ Exp::Simple(Simple::Scalar(_))) => {
    //                 //     state.push_back(rhs);
    //                 //     let mut exp = Exp::Noop;
    //                 //     compose_prefix(prefix, ctx, state).bind(|(exp_, _)| {
    //                 //         exp = exp_.clone();

    //                 //         Monad::ret((exp_, ctx))
    //                 //     });
    //                 //     if !matches!(exp, Exp::Noop) {
    //                 //         exps.push(exp);
    //                 //     }
    //                 // }
    //                 (lhs, rhs) => {
    //                     exps.extend(vec![lhs, rhs]);
    //                 }
    //             }
    //         } else {
    //             exps.push(rhs);
    //         }
    //         exps
    //     })
    // } else {
    //     exps
    // };

    // c!{{
    //     eprintln!("{exps:#?}");
    //     pause();
    // }}

    let (exps_, index_) = state.set_exps(exps, 0);
    let mut stack = state.set_stack(Vec::new());
    let mut m = Monad::ret((state.next().unwrap_or_default(), ctx));

    loop {
        let mut should_break = false;
        m = m.bind(Box::new(|(exp, ctx_): (Exp, Ctx)| {
            // c! {{
            //     eprintln!("{ctx_:?} {exp:?} ");
            //     pause();
            // }}

            match exp {
                Exp::Noop => Monad::ret((
                    state
                        .next()
                        .and_then(|exp| {
                            if matches!(exp, Exp::Noop) {
                                Some(Exp::EOI)
                            } else {
                                Some(exp)
                            }
                        })
                        .unwrap_or(Exp::EOI),
                    ctx,
                )),
                Exp::EOI => {
                    should_break = true;
                    Monad::ret((Exp::Noop, ctx_))
                }
                // Exp::Simple(simple) => compose_simple(simple, ctx_, state),
                // Exp::Compound(compound) => compose_compound(compound, ctx_, state),
                exp => compose_exp(exp, ctx_, state),
            }
        }));

        // c! {{
        //     print_state(state, ctx);
        //     pause();
        // }}
        // log(format!("should_break: {should_break}"));
        if should_break {
            break;
        }
    }

    // eprint!("{}", color!());
    // print_stacks(state);

    state.set_exps(exps_, index_);
    state.set_stack(stack);
    // let len = state.pcs(ctx).unwrap_or_default().len();
    // state.pad(ctx, len);

    // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));

    // dbg!(&m);
    //

    // print_stacks(state);
    // print_state(state, ctx);
    // pause();

    Monad::ret((state.next().unwrap_or_default(), ctx))
    // Monad::ret((state.next().unwrap_or_default(), state.parent(ctx)))
}

fn compose_exp(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!(
    //     "{}\nCOMPOSE EXP {exp} {ctx:?} {:?}",
    //     color!(),
    //     state.scope(ctx)
    // );

    let next = state.next().unwrap_or_default();

    match next {
        next @ Exp::Simple(Simple::Infix(_)) => {
            if matches!(exp, Exp::Simple(Simple::Ident(_))) {
                state.push_back(next);
            } else {
                // dbg!(&next, &exp);
                // pause();
                state.store(exp);
                return Monad::ret((next, ctx));
            }
        }
        Exp::Simple(Simple::Suffix(suffix)) => {
            state.store(exp);
            return compose_suffix(suffix, ctx, state);
        }
        _ => state.push_back(next),
    }

    match exp.clone() {
        Exp::Noop => Monad::ret((
            state
                .next()
                .and_then(|exp| {
                    if matches!(exp, Exp::Noop) {
                        Some(Exp::EOI)
                    } else {
                        Some(exp)
                    }
                })
                .unwrap_or_default(),
            ctx,
        )),
        Exp::EOI => Monad::ret((Exp::EOI, ctx)),
        Exp::Simple(Simple::Ident(ident)) => {
            let binding = state.binding(ctx, &ident).unwrap_or_default();

            Monad::ret((binding, ctx))
        }
        Exp::Simple(simple) => compose_simple(simple, ctx, state),
        Exp::Compound(compound) => compose_compound(compound, ctx, state),
        Exp::Decl(decl) => compose_decl(decl, ctx, state),
    }
}

fn compose_compound(compound: Compound, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let ctx_ = state.append_child(ctx);
    state.set_scope(ctx_, compound.scope());

    // c! {
    //     {
    //         dbg!();
    //         print_state(state, ctx);
    //         pause();
    //     }
    // }

    let mut exps = compound.to_vecdeque();

    precompose(ctx, ctx_, state);
    // c! {{
    //         dbg!();
    //         print_state(state, ctx);
    //         print_state(state, ctx_);
    // }}

    let m = compose(ctx_, exps.into(), state);
    // c! {{dbg!();
    // print_state(state, ctx);
    // print_state(state, ctx_);}}
    // pause();

    postcompose(ctx, state, ctx_);

    // c! {{dbg!();
    // print_state(state, ctx);
    // print_state(state, ctx_);
    // }}
    // pause();

    m.bind(Box::new(|(exp, _)| Monad::ret((exp, ctx))))
}

fn postcompose(ctx: Ctx, state: &mut State, ctx_: Ctx) {
    c! {{
        print_state(state, ctx_);
    }}

    state.call_thunks(ctx, ctx_, LifeCycleEvent::PostComposition);

    // c! {{
    //     print_state(state, ctx_);
    //     pause();
    // }}

    // eprintln!(
    //     "{}{ctx:?}, {:?}, {ctx_:?}, {:?}",
    //     color!(),
    //     state.scope(ctx),
    //     state.scope(ctx_)
    // );

    // c!{{
    //     eprintln!("{ctx:?} {:?}, {ctx_:?} {:?}", state.scope(ctx), state.scope(ctx_));
    //     print_state(state, ctx_);
    //     pause();
    // }}

    // if matches!(ctx_, Ctx::Id(_) ) {
    let children = state.children(ctx_);
    // if children.is_empty() {

    match (state.scope(ctx), state.scope(ctx_)) {
        (Scope::Stack, Scope::Stack) => {}
        (Scope::None, Scope::Sequence) => {
            // state.call_thunks(ctx_, ctx_, LifeCycleEvent::Sequence);
            // state.call_thunks(ctx, ctx_, LifeCycleEvent::PostSequence);
        }
        (Scope::Sequence, Scope::Sequence) => {
            state.call_thunks(ctx_, ctx_, LifeCycleEvent::Sequence);
            state.call_thunks(ctx, ctx_, LifeCycleEvent::PostSequence);
            state.hoist(ctx_, ctx);
            state.drop(ctx_);
        }
        (Scope::Sequence, Scope::Stack) => {
            state.call_thunks(ctx, ctx_, LifeCycleEvent::PreSequence);
            state.call_thunks(ctx, ctx_, LifeCycleEvent::PostSequence);
            if children.is_empty() {
                state.hoist(ctx_, ctx);
                state.drop(ctx_);
            }
        }
        (Scope::Stack, Scope::Sequence) => {}

        _ => (),
    }
    // }
    // }
}

fn precompose(ctx: Ctx, ctx_: Ctx, state: &mut State) {
    // c! {{
    //     print_state(state, ctx);
    //     print_state(state, ctx_);
    //     // pause();
    // }}
    state.call_thunks(ctx, ctx_, LifeCycleEvent::PreComposition);
    // c! {{
    //     print_state(state, ctx);
    //     print_state(state, ctx_);
    //     pause();
    // }}
}

fn compose_simple(simple: Simple, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // status(format!("{}COMPOSE SIMPLE {simple:?} {ctx:?}", color!()));
    match simple {
        Simple::Prefix(prefix) => compose_prefix(prefix, ctx, state),
        Simple::Infix(infix) => compose_infix(infix, ctx, state),
        Simple::Suffix(suffix) => {
            error(format!("unexpected expression: {suffix:?}"));
            exit(1);
        }
        Simple::Decl(decl) => compose_decl(decl, ctx, state),
        Simple::Scalar(scalar) => compose_scalar(scalar, ctx, state),
        Simple::Ident(ident) => compose_ident(ident, ctx, state),
        Simple::Controller(controller_type) => compose_controller(controller_type, ctx, state),
    }
}

fn compose_controller(
    controller_type: ControllerType,
    ctx: Ctx,
    state: &mut State,
) -> Monad<(Exp, Ctx)> {
    match controller_type {
        ControllerType::Pedal => {
            state.toggle_pedal(0);
        }
    }

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_decl(decl: Decl, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match decl {
        Decl::ImportDecl(import_decl) => {
            error("import declarations not yet implemented".to_string());
            exit(1);
        }
        Decl::ExpDecl(exp_decl) => compose_expdecl(exp_decl, ctx, state),
        Decl::FuncDecl(func_decl) => {
            error("function declarations not yet implemented".to_string());
            exit(1);
        }
    }
}

fn compose_expdecl(expdecl: ExpDecl, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE EXPDECL {expdecl:?} {ctx:?}", color!());
    let ExpDecl { ident, binding } = expdecl;
    state.add_binding(ctx, ident, *binding);
    // dbg!(ctx, state.bindings(ctx));
    // pause();
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_prefix(prefix: Prefix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // info(format!("{}COMPOSE PREFIX {prefix:?} {ctx:?}", color!()));
    let rhs = state.next().unwrap_or_default();
    // c!{{
    //     eprintln!("{ctx:?} {prefix:?} {rhs}");
    //     pause();
    // }}
    // trace(IntenseBoldGreen);
    // dbg!(&rhs);
    // pause();
    match (prefix, rhs.clone()) {
        (Prefix::Dur, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) => {
            let duration = Duration::Fractional(Fractional::Absolute(abs));

            Monad::ret((Exp::Simple(Simple::Scalar(Scalar::Duration(duration))), ctx))
        }
        (Prefix::Dur, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Rational(rational))))) => {
            Monad::ret((
                Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
                    Fractional::Rational(rational),
                )))),
                ctx,
            ))
        }

        (Prefix::Dur, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Relative(rel))))) => {
            compose_relative::<Length>(rel, ctx, state)
        }
        (
            Prefix::Dur,
            Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
                Fractional::Tuplet(tuplet),
            )))),
        ) => compose_fractional_duration(Fractional::Tuplet(tuplet), ctx, state),
        (Prefix::Reg, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) => {
            Monad::ret((
                Exp::Simple(Simple::Scalar(Scalar::Register(Register::Reg(
                    abs.as_u64() as i8,
                )))),
                ctx,
            ))
        }
        (Prefix::Reg, Exp::Compound(compound)) => {
            let exps: Vec<Exp> = compound
                .to_vecdeque()
                .into_iter()
                .flat_map(|exp| {
                    if matches!(exp, Exp::Simple(Simple::Scalar(Scalar::Pure(_)))) {
                        vec![Exp::Simple(Simple::Prefix(Prefix::Reg)), exp]
                    } else {
                        vec![exp]
                    }
                })
                .collect();

            let compound = match compound.scope() {
                Scope::Sequence => Compound::Parens(exps),
                Scope::Stack => Compound::Brackets(exps),
                Scope::Set => {
                    error("unexpected scope: Set".into());
                    exit(1);
                }
                Scope::None => {
                    error("unexpected scope: None".into());
                    exit(1);
                }
            };

            Monad::ret((compound.to_exp(), ctx))
        }
        (Prefix::Reg, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Relative(rel))))) => {
            compose_relative::<Register>(rel, ctx, state)
        }
        (Prefix::Prog, Exp::Simple(simple)) => {
            error(format!("unexpected expression: {simple:?}"));
            exit(1);
        }
        (_, Exp::Noop) => {
            error(format!("unexpected expression: {}", Exp::Noop));
            exit(1);
        }
        (_, Exp::EOI) => {
            error(format!("unexpected end of input"));
            exit(1);
        }
        _ => {
            error(format!("Unexpected expression: {rhs}"));
            exit(1);
        }
    }
}

fn compose_prog(prog: Prog, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE PROG {prog:?} {ctx:?}", color!());

    let thunk = Thunk::new(Box::new(move |ctx, state| {
        state.add(ctx, vec![prog]);
    }));
    state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());

    state.add_thunk(ctx, LifeCycleEvent::Composition, thunk);

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_infix(infix: Infix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE INFIX {ctx:?} {infix:?}", color!());
    match infix {
        Infix::Colon => {
            error(format!("unexpected token: ':'"));
            exit(1);
        }
        Infix::Intercalate => {
            error(format!("not yet implemented: ><"));
            exit(1);
        }
        Infix::Range => compose_interpolation(Infix::Range, ctx, state),
        Infix::Lerp => compose_interpolation(Infix::Lerp, ctx, state),
        Infix::Plus => {
            error(format!("not yet implemented: +"));
            exit(1);
        }
        Infix::Minus => {
            error(format!("not yet implemented: -"));
            exit(1);
        }
        Infix::Mul => {
            let lhs = state.load().unwrap_or_default();
            let rhs = state.next().unwrap_or_default();
            // dbg!(&lhs, &rhs);
            match (lhs, rhs) {
                (Exp::Compound(lhs), Exp::Compound(rhs)) => todo!(),
                (Exp::Compound(compound), Exp::Simple(Simple::Scalar(scalar))) => {
                    match (compound, scalar) {
                        (Compound::Parens(exps), Scalar::Duration(duration)) => todo!(),
                        (Compound::Parens(exps), Scalar::Pure(Pure::Absolute(absolute))) => {
                            // dbg!(&exps);
                            let len = exps.len();
                            Monad::ret((
                                Exp::Compound(Compound::Parens(
                                    exps.iter()
                                        .cloned()
                                        .cycle()
                                        .take(len * absolute.as_usize())
                                        .collect(),
                                )),
                                ctx,
                            ))
                        }
                        (Compound::Parens(exps), Scalar::Dynamic(dynamic)) => todo!(),
                        (Compound::Parens(exps), Scalar::Tuplet(tuplet)) => todo!(),
                        (Compound::Parens(exps), Scalar::Prog(prog)) => todo!(),
                        (Compound::Parens(exps), Scalar::Rest) => todo!(),
                        (Compound::Braces(exps), Scalar::Duration(duration)) => todo!(),
                        (
                            compound @ Compound::Braces(_),
                            Scalar::Pure(Pure::Absolute(absolute)),
                        ) => {
                            let exps: Vec<Exp> =
                                repeat_n(Exp::Compound(compound), absolute.as_usize()).collect();
                            compose(ctx, exps, state)
                        }
                        (Compound::Braces(exps), Scalar::Dynamic(dynamic)) => todo!(),
                        (Compound::Braces(exps), Scalar::Tuplet(tuplet)) => todo!(),
                        (Compound::Braces(exps), Scalar::Prog(prog)) => todo!(),
                        (Compound::Braces(exps), Scalar::Rest) => todo!(),
                        _ => todo!(),
                    }
                }
                (Exp::Simple(Simple::Scalar(scalar)), Exp::Compound(compound)) => {
                    match (scalar, compound) {
                        (Scalar::Duration(duration), Compound::Parens(exps)) => todo!(),
                        (Scalar::Duration(duration), Compound::Braces(exps)) => todo!(),
                        (Scalar::Duration(duration), Compound::Brackets(exps)) => todo!(),
                        (Scalar::Pure(pure), Compound::Parens(exps)) => todo!(),
                        (Scalar::Pure(pure), Compound::Braces(exps)) => todo!(),
                        (Scalar::Pure(pure), Compound::Brackets(exps)) => todo!(),
                        (Scalar::Dynamic(dynamic), Compound::Parens(exps)) => todo!(),
                        (Scalar::Dynamic(dynamic), Compound::Braces(exps)) => todo!(),
                        (Scalar::Dynamic(dynamic), Compound::Brackets(exps)) => todo!(),
                        (Scalar::Tuplet(tuplet), Compound::Parens(exps)) => todo!(),
                        (Scalar::Tuplet(tuplet), Compound::Braces(exps)) => todo!(),
                        (Scalar::Tuplet(tuplet), Compound::Brackets(exps)) => todo!(),
                        (Scalar::Prog(prog), Compound::Parens(exps)) => todo!(),
                        (Scalar::Prog(prog), Compound::Braces(exps)) => todo!(),
                        (Scalar::Prog(prog), Compound::Brackets(exps)) => todo!(),
                        (Scalar::Rest, Compound::Parens(exps)) => todo!(),
                        (Scalar::Rest, Compound::Braces(exps)) => todo!(),
                        (Scalar::Rest, Compound::Brackets(exps)) => todo!(),
                        _ => todo!(),
                    }
                }
                (Exp::Simple(Simple::Scalar(lhs)), Exp::Simple(Simple::Scalar(rhs))) => {
                    match (lhs, rhs) {
                        (Scalar::Rest, Scalar::Pure(Pure::Absolute(abs))) => {
                            for _ in 0..abs.as_usize() {
                                state.add(ctx, vec![Pc::None]);
                                state.pad(ctx);
                            }
                            Monad::ret((state.next().unwrap_or_default(), ctx))
                        }
                        (Scalar::Rest, Scalar::Dynamic(dynamic)) => todo!(),
                        (Scalar::Rest, Scalar::Tuplet(tuplet)) => todo!(),
                        (Scalar::Rest, Scalar::Prog(prog)) => todo!(),
                        (Scalar::Rest, Scalar::Register(register)) => todo!(),
                        (Scalar::Rest, Scalar::Rest) => todo!(),
                        _ => todo!(),
                    }
                }
                _ => {
                    todo!()
                }
            }
        }
        Infix::Div => todo!(),
    }
}

fn compose_interpolation(infix: Infix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let mut scalars = extract_scalars(state);
    let first = scalars.first().cloned().unwrap();

    let thunk = Thunk::new(Box::new(move |ctx, state| {
        let max: f64 = scalars.iter().cloned().fold(0.0, |max, scalar| {
            max.max(scalar_to_f64(ctx, state, &scalar))
        });
        let min: f64 = scalars.iter().cloned().fold(f64::MAX, |min, scalar| {
            min.min(scalar_to_f64(ctx, state, &scalar))
        });

        let interval = 1.0 / (scalars.len() as f64 - 1.0);

        // c! {{
        //     eprintln!("interval: {interval}");
        // }}
        //

        // TODO: change Length to BigRational
        let points: Vec<(f64, f64)> = scalars
            .iter_mut()
            .enumerate()
            .map(|(idx, scalar)| {
                let x = idx as f64 * interval;
                let y = scalar_to_f64(ctx, state, &scalar).min(max).max(min)
                    / match first {
                        Scalar::Duration(_) => 1.0,
                        Scalar::Frequency(_) => todo!(),
                        Scalar::Pure(_) => 127.0,
                        Scalar::Dynamic(_) => 127.0,
                        Scalar::Tuplet(_) => 127.0,
                        Scalar::Prog(_) => 127.0,
                        Scalar::Register(_) => 8.0,
                        Scalar::Rest => 127.0,
                        Scalar::Tempo(_) => 1.0,
                    };
                (x, y)
            })
            .collect::<Vec<_>>();

        // c! {{
        //     eprintln!("{ctx:?}\n{}", points.iter().map(|(x, y)| format!("x: {} y: {}", x.as_ratio().to_f64().unwrap_or_default(), y.as_ratio().to_f64().unwrap_or_default())).collect::<Vec<_>>().join("\n"));
        //     pause();
        // }}

        // let children = state.children(ctx);
        // let mut ctxs = if children.is_empty() {
        //     vec![ctx]
        // } else {
        //     children
        // };
        // let first_ctx = ctxs.first().cloned().unwrap_or_default();

        // let mut ctx_len: Length = state.get_ctx_length(first_ctx).max(Length::MicroSeconds(
        //     Ratio::from_usize(state.pcs(first_ctx).unwrap_or_default().len()).unwrap()
        //         * state.tempo().0.clone(),
        // ));

        // if ctx_len.as_ratio() == BigRational::ZERO {
        //     ctx_len = Length::MicroSeconds(
        //         state
        //             .tempos(first_ctx)
        //             .unwrap_or_default()
        //             .iter()
        //             .cloned()
        //             .map(|tmps| {
        //                 tmps.iter()
        //                     .cloned()
        //                     .min_by(|a, b| a.0.cmp(&b.0))
        //                     .unwrap_or_default()
        //                     .0
        //             })
        //             .sum(),
        //     );
        // }

        // c! {{
        //     eprintln!("{ctx:?} {ctx_len:?}");
        //     pause();
        // }}

        // c! {{
        //     eprint!("{ctx:?} ctx_len: {}", ctx_len.as_u64());
        //     pause();
        // }}

        // let mut length_ = Length::default_max();

        // let mut ctx_iter = ctxs.iter().cloned();
        // let init = state
        //     .pcs(ctx_iter.next().unwrap_or_default())
        //     .unwrap_or_default()
        //     .len() as f64;

        // let max_pc_len = ctx_iter.fold(init, |a, b| {
        //     a.max(state.pcs(b).unwrap_or_default().len() as f64)
        // });

        // let mut step = 1. / max_pc_len as f64;

        let pc_len = state.pcs(ctx).unwrap_or_default().len();

        let step = 1.0 / pc_len as f64;

        for idx in 0..pc_len {
            let y = match infix {
                Infix::Colon => todo!(),
                Infix::Intercalate => todo!(),
                Infix::Range => L(idx as f64 * step, &points),
                Infix::Lerp => lerp(idx as f64 * step, &points),
                Infix::Plus => todo!(),
                Infix::Minus => todo!(),
                Infix::Mul => todo!(),
                Infix::Div => todo!(),
            };

            c! {{
                eprintln!("y: {y}");
                // pause();
            }}

            let len = state
                .pcs(ctx)
                .unwrap_or_default()
                .get(idx)
                .cloned()
                .unwrap_or_default()
                .len();

            match first {
                Scalar::Duration(_) => {
                    let length = Length::from(
                        y / 4.0
                            * state
                                .tempos(ctx)
                                .unwrap_or_default()
                                .get(idx)
                                .cloned()
                                .unwrap_or_default()
                                .first()
                                .cloned()
                                .unwrap_or_default(),
                    );

                    c! {{
                        eprintln!("Length: {}", length.as_u64())
                    }}

                    state.lengths_mut(ctx).get_mut(idx).and_then(|lengths| {
                        *lengths = repeat_n(length, len).collect();
                        Some(())
                    });
                }
                Scalar::Frequency(_) => todo!(),
                Scalar::Pure(_) => {
                    state.pcs_mut(ctx).get_mut(idx).and_then(|pcs| {
                        *pcs = repeat_n(Pc::Class(y * 12.0), len).collect();
                        Some(())
                    });
                }
                Scalar::Dynamic(_) => {
                    state
                        .velocities_mut(ctx)
                        .get_mut(idx)
                        .and_then(|velocities| {
                            *velocities = repeat_n(Velocity::from(Dynamic::from(y)), len).collect();
                            Some(())
                        });
                }
                Scalar::Tempo(_) => {
                    state.tempos_mut(ctx).get_mut(idx).and_then(|tempos| {
                        *tempos = repeat_n(Mpb(BigRational::from_f64(y).unwrap_or_default()), len).collect();
                        Some(())
                    });

                }
                Scalar::Tuplet(_) => todo!(),
                Scalar::Prog(_) => todo!(),
                Scalar::Register(_) => todo!(),
                Scalar::Rest => todo!(),
            }
        }

        // c! {{
        //     eprintln!("step: {step}");
        // }}
        // // print_state(state, ctx);
        // // dbg!(step);
        // pause();

        // let mut lengths = Vec::<Vec<(usize, Vec<Length>)>>::new();
        // let mut counters = Vec::<Length>::new();

        // ctxs.iter().cloned().for_each(|ctx| {
        //     let pcs = state.pcs(ctx).unwrap_or_default();
        //     let lengths_ = state.lengths(ctx).unwrap_or_default();
        //     let len = pcs.len().max(lengths_.len());
        //     let lens: Vec<Length> = (0..len)
        //         .map(|idx| Length::from(idx as f64 / len as f64))
        //         .collect();

        //     lengths.push(
        //         // repeat_n(Length::MicroSeconds(state.tempo().0.clone()), len).enumerate().collect::<Vec<_>>());
        //         state
        //             .lengths(ctx)
        //             .unwrap_or_default()
        //             .clone()
        //             .into_iter()
        //             .enumerate()
        //             .rev()
        //             .collect::<Vec<_>>(),
        //     );
        //     counters.push(Length::MicroSeconds(BigRational::ZERO));
        // });

        // if lengths.first().cloned().iter().cloned().all(|lengths| {
        //     lengths
        //         .iter()
        //         .cloned()
        //         .all(|(_, lens)| lens.iter().all(|len| len.as_ratio() == BigRational::ZERO))
        // }) {
        //     let len = state.pcs(ctx).unwrap_or_default().len();
        //     lengths.first_mut().and_then(|first| {
        //         first.iter_mut().for_each(|(_, lengths)| {
        //             let len = lengths.len();
        //             *lengths = repeat_n(Length::MicroSeconds(state.tempo().0.clone()), len)
        //                 .collect::<Vec<_>>();
        //         });
        //         Some(())
        //     });
        // }

        // let mut iters = zip(ctxs.clone(), zip(lengths, counters)).collect::<Vec<_>>();

        // while iters.iter().any(|(_, (lengths, _))| {
        //     // c! {{
        //     //     eprintln!("{}", lengths.len());

        //     // }}
        //     lengths.len() > 0
        // }) {
        //     for (ctx, (lengths, counter)) in iters.iter_mut() {
        //         // if matches!(ctx, Ctx::Id(2)) {c!{{
        //         //     eprintln!("{ctx:?} {} {counter:?}", lengths.iter().map(|(idx, lengths)| {
        //         //         format!("{idx}: [{}]", lengths.iter().map(|length| format!("{}", length.as_u64())).collect::<Vec<_>>().join(" "))
        //         //     }).collect::<Vec<_>>().join(", "));
        //         //     // pause();
        //         // }}}

        //         if state.pcs(*ctx).unwrap_or_default().len() == 0 {
        //             continue;
        //         }

        //         let Some((idx, lengths)) = lengths.pop() else {
        //             continue;
        //         };

        //         // c! {{
        //         //     print_state(state, *ctx);
        //         //     eprintln!("{ctx:?} lens: {}, counter: {}",
        //         //         lengths.iter().map(|length| length.clone().max(Length::MicroSeconds(state.tempo().0.clone()))).map(|length| length.as_u64()).collect::<Vec<_>>().iter()
        //         //             .map(u64::to_string).collect::<Vec<_>>().join(" "), counter);

        //         //     pause();
        //         // }}

        //         let mut length = lengths.first().cloned().unwrap_or_default();
        //         // c! {{
        //         //     eprintln!("length: {}", length.as_ratio().to_u64().unwrap_or_default())
        //         // }}
        //         let y = match infix {
        //             Infix::Colon => todo!(),
        //             Infix::Intercalate => todo!(),
        //             Infix::Range => L(
        //                 counter.clone(),
        //                 Length::MicroSeconds(BigRational::ZERO),
        //                 &points,
        //             ),
        //             Infix::Lerp => {
        //                 c!{{print_state(state, *ctx);}}
        //                 let y = lerp(
        //                     counter.clone(),
        //                     Length::MicroSeconds(BigRational::ZERO),
        //                     &points,
        //                 );

        //                 c! {{
        //                     eprintln!("x: {} y: {:?}",counter.as_ratio().to_f64().unwrap_or_default(), y.as_ratio().to_u64().unwrap_or_default());
        //                     pause();
        //                 }}

        //                 y
        //             }
        //             Infix::Plus => todo!(),
        //             Infix::Minus => todo!(),
        //             Infix::Mul => todo!(),
        //             Infix::Div => todo!(),
        //         };

        //         // if matches!(ctx, Ctx::Id(2)) {
        //         //     c! {{
        //         //         eprintln!("{ctx:?} y: {y:?}");
        //         //         // pause();
        //         //     }}
        //         // }

        //         // c! {{
        //         //     eprintln!("y: {y}");
        //         //     // pause();
        //         // }}

        //         match first {
        //             Scalar::Duration(_) => {
        //                 // dbg!();
        //                 let micros =
        //                     Duration::Fractional(Fractional::Absolute(Absolute::Float(y.as_f64())))
        //                         .to_micros(state);

        //                 length = Length::MicroSeconds(micros.clone());

        //                 if let Some(lengths) = state.lengths_mut(ctx.clone()).get_mut(idx.clone()) {
        //                     *lengths =
        //                         repeat_n(Length::MicroSeconds(micros), lengths.len()).collect();
        //                 } else {
        //                     state.add(*ctx, vec![Length::MicroSeconds(micros)]);
        //                 }
        //             }
        //             Scalar::Frequency(_) => todo!(),
        //             Scalar::Pure(_) => {
        //                 todo!()
        //             }
        //             Scalar::Dynamic(_) => {
        //                 let val = f64::ceil(y.as_f64() * u7::max_value().as_int() as f64)
        //                     .min(max)
        //                     .max(min);
        //                 let vel = Velocity::from(val as u8);

        //                 if let Some(velocities) = state.velocities_mut(*ctx).get_mut(idx.clone()) {
        //                     *velocities = vec![Velocity::from(vel)];
        //                 } else {
        //                     state.add(*ctx, vec![Velocity::from(vel)]);
        //                 }
        //             }
        //             Scalar::Tuplet(_) => todo!(),
        //             Scalar::Prog(_) => todo!(),
        //             Scalar::Register(_) => todo!(),
        //             Scalar::Rest => todo!(),
        //             Scalar::Tempo(_) => {
        //                 if let Some(tempos) = state.tempos_mut(*ctx).get_mut(idx.clone()) {
        //                     let tmp = tempos.first().cloned().unwrap_or_default();
        //                     let len = tempos.len();
        //                     let new_tempo = Mpb(Ratio::<BigInt>::from_float(
        //                         60_000_000. / y.as_f64().min(max).max(min),
        //                     )
        //                     .unwrap());
        //                     *tempos = repeat_n(new_tempo.clone(), len).collect();

        //                     if let Some(lengths) = state.lengths_mut(*ctx).get_mut(idx.clone()) {
        //                         lengths.iter_mut().for_each(|length| {
        //                             let dur = tmp.0.to_integer().to_f64().unwrap()
        //                                 / length.as_f64()
        //                                 * QUARTER_NOTE_RATIO;
        //                             let len = Length::MicroSeconds(new_tempo.0.clone())
        //                                 / Length::MicroSeconds(
        //                                     Ratio::<BigInt>::from_float(dur / QUARTER_NOTE_RATIO)
        //                                         .unwrap_or_else(|| {
        //                                             error(format!("missing duration values"));
        //                                             exit(1)
        //                                         }),
        //                                 );

        //                             // step = gcd(step.clone(), len.clone());
        //                             length_ = len.clone();

        //                             *length = length_.clone();
        //                         });
        //                     }
        //                 }
        //             }
        //         }
        //         // *counter += length.as_f64() / step;

        //         // *counter += Length::MicroSeconds(length.as_ratio() / ctx_len.as_ratio());
        //         *counter += Length::MicroSeconds(length.as_ratio());
        //         // *counter += 1.0 / state.pcs(*ctx).unwrap_or_default().len() as f64;

        //         c! {{
        //             eprintln!("{ctx:?} counter: {}", counter.as_ratio().to_f64().unwrap());
        //         }}

        //         // c! {{print_state(state, *ctx);}}

        //         // pause();
        //         // }
        //         // c!{{
        //         //     print_state(state, *ctx);
        //         // }}
        //         // pause();
        //     }
        // }
    }));

    let life_cycle_event = match infix {
        Infix::Colon => todo!(),
        Infix::Intercalate => todo!(),
        Infix::Range => LifeCycleEvent::PreSequence,
        Infix::Lerp => LifeCycleEvent::PreSequence,
        Infix::Plus => todo!(),
        Infix::Minus => todo!(),
        Infix::Mul => todo!(),
        Infix::Div => todo!(),
    };

    state.add_thunk(ctx, life_cycle_event, thunk.clone());

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn extract_scalars(state: &mut State) -> Vec<Scalar> {
    let mut lhs = state.load().unwrap_or_default();

    if let Exp::Simple(Simple::Suffix(Suffix::Bpm)) = lhs {
        if let Some(Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) = state.next() {
            lhs = Exp::Simple(Simple::Scalar(Scalar::Tempo(abs)));
        }
    }
    let mut scalars = Vec::<Scalar>::new();
    if let Exp::Simple(Simple::Scalar(scalar)) = lhs {
        match scalar {
            Scalar::Pure(Pure::Absolute(absolute)) => {
                let next = state.next().unwrap_or_default();
                if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
                    scalars.push(Scalar::Tempo(absolute))
                } else {
                    state.push_back(next.clone());
                    scalars.push(scalar);
                }
            }
            scalar => {
                scalars.push(scalar);
            }
        }
    }
    if let Some(next) = state.next() {
        // c! {{
        //     eprintln!("{ctx:?} {next:?}");
        // }}
        match next {
            Exp::Compound(compound) => todo!(),
            Exp::Simple(simple) => match simple {
                Simple::Prefix(prefix) => match prefix {
                    Prefix::Dur => {
                        if let Some(Exp::Simple(Simple::Scalar(Scalar::Pure(pure)))) = state.next()
                        {
                            match pure {
                                Pure::Relative(relative) => todo!(),
                                Pure::Absolute(absolute) => {
                                    scalars.push(Scalar::Duration(Duration::Fractional(
                                        Fractional::Absolute(absolute),
                                    )));
                                }
                                Pure::Rational(rational) => {
                                    scalars.push(Scalar::Duration(Duration::Fractional(
                                        Fractional::Rational(rational),
                                    )));
                                }
                            }
                        }
                    }
                    Prefix::Reg => todo!(),
                    Prefix::Prog => todo!(),
                },
                Simple::Scalar(scalar) => match scalar {
                    Scalar::Pure(Pure::Absolute(absolute)) => {
                        let next = state.next().unwrap_or_default();
                        if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
                            scalars.push(Scalar::Tempo(absolute))
                        } else {
                            state.push_back(next.clone());
                            scalars.push(scalar);
                        }
                    }
                    scalar => {
                        scalars.push(scalar);
                    }
                },
                Simple::Infix(infix) => todo!(),
                Simple::Suffix(suffix) => todo!(),
                Simple::Decl(decl) => todo!(),
                Simple::Controller(controller_type) => todo!(),
                Simple::Ident(ident) => todo!(),
            },
            Exp::Decl(decl) => todo!(),
            Exp::Noop => todo!(),
            Exp::EOI => todo!(),
        }
    }

    // if matches!(ctx, Ctx::Id(2)) {c! {{
    //     eprintln!("{scalars:?}");
    //     pause();
    // }}}

    let mut next = state.next().unwrap_or_default();

    while matches!(next, Exp::Simple(Simple::Infix(_))) {
        let rhs = state.next().unwrap_or_default();
        // c! {{
        //     eprintln!("{ctx:?} rhs: {rhs:?}");
        //     pause();
        // }}
        scalars.push(match rhs.clone() {
            Exp::Simple(simple) => match simple {
                Simple::Prefix(prefix) => match prefix {
                    Prefix::Dur => {
                        let next = state.next().unwrap_or_default();
                        match next {
                            Exp::Simple(Simple::Scalar(scalar)) => match scalar {
                                Scalar::Pure(pure) => match pure {
                                    Pure::Relative(relative) => todo!(),
                                    Pure::Absolute(absolute) => Scalar::Duration(
                                        Duration::Fractional(Fractional::Absolute(absolute)),
                                    ),
                                    Pure::Rational(rational) => Scalar::Duration(
                                        Duration::Fractional(Fractional::Rational(rational)),
                                    ),
                                },
                                _ => todo!(),
                            },
                            _ => todo!(),
                        }
                    }
                    Prefix::Reg => todo!(),
                    Prefix::Prog => todo!(),
                },
                Simple::Scalar(scalar) => match scalar {
                    ref scalar @ Scalar::Pure(Pure::Absolute(ref absolute)) => {
                        let next = state.next().unwrap_or_default();
                        if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
                            Scalar::Tempo(*absolute)
                        } else {
                            scalar.clone()
                        }
                    }
                    scalar => scalar,
                },

                _ => todo!(),
            },
            _ => todo!(),
        });

        next = state.next().unwrap_or_default()
    }

    state.push_back(next.clone());
    scalars
}

fn scalar_to_f64(ctx: Ctx, state: &mut State, scalar: &Scalar) -> f64 {
    match scalar {
        Scalar::Duration(duration) => duration.as_f64(),
        Scalar::Frequency(absolute) => absolute.as_f64(),
        Scalar::Pure(pure) => match pure {
            Pure::Relative(relative) => {
                let val = state
                    .get::<Pc>(ctx)
                    .first()
                    .cloned()
                    .unwrap_or_default()
                    .as_f64();
                match relative.sign {
                    Sign::Plus => val + relative.val.as_f64(),
                    Sign::Minus => val - relative.val.as_f64(),
                }
            }
            Pure::Absolute(absolute) => absolute.as_f64(),
            Pure::Rational(rational) => rational.num.as_f64() / rational.den.as_f64(),
        },
        Scalar::Dynamic(dynamic) => dynamic.as_f64(),
        Scalar::Tuplet(tuplet) => {
            state
                .get::<Length>(ctx)
                .first()
                .cloned()
                .unwrap_or_default()
                .as_f64()
                * tuplet.rhs.as_f64()
                / tuplet.lhs.as_f64()
        }
        Scalar::Prog(prog) => todo!(),
        Scalar::Register(register) => todo!(),
        Scalar::Rest => todo!(),
        Scalar::Tempo(absolute) => absolute.as_f64(),
    }
}

fn exp_to_f64(exp: Exp, state: &mut State, ctx: Ctx) -> f64 {
    match exp {
        Exp::Compound(compound) => todo!(),
        Exp::Simple(simple) => match simple {
            Simple::Prefix(prefix) => match prefix {
                Prefix::Dur => {
                    let next = state.next().unwrap_or_default();
                    match next {
                        Exp::Compound(compound) => todo!(),
                        Exp::Simple(simple) => match simple {
                            Simple::Prefix(prefix) => todo!(),
                            Simple::Scalar(scalar) => scalar_to_f64(ctx, state, &scalar),
                            Simple::Infix(infix) => todo!(),
                            Simple::Suffix(suffix) => todo!(),
                            Simple::Decl(decl) => todo!(),
                            Simple::Ident(ident) => todo!(),
                            Simple::Controller(controller_type) => todo!(),
                        },
                        Exp::Decl(decl) => todo!(),
                        Exp::Noop => todo!(),
                        Exp::EOI => todo!(),
                    }
                }
                Prefix::Reg => todo!(),
                Prefix::Prog => todo!(),
            },
            Simple::Scalar(scalar) => scalar_to_f64(ctx, state, &scalar),
            Simple::Infix(infix) => todo!(),
            Simple::Suffix(suffix) => todo!(),
            Simple::Decl(decl) => todo!(),
            Simple::Ident(ident) => todo!(),
            Simple::Controller(controller_type) => todo!(),
        },
        Exp::Decl(decl) => todo!(),
        Exp::Noop => todo!(),
        Exp::EOI => todo!(),
        _ => todo!(),
    }
}

// fn compose_interpolation(
//     interpolation: Interpolation,
//     ctx: Ctx,
//     state: &mut State,
// ) -> Monad<(Exp, Ctx)> {
//     let mut lhs = state.load().unwrap_or_default();

//     if let Exp::Simple(Simple::Suffix(Suffix::Bpm)) = lhs {
//         if let Some(Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) = state.next() {
//             lhs = Exp::Simple(Simple::Scalar(Scalar::Tempo(abs)));
//         }
//     }
//     let mut scalars = Vec::<Scalar>::new();
//     if let Exp::Simple(Simple::Scalar(scalar)) = lhs {
//         match scalar {
//             Scalar::Pure(Pure::Absolute(absolute)) => {
//                 let next = state.next().unwrap_or_default();
//                 if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                     scalars.push(Scalar::Tempo(absolute))
//                 } else {
//                     state.push_back(next.clone());
//                     scalars.push(scalar);
//                 }
//             }
//             scalar => {
//                 scalars.push(scalar);
//             }
//         }
//     }
//     if let Some(Exp::Simple(Simple::Scalar(scalar))) = state.next() {
//         match scalar {
//             Scalar::Pure(Pure::Absolute(absolute)) => {
//                 let next = state.next().unwrap_or_default();
//                 if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                     scalars.push(Scalar::Tempo(absolute))
//                 } else {
//                     state.push_back(next.clone());
//                     scalars.push(scalar);
//                 }
//             }
//             scalar => {
//                 scalars.push(scalar);
//             }
//         }
//     }

//     let mut next = state.next().unwrap_or_default();

//     while matches!(next, Exp::Simple(Simple::Infix(Infix::Interpolation(_)))) {
//         let rhs = state.next().unwrap_or_default();
//         scalars.push(match rhs.clone() {
//             Exp::Simple(simple) => match simple {
//                 Simple::Prefix(prefix) => match prefix {
//                     Prefix::Dur => {
//                         let next = state.next().unwrap_or_default();
//                         match next {
//                             Exp::Simple(Simple::Scalar(scalar)) => match scalar {
//                                 Scalar::Pure(pure) => match pure {
//                                     Pure::Relative(relative) => todo!(),
//                                     Pure::Absolute(absolute) => Scalar::Duration(
//                                         Duration::Fractional(Fractional::Absolute(absolute)),
//                                     ),
//                                     Pure::Rational(rational) => todo!(),
//                                 },
//                                 _ => todo!(),
//                             },
//                             _ => todo!(),
//                         }
//                     }
//                     Prefix::Reg => todo!(),
//                     Prefix::Prog => todo!(),
//                 },
//                 Simple::Scalar(scalar) => match scalar {
//                     ref scalar @ Scalar::Pure(Pure::Absolute(ref absolute)) => {
//                         let next = state.next().unwrap_or_default();
//                         if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                             Scalar::Tempo(*absolute)
//                         } else {
//                             scalar.clone()
//                         }
//                     }
//                     scalar => scalar,
//                 },

//                 _ => todo!(),
//             },
//             _ => todo!(),
//         });

//         next = state.next().unwrap_or_default()
//     }

//     state.push_back(next.clone());

//     misc(format!("{}scalars: {scalars:?}", color!()));

//     let first = scalars.first().cloned().unwrap();

//     state.add_thunk(
//         ctx,
//         LifeCycleEvent::PostSequence,
//         Thunk::new(Box::new(move |ctx, state| {
//             let max: f64 = scalars.iter().cloned().fold(0.0, |max, scalar| {
//                 max.max(scalar_to_f64(ctx, state, &scalar))
//             });
//             let min: f64 = scalars.iter().cloned().fold(f64::MAX, |min, scalar| {
//                 min.min(scalar_to_f64(ctx, state, &scalar))
//             });

//             let interval = 1.0 / (scalars.len() as f64 - 1.0);

//             let points: Vec<f64> = scalars
//                 .iter_mut()
//                 .enumerate()
//                 .map(|(idx, scalar)| {
//                     let x = idx as f64 * interval;
//                     let y = scalar_to_f64(ctx, state, &scalar).min(max).max(min)
//                         / match first {
//                             Scalar::Duration(_) => 1.0,
//                             Scalar::Frequency(_) => todo!(),
//                             Scalar::Pure(_) => 127.0,
//                             Scalar::Dynamic(_) => 127.0,
//                             Scalar::Tuplet(_) => 127.0,
//                             Scalar::Prog(_) => 127.0,
//                             Scalar::Register(_) => 8.0,
//                             Scalar::Rest => 127.0,
//                             Scalar::Tempo(_) => 1.0,
//                         };
//                     y
//                 })
//                 .collect();

//             let mut ctxs = state.children(ctx);
//             ctxs.push(ctx);
//             let mut pc_len: usize = 0;
//             let ctx_len = state.get_ctx_length(ctx);
//             let mut length_ = Length::default();

//             let min_length = ctxs
//                 .iter()
//                 .cloned()
//                 .fold(Length::default_max(), |step, ctx| {
//                     step.clone()
//                         .min(state.lengths(ctx).unwrap().iter().cloned().fold(
//                             step,
//                             |step, lengths| {
//                                 step.clone().min(
//                                     lengths
//                                         .iter()
//                                         .cloned()
//                                         .fold(step.clone(), |step, length| step.min(length)),
//                                 )
//                             },
//                         ))
//                 });

//             let step = min_length.as_f64() / ctx_len.as_f64();

//             let mut lengths = Vec::<Vec<(usize, Vec<Length>)>>::new();
//             let mut counters = Vec::<f64>::new();

//             ctxs.iter().cloned().for_each(|ctx| {
//                 lengths.push(
//                     state
//                         .lengths(ctx)
//                         .unwrap_or_default()
//                         .clone()
//                         .into_iter()
//                         .enumerate()
//                         .rev()
//                         .collect::<Vec<_>>(),
//                 );
//                 counters.push(0.0);
//             });

//             let mut iters = zip(ctxs, zip(lengths, counters)).collect::<Vec<_>>();

//             let mut x: f64 = 0.0;
//             while x <= 1.0 {
//                 for (ctx, (lengths, counter)) in iters.iter_mut() {
//                     if state.pcs(*ctx).unwrap_or_default().len() == 0 {
//                         continue;
//                     }
//                     if *counter <= x {
//                         let Some((idx, lengths)) = lengths.pop() else {
//                             continue;
//                         };

//                         let length = lengths.first().cloned().unwrap_or_default();
//                         let y = bezier(points.clone(), x);
//                         match first {
//                             Scalar::Duration(_) => {
//                                 let micros =
//                                     Duration::Fractional(Fractional::Absolute(Absolute::Float(y)))
//                                         .to_micros(state);

//                                 if let Some(lengths) = state.lengths_mut(ctx.clone()) {
//                                     if let Some(lengths) = lengths.get_mut(idx.clone()) {
//                                         *lengths =
//                                             repeat_n(Length::MicroSeconds(micros), lengths.len())
//                                                 .collect();
//                                     } else {
//                                         lengths.push(vec![Length::MicroSeconds(micros)])
//                                     }
//                                 } else {
//                                     state.add_lengths(*ctx, vec![Length::MicroSeconds(micros)]);
//                                 }
//                             }
//                             Scalar::Frequency(_) => todo!(),
//                             Scalar::Pure(_) => {
//                                 todo!()
//                             }
//                             Scalar::Dynamic(_) => {
//                                 let val = f64::ceil(y * u7::max_value().as_int() as f64)
//                                     .min(max)
//                                     .max(min);
//                                 let vel = Velocity::from(val as u8);
//                                 if let Some(velocities) = state.velocities_mut(*ctx) {
//                                     if let Some(velocities) = velocities.get_mut(idx.clone()) {
//                                         *velocities = vec![Velocity::from(vel)];
//                                     } else {
//                                         velocities.push(vec![Velocity::from(vel)])
//                                     }
//                                 } else {
//                                     state.add_velocities(*ctx, vec![Velocity::from(vel)]);
//                                 }
//                             }
//                             Scalar::Tuplet(_) => todo!(),
//                             Scalar::Prog(_) => todo!(),
//                             Scalar::Register(_) => todo!(),
//                             Scalar::Rest => todo!(),
//                             Scalar::Tempo(_) => {
//                                 if let Some(tempos) = state.tempos_mut(*ctx) {
//                                     if let Some(tempos_) = tempos.get_mut(idx.clone()) {
//                                         let tmp = tempos_.first().cloned().unwrap_or_default();
//                                         let len = tempos_.len();
//                                         let new_tempo = Mpb(Ratio::<BigInt>::from_integer(
//                                             BigInt::from_f64(60_000_000. / y.min(max).max(min))
//                                                 .unwrap(),
//                                         ));
//                                         *tempos_ = repeat_n(new_tempo.clone(), len).collect();

//                                         if let Some(lengths) = state.lengths_mut(*ctx) {
//                                             if let Some(lengths) = lengths.get_mut(idx.clone()) {
//                                                 lengths.iter_mut().for_each(|length| {
//                                                     let dur = tmp.0.to_integer().to_f64().unwrap()
//                                                         / length.as_f64()
//                                                         * QUARTER_NOTE_RATIO;
//                                                     dbg!(
//                                                         length.as_f64(),
//                                                         tmp.0.to_integer().to_f64(),
//                                                         dur,
//                                                         new_tempo.0.to_integer().to_f64().unwrap()
//                                                     );

//                                                     let len = Length::MicroSeconds(
//                                                         new_tempo.0.clone(),
//                                                     ) / Length::MicroSeconds(
//                                                         Ratio::<BigInt>::from_integer(
//                                                             BigInt::from_f64(
//                                                                 dur / QUARTER_NOTE_RATIO,
//                                                             )
//                                                             .unwrap_or_else(|| {
//                                                                 error(format!(
//                                                                     "missing duration values"
//                                                                 ));
//                                                                 exit(1)
//                                                             }),
//                                                         ),
//                                                     );
//                                                     length_ = len;
//                                                     *length = length_.clone();
//                                                 });
//                                             }
//                                         }
//                                     }
//                                 }
//                             }
//                         }
//                         *counter += length_.as_f64() / ctx_len.as_f64();
//                     }
//                 }

//                 x = x + step;
//             }
//         })),
//     );

//     Monad::ret((state.next().unwrap_or_default(), ctx))
// }

// fn compose_interpolation(ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
//     let mut lhs = state.load().unwrap_or_default();

//     if let Exp::Simple(Simple::Suffix(Suffix::Bpm)) = lhs {
//         if let Some(Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) = state.next() {
//             lhs = Exp::Simple(Simple::Scalar(Scalar::Tempo(abs)));
//         }
//     }

//     c! {{
//         eprintln!("{ctx:?} {lhs:?}");
//         pause();
//     }}

//     let mut scalars = Vec::<Scalar>::new();
//     if let Exp::Simple(Simple::Scalar(scalar)) = lhs {
//         c! {{
//             eprintln!("{ctx:?} {scalar:?}");
//             pause();
//         }}

//         match scalar {
//             Scalar::Pure(Pure::Absolute(absolute)) => {
//                 let next = state.next().unwrap_or_default();
//                 c! {{
//                     eprintln!("{ctx:?} {next:?}");
//                     pause();
//                 }}
//                 if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                     scalars.push(Scalar::Tempo(absolute))
//                 } else {
//                     state.push_back(next.clone());
//                     scalars.push(scalar);
//                 }
//             }
//             scalar => {
//                 scalars.push(scalar);
//             }
//         }
//     }
//     c! {{
//         let next = state.next().unwrap_or_default();
//         eprintln!("{ctx:?} {scalars:?} {:?}", next.clone());
//         pause();
//         state.push_back(next.clone());
//     }}

//     match state.next().unwrap_or_default() {
//         Exp::Compound(compound) => todo!(),
//         Exp::Simple(simple) => match simple {
//             Simple::Prefix(prefix) => match prefix {
//                 Prefix::Dur => {
//                     if let Some(Exp::Simple(Simple::Scalar(Scalar::Pure(pure)))) = state.next() {
//                         match pure {
//                             Pure::Relative(relative) => todo!(),
//                             Pure::Absolute(absolute) => {
//                                 scalars.push(Scalar::Duration(Duration::Fractional(Fractional::Absolute(absolute))));
//                             },
//                             Pure::Rational(rational) => {
//                                 scalars.push(Scalar::Duration(Duration::Fractional(Fractional::Rational(rational))))
//                             },
//                         }
//                     }
//                 },
//                 Prefix::Reg => todo!(),
//                 Prefix::Prog => todo!(),
//             },
//             Simple::Scalar(scalar) => match scalar {
//                 Scalar::Pure(Pure::Absolute(absolute)) => {
//                     let next = state.next().unwrap_or_default();
//                     if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                         scalars.push(Scalar::Tempo(absolute))
//                     } else {
//                         state.push_back(next.clone());
//                         scalars.push(scalar);
//                     }
//                 }
//                 scalar => {
//                     scalars.push(scalar);
//                 }
//             },
//             Simple::Infix(infix) => todo!(),
//             Simple::Suffix(suffix) => todo!(),
//             Simple::Decl(decl) => todo!(),
//             Simple::Controller(controller_type) => todo!(),
//             Simple::Ident(ident) => todo!(),
//         },
//         Exp::Decl(decl) => todo!(),
//         Exp::Noop => todo!(),
//         Exp::EOI => todo!(),
//     }
//     if let Some(Exp::Simple(Simple::Scalar(scalar))) = state.next() {
//         c! {{
//             eprintln!("{ctx:?} {scalar:?}");
//             pause();
//         }}
//     }

//     let mut next = state.next().unwrap_or_default();

//     while matches!(next, Exp::Simple(Simple::Infix(Infix::Interpolation(_)))) {
//         let rhs = state.next().unwrap_or_default();
//         scalars.push(match rhs.clone() {
//             Exp::Simple(simple) => match simple {
//                 Simple::Prefix(prefix) => match prefix {
//                     Prefix::Dur => {
//                         let next = state.next().unwrap_or_default();
//                         match next {
//                             Exp::Simple(Simple::Scalar(scalar)) => match scalar {
//                                 Scalar::Pure(pure) => match pure {
//                                     Pure::Relative(relative) => todo!(),
//                                     Pure::Absolute(absolute) => Scalar::Duration(
//                                         Duration::Fractional(Fractional::Absolute(absolute)),
//                                     ),
//                                     Pure::Rational(rational) => todo!(),
//                                 },
//                                 _ => todo!(),
//                             },
//                             _ => todo!(),
//                         }
//                     }
//                     Prefix::Reg => todo!(),
//                     Prefix::Prog => todo!(),
//                 },
//                 Simple::Scalar(scalar) => match scalar {
//                     ref scalar @ Scalar::Pure(Pure::Absolute(ref absolute)) => {
//                         let next = state.next().unwrap_or_default();
//                         if matches!(next, Exp::Simple(Simple::Suffix(Suffix::Bpm))) {
//                             Scalar::Tempo(*absolute)
//                         } else {
//                             scalar.clone()
//                         }
//                     }
//                     scalar => scalar,
//                 },

//                 _ => todo!(),
//             },
//             _ => todo!(),
//         });

//         next = state.next().unwrap_or_default()
//     }

//     state.push_back(next.clone());

//     let first = scalars.first().cloned().unwrap();

//     let thunk = Thunk::new(Box::new(move |ctx, state| {
//         // dbg!();
//         // print_state(state, ctx);
//         // state
//         //     .children(ctx)
//         //     .iter()
//         //     .cloned()
//         //     .for_each(|ctx| print_state(state, ctx));
//         // pause();

//         let max: f64 = scalars.iter().cloned().fold(0.0, |max, scalar| {
//             max.max(scalar_to_f64(ctx, state, &scalar))
//         });
//         let min: f64 = scalars.iter().cloned().fold(f64::MAX, |min, scalar| {
//             min.min(scalar_to_f64(ctx, state, &scalar))
//         });

//         let interval = 1.0 / (scalars.len() as f64);

//         // c!{{
//         //     eprintln!("interval: {interval}");
//         // }}

//         let points: Vec<(f64, f64)> = scalars
//             .iter_mut()
//             .enumerate()
//             .map(|(idx, scalar)| {
//                 let x = idx as f64 * interval;
//                 let y = scalar_to_f64(ctx, state, &scalar).min(max).max(min)
//                     / match first {
//                         Scalar::Duration(_) => 1.0,
//                         Scalar::Frequency(_) => todo!(),
//                         Scalar::Pure(_) => 127.0,
//                         Scalar::Dynamic(_) => 127.0,
//                         Scalar::Tuplet(_) => 127.0,
//                         Scalar::Prog(_) => 127.0,
//                         Scalar::Register(_) => 8.0,
//                         Scalar::Rest => 127.0,
//                         Scalar::Tempo(_) => 1.0,
//                     };
//                 (x, y)
//             })
//             .collect();

//         dbg!(&points);
//         pause();
//         let children = state.children(ctx);
//         let mut ctxs = if children.is_empty() {
//             vec![ctx]
//         } else {
//             children
//         };
//         let first_ctx = ctxs.first().cloned().unwrap_or_default();
//         let mut ctx_len: Length = state.get_ctx_length(first_ctx);
//         if ctx_len.as_u64() == 0 {
//             ctx_len = Length::MicroSeconds(
//                 state
//                     .tempos(first_ctx)
//                     .unwrap_or_default()
//                     .iter()
//                     .cloned()
//                     .map(|tmps| {
//                         tmps.iter()
//                             .cloned()
//                             .min_by(|a, b| a.0.cmp(&b.0))
//                             .unwrap_or_default()
//                             .0
//                     })
//                     .sum(),
//             );
//         }
//         // dbg!(ctx_len.as_u64());
//         let mut length_ = Length::default_max();

//         let mut ctx_iter = ctxs.iter().cloned();
//         let init = state
//             .pcs(ctx_iter.next().unwrap_or_default())
//             .unwrap_or_default()
//             .len() as f64;

//         let max_pc_len = ctx_iter.fold(init, |a, b| {
//             a.max(state.pcs(b).unwrap_or_default().len() as f64)
//         });

//         // let mut step = 1. / state.pcs(ctx).unwrap_or_default().len() as f64;
//         let mut step = 1. / max_pc_len as f64;

//         // c! {{
//         //     eprintln!("step: {step}");
//         // }}
//         // // print_state(state, ctx);
//         // // dbg!(step);
//         // pause();

//         let mut lengths = Vec::<Vec<(usize, Vec<Length>)>>::new();
//         let mut counters = Vec::<f64>::new();

//         ctxs.iter().cloned().for_each(|ctx| {
//             lengths.push(
//                 state
//                     .lengths(ctx)
//                     .unwrap_or_default()
//                     .clone()
//                     .into_iter()
//                     .enumerate()
//                     .rev()
//                     .collect::<Vec<_>>(),
//             );
//             counters.push(0.0);
//         });

//         let mut iters = zip(ctxs.clone(), zip(lengths, counters)).collect::<Vec<_>>();
//         // let mut iters = iters.iter_mut();
//         let mut x: f64 = 0.0;

//         // while x <= 1.0 {
//         // dbg!(x);
//         // let mut min_lengths = Vec::<Length>::new();
//         // step = Length::default_max().as_f64();
//         while iters.iter().any(|(_, (lengths, _))| lengths.len() > 0) {
//             for (ctx, (lengths, counter)) in iters.iter_mut() {
//                 if state.pcs(*ctx).unwrap_or_default().len() == 0 {
//                     continue;
//                 }
//                 // dbg!(&counter);

//                 // if *counter <= x {
//                 let Some((idx, lengths)) = lengths.pop() else {
//                     continue;
//                 };

//                 // c! {{
//                 //     print_state(state, *ctx);
//                 //     eprintln!("{ctx:?} lens: {}, counter: {}, x: {x}",
//                 //         lengths.iter().map(Length::as_u64).collect::<Vec<_>>().iter()
//                 //             .map(u64::to_string).collect::<Vec<_>>().join(" "), counter);

//                 //     pause();
//                 // }}

//                 let mut length = lengths.first().cloned().unwrap_or_default()
//                         // .max(min.into()).min(max.into())
//                     ;
//                 let mut iter = points.iter().cloned();
//                 let init = iter.next().unwrap_or_default();
//                 let y = points.windows(2).fold(0.0, |y, window| {
//                     let (x0, y0) = window[0];
//                     let (x1, y1) = window[1];
//                     c! {{
//                         eprintln!("x0: {x0}, y0: {y0}, x1: {x1}, y1: {y1}");
//                     }}
//                     y + if *counter > x0 && *counter <= x1 {
//                         *counter * (x1 - x0 / y1 - y0)
//                     } else {
//                         0.0
//                     }
//                 });
//                 c! {{
//                     eprintln!("y: {y}");
//                     pause();
//                 }}

//                 // dbg!(y);
//                 match first {
//                     Scalar::Duration(_) => {
//                         // dbg!();
//                         let micros = Duration::Fractional(Fractional::Absolute(Absolute::Float(y)))
//                             .to_micros(state);

//                         length = Length::MicroSeconds(micros.clone());

//                         if let Some(lengths) = state.lengths_mut(ctx.clone()).get_mut(idx.clone()) {
//                             *lengths =
//                                 repeat_n(Length::MicroSeconds(micros), lengths.len()).collect();
//                         } else {
//                             state.add(*ctx, vec![Length::MicroSeconds(micros)]);
//                         }
//                     }
//                     Scalar::Frequency(_) => todo!(),
//                     Scalar::Pure(_) => {
//                         todo!()
//                     }
//                     Scalar::Dynamic(_) => {
//                         let val = f64::ceil(y * u7::max_value().as_int() as f64)
//                             .min(max)
//                             .max(min);
//                         let vel = Velocity::from(val as u8);

//                         if let Some(velocities) = state.velocities_mut(*ctx).get_mut(idx.clone()) {
//                             *velocities = vec![Velocity::from(vel)];
//                         } else {
//                             state.add(*ctx, vec![Velocity::from(vel)]);
//                         }
//                     }
//                     Scalar::Tuplet(_) => todo!(),
//                     Scalar::Prog(_) => todo!(),
//                     Scalar::Register(_) => todo!(),
//                     Scalar::Rest => todo!(),
//                     Scalar::Tempo(_) => {
//                         if let Some(tempos) = state.tempos_mut(*ctx).get_mut(idx.clone()) {
//                             let tmp = tempos.first().cloned().unwrap_or_default();
//                             let len = tempos.len();
//                             let new_tempo = Mpb(Ratio::<BigInt>::from_integer(
//                                 BigInt::from_f64(60_000_000. / y.min(max).max(min)).unwrap(),
//                             ));
//                             *tempos = repeat_n(new_tempo.clone(), len).collect();

//                             if let Some(lengths) = state.lengths_mut(*ctx).get_mut(idx.clone()) {
//                                 lengths.iter_mut().for_each(|length| {
//                                     let dur = tmp.0.to_integer().to_f64().unwrap()
//                                         / length.as_f64()
//                                         * QUARTER_NOTE_RATIO;
//                                     let len = Length::MicroSeconds(new_tempo.0.clone())
//                                         / Length::MicroSeconds(
//                                             Ratio::<BigInt>::from_float(dur / QUARTER_NOTE_RATIO)
//                                                 .unwrap_or_else(|| {
//                                                     error(format!("missing duration values"));
//                                                     exit(1)
//                                                 }),
//                                         );

//                                     // step = gcd(step.clone(), len.clone());
//                                     length_ = len.clone();

//                                     *length = length_.clone();
//                                 });
//                             }
//                         }
//                     }
//                 }
//                 // *counter += length.as_f64() / step;

//                 *counter += length.as_f64() / ctx_len.as_f64();

//                 // c! {{
//                 //     eprintln!("{ctx:?} length: {} ctx_length: {} counter: {counter} x: {x}", length.as_f64(), ctx_len.as_u64());
//                 // }}

//                 // c! {{print_state(state, *ctx);}}

//                 // pause();
//                 // }
//                 // c!{{
//                 //     print_state(state, *ctx);
//                 // }}
//                 // pause();
//             }
//         }

//         // step = min_lengths.iter().cloned().min().unwrap_or_default().as_f64();
//         // dbg!(step);

//         x = x + step;

//         // dbg!(x, step);

//         // print_state(state, ctx);

//         // }

//         // c! {{ ctxs.clone().iter().cloned().for_each(|ctx| {
//         //      print_state(state, ctx);
//         //  });
//         // pause();}}

//         // dbg!();
//         // print_state(state, ctx);
//         // state
//         //     .children(ctx)
//         //     .iter()
//         //     .cloned()
//         //     .for_each(|ctx| print_state(state, ctx));
//         // pause();
//     }));

//     state.add_thunk(ctx, LifeCycleEvent::PreSequence, thunk.clone());

//     Monad::ret((state.next().unwrap_or_default(), ctx))
// }

// fn compose_interpolation(
//     interpolation: Interpolation,
//     ctx: Ctx,
//     state: &mut State,
// ) -> Monad<(Exp, Ctx)> {
//     // eprintln!(
//     //     "{}COMPOSE INTERPOLATION {ctx:?} {interpolation:?}",
//     //     color!()
//     // );
//     // print_stacks(state);
//     // pause();
//     // eprintln!("{}", std::backtrace::Backtrace::capture().to_string());
//     // dbg!();
//     // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));
//     let lhs = state.load().unwrap_or_default();
//     // dbg!(&lhs);
//     // pause();
//     let mut data: Vec<Vec<Data>> =
//         if let ref exp @ Exp::Simple(Simple::Scalar(ref lhs)) = lhs.clone() {
//             let rhs = state.next().unwrap_or_default();
//             // dbg!(&rhs);
//             let mut data = Vec::<Vec<Data>>::new();
//             match rhs.clone() {
//                 Exp::Simple(Simple::Scalar(rhs)) => {
//                     // eprintln!("{}{lhs:?} {interpolation} {rhs:?}", color!());
//                     // pause();
//                     match (lhs.clone(), rhs) {
//                         (Scalar::Duration(lhs_duration), Scalar::Duration(rhs_duration)) => {
//                             // dbg!(&lhs_duration, &rhs_duration);
//                             let lhs_length = duration_to_length(lhs_duration, ctx, state);
//                             let rhs_length = duration_to_length(rhs_duration, ctx, state);
//                             // state.add_length(ctx, lhs_length);
//                             // graph(state, state.parent(ctx), 3);
//                             // print_state(state, ctx);

//                             data.push(vec![lhs_length.into(), rhs_length.into()]);
//                         }
//                         (Scalar::Register(lhs), Scalar::Register(rhs)) => {
//                             data.push(vec![lhs.into(), rhs.into()]);
//                         }
//                         (Scalar::Frequency(lhs), Scalar::Frequency(rhs)) => todo!(),
//                         (Scalar::Pure(lhs), Scalar::Pure(rhs)) => todo!(),
//                         (Scalar::Dynamic(lhs), Scalar::Dynamic(rhs)) => {
//                             let lhs_velocity = Velocity::from(lhs);
//                             let rhs_velocity = Velocity::from(rhs);
//                             // graph(state, state.parent(ctx), 5);
//                             // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));
//                             data.push(vec![lhs_velocity.into(), rhs_velocity.into()]);
//                         }
//                         (Scalar::Tuplet(lhs), Scalar::Tuplet(rhs)) => todo!(),
//                         (Scalar::Prog(lhs), Scalar::Prog(rhs)) => todo!(),
//                         (Scalar::Rest, Scalar::Rest) => todo!(),
//                         _ => todo!(),
//                     }
//                 }
//                 _ => {
//                     state.push_back(rhs);
//                 }
//             }
//             let mut next = state.next().unwrap_or_default();
//             // dbg!(&next);
//             while matches!(next, Exp::Simple(Simple::Infix(Infix::Interpolation(_)))) {
//                 if let Exp::Simple(Simple::Infix(Infix::Interpolation(lerp))) = next.clone() {
//                     let rhs = state.next().unwrap_or_default();
//                     // dbg!(&rhs);
//                     if let Exp::Simple(Simple::Scalar(scalar)) = rhs.clone() {
//                         let t: Data = match scalar {
//                             Scalar::Duration(duration) => duration.into(),
//                             Scalar::Frequency(absolute) => todo!(),
//                             Scalar::Pure(pure) => match pure {
//                                 Pure::Relative(relative) => todo!(),
//                                 Pure::Absolute(absolute) => Pc::from(absolute).into(),
//                                 Pure::Rational(rational) => todo!(),
//                             },
//                             Scalar::Dynamic(dynamic) => Velocity::from(dynamic).into(),
//                             Scalar::Tuplet(tuplet) => todo!(),
//                             Scalar::Prog(prog) => prog.into(),
//                             Scalar::Rest => todo!(),
//                             Scalar::Register(register) => todo!(),
//                             Scalar::Tempo(absolute) => todo!(),
//                         };
//                         let last = data
//                             .last()
//                             .cloned()
//                             .unwrap_or_default()
//                             .last()
//                             .cloned()
//                             .unwrap_or_default();
//                         data.push(vec![last, t])
//                     } else {
//                         state.push_back(rhs.clone());
//                     }
//                 }
//                 *&mut next = state.next().unwrap_or_default();
//             }
//             state.push_back(next);
//             data
//         } else {
//             vec![]
//         };

//     // dbg!(&data);

//     let thunk = match data
//         .first()
//         .cloned()
//         .unwrap_or_default()
//         .first()
//         .cloned()
//         .unwrap_or_default()
//     {
//         Data::Pc(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Pc>(ctx, data.clone(), state)
//         })),
//         Data::Length(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Length>(ctx, data.clone(), state)
//         })),
//         Data::Velocity(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Velocity>(ctx, data.clone(), state)
//         })),
//         Data::Tempo(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Tempo>(ctx, data.clone(), state)
//         })),
//         Data::Register(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Register>(ctx, data.clone(), state)
//         })),
//         Data::Interpolation(interpolation) => todo!(),
//         Data::Program(_) => Thunk::new(Box::new(move |ctx, state| {
//             interpolate::<Prog>(ctx, data.clone(), state)
//         })),
//         Data::None => todo!(),
//         Data::Controller(hash_map) => todo!(),
//     };
//     // eprintln!("{}{:#?}", color!(), thunk);
//     // pause();
//     state.push_thunk(LifeCycleEvent::PreSequence, thunk);

//     Monad::ret((state.next().unwrap_or_default(), ctx))
// }

fn interpolate<T: Interpolant + ToString>(ctx: Ctx, data: Vec<Vec<Data>>, state: &mut State)
where
    Data: From<T>,
{
    // eprintln!("{}INTERPOLATE {ctx:?}", color!());
    // data.iter().for_each(|data| eprintln!("{data:?}"));

    // print_state(state, ctx);
    // misc(state_str(state, ctx));
    // pause();

    let leaves = state.get_leaves(ctx);
    // eprintln!("LEAVES: {leaves:?}");
    let pc_len = get_pc_count(&leaves, state);
    let pc_count = pc_len as f64 / data.len() as f64;
    let step = 1.0 / pc_count;
    // dbg!(pc_len, pc_count, step);

    let mut data__ = Vec::<T>::new();
    data.iter().cloned().for_each(|data_| {
        let mut t: f64 = 0.0;
        let ts: Vec<T> = data_.iter().cloned().map(T::from).collect();
        while t <= 1.0 {
            // eprint!("{}t: {t}{ResetColor} ", color!());
            let value = <T as From<f64>>::from(bezier(ts.clone(), t));
            data__.push(value);
            t += step;
        }
        // eprint!("\n");
    });

    // eprintln!("DATA: {data__:?}");

    let ts: Vec<T> = state.get(ctx);

    fill_leaves(state, &leaves, data__);
    // eprintln!("{:?}", ts);
    // print_state(state, ctx);
    // misc(state_str(state, ctx));
    // print_states(state, Ctx::Root);
    // pause();
}

fn get_pc_count(leaves: &Vec<Ctx>, state: &State) -> usize {
    let pc_len: usize = leaves.iter().cloned().fold(0, |len, ctx| {
        // print_state(state, ctx);
        len + match state.scope(ctx) {
            Scope::Sequence => state.pcs(ctx).unwrap_or_default().len(),
            Scope::Stack => 1,
            Scope::Set => todo!(),
            Scope::None => todo!(),
        }
    });
    pc_len
}

fn fill_leaves<T: Interpolant>(state: &mut State, leaves: &Vec<Ctx>, mut data: Vec<T>) {
    // eprintln!("{}FILL LEAVES: {leaves:?}\n", color!());
    let mut ts: Vec<Vec<Vec<T>>> = leaves
        .iter()
        .cloned()
        .map(|ctx| T::get_vec(state, ctx).unwrap_or_default())
        .collect();

    let mut data_iter = data.iter().cloned();

    leaves.iter().cloned().for_each(|ctx| {
        T::get_vec_mut(state, ctx).iter_mut().for_each(|ts| {
            let t_ = data_iter.next().unwrap_or_default();
            ts.iter_mut().for_each(|t| {
                *t = t_.clone();
            });
        });
    });

    // print_states(
    //     state,
    //     state.parent(leaves.first().cloned().unwrap_or_default()),
    // );
    // pause();
}

fn duration_to_length(duration: Duration, ctx: Ctx, state: &mut State) -> Length {
    // dbg!(&duration);
    match duration {
        Duration::Fixed(fixed) => fixed.into(),
        Duration::Fractional(fractional) => match fractional {
            Fractional::Absolute(absolute) => Length::from(
                QUARTER_NOTE_RATIO / absolute
                    * state.get::<Tempo>(ctx).iter().cloned().min().unwrap(),
            ),
            Fractional::Tuplet(tuplet) => {
                let Tuplet { lhs, rhs } = tuplet;
                let length = state.get::<Length>(ctx);
                (rhs / lhs).into()
            }
            Fractional::Rational(rational) => {
                let Rational { num, den } = rational;
                let length = state.get::<Length>(ctx);
                Length::from(num) / Length::from(den) * length.iter().cloned().min().unwrap()
            }
        },
    }
}

fn compose_suffix(suffix: Suffix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!(
    //     "{}COMPOSE SUFFIX {suffix:?} {ctx:?} {:?}",
    //     color!(),
    //     state.scope(ctx)
    // );
    // pause();
    let lhs = state.load().unwrap_or_default();

    // c! {{
    //     eprintln!("{lhs} {suffix:?}");
    //     pause();
    // }}

    if let Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(lhs)))) = lhs {
        // eprintln!("{}{lhs:?}", color!());
        match suffix {
            Suffix::Amp => state.add::<Velocity>(ctx, vec![lhs.into()]),
            Suffix::Bpm => {
                if let Some(next) = state.next() {
                    if matches!(next, Exp::Simple(Simple::Infix(Infix::Range))) {
                        // dbg!(&next);
                        // pause();
                        state.store(Exp::Simple(Simple::Scalar(Scalar::Tempo(lhs))));
                        return compose_interpolation(Infix::Range, ctx, state);
                    } else {
                        state.push_back(next.clone());
                    }
                }
                state.set_bpm(lhs);
                state.add(
                    ctx,
                    vec![Mpb(Ratio::<BigInt>::from_integer(
                        BigInt::from_f64(60_000_000. / lhs.as_f64()).unwrap(),
                    ))],
                );
            }
            Suffix::Freq => todo!(),
        }
        Monad::ret((state.next().unwrap_or_default(), ctx))
    } else {
        Monad::ret((lhs, ctx))
    }
}

fn compose_scalar(scalar: Scalar, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE SCALAR {ctx:?} {scalar:?}", color!());

    match scalar {
        Scalar::Duration(duration) => compose_duration(duration, ctx, state),
        Scalar::Frequency(absolute) => compose_absolute(absolute, ctx, state),
        Scalar::Pure(pure) => compose_pure(pure, ctx, state),
        Scalar::Dynamic(dynamic) => compose_dynamic(dynamic, ctx, state),
        Scalar::Rest => compose_rest(ctx, state),
        Scalar::Tuplet(tuplet) => {
            // compose_fractional_duration(Fractional::Tuplet(tuplet), ctx, state).bind()
            todo!()
        }
        Scalar::Prog(prog) => compose_prog(prog, ctx, state),
        Scalar::Register(register) => {
            compose_register(register, ctx, state)
            // state.add(ctx, vec![register]);
            // Monad::ret((state.next().unwrap_or_default(), ctx))
        }
        Scalar::Tempo(absolute) => todo!(),
    }
}

fn compose_absolute(absolute: Absolute, mut ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // log(format!(
    //     "{}COMPOSE ABSOLUTE {absolute:?} {ctx:?} {:?}",
    //     color!(),
    //     state.scope(ctx)
    // ));

    // c! {{eprintln!(
    //     "{}COMPOSE ABSOLUTE {absolute:?} {ctx:?} {:?}",
    //     color!(),
    //     state.scope(ctx)
    // );
    // pause();
    // }}

    match absolute {
        Absolute::UInt(uint) => {
            state.add(ctx, vec![Pc::Class(uint as f64)]);
        }
        Absolute::Float(float) => {
            state.add(ctx, vec![Pc::Class(float)]);
        }
    }
    state.call_thunks(ctx, ctx, LifeCycleEvent::Composition);
    state.pad(ctx);

    // c! {{
    //     print_state(state, ctx);
    //     pause();
    // }}
    // eprintln!("{:?}", state.pcs(ctx).unwrap_or_default());
    let next = state.next().unwrap_or_default();
    // dbg!(&next);
    Monad::ret((next, ctx))
}

fn compose_pure(pure: Pure, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let next = state.next().unwrap_or_default();
    match next {
        Exp::Simple(Simple::Suffix(suffix)) => match suffix {
            Suffix::Amp => todo!(),
            Suffix::Bpm => match pure {
                Pure::Relative(relative) => todo!(),
                Pure::Absolute(absolute) => {
                    state.add::<Pc>(ctx, vec![absolute.into()]);
                    // eprintln!("{:?}", state.pcs(ctx).unwrap_or_default());
                    Monad::ret((state.next().unwrap_or_default(), ctx))
                }
                Pure::Rational(rational) => todo!(),
            },
            Suffix::Freq => todo!(),
        },
        Exp::Simple(Simple::Infix(infix)) => compose_infix(infix, ctx, state),
        next => {
            // dbg!(&next);
            // pause();
            state.push_back(next);
            match pure {
                Pure::Relative(relative) => compose_relative::<Pc>(relative, ctx, state),
                Pure::Absolute(absolute) => compose_absolute(absolute, ctx, state),
                Pure::Rational(rational) => {
                    error(format!(
                        "Syntax error: {}/{}",
                        rational.num.as_f64(),
                        rational.den.as_f64()
                    ));
                    exit(1);
                }
            }
        }
    }
}

fn compose_relative<T>(relative: Relative, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)>
where
    T: std::fmt::Debug
        + Default
        + Clone
        + Into<Data>
        + From<Data>
        + Ord
        + From<Absolute>
        + Add<Output = T>
        + Sub<Output = T>
        + ToString
        + 'static,
    Data: From<T>,
{
    // c! {{
    //     eprintln!("COMPOSE RELATIVE {ctx:?} {:?} {relative:?}", state.get(ctx).iter().cloned().min().unwrap());
    //     pause();
    // }}

    let thunk = Thunk::new(Box::new(move |ctx_, state| {
        let t: T = state.get(ctx_).iter().cloned().min().unwrap();
        // dbg!(&t);

        let Relative { sign, val } = relative;
        // dbg!(&sign, val, &t);
        let t_: T = match sign {
            Sign::Plus => t + T::from(val),
            Sign::Minus => t - T::from(val),
        };

        // dbg!(&t_);

        // pause();
        state.add(ctx_, vec![t_.clone()]);
        // c! {{
        //     print_state(state, ctx);
        //     pause();
        // }}
    }));

    // state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());

    let thunk_clone = thunk.clone();
    state.add_thunk(
        ctx,
        LifeCycleEvent::PreComposition,
        Thunk::new(Box::new(move |ctx_, state| {
            state.add_thunk(ctx_, LifeCycleEvent::Composition, thunk_clone.clone());
            // c! {{
            //     print_state(state, ctx);
            //     pause();
            // }}
        })),
    );

    // state.add_thunk(
    //     ctx,
    //     LifeCycleEvent::PreComposition,
    //     Thunk::new(Box::new(move |ctx_, state| {
    //         let parent = state.parent(ctx_);
    //         state.call_thunks(parent, ctx_, LifeCycleEvent::Composition);
    //     })),
    // );

    state.add_thunk(ctx, LifeCycleEvent::Composition, thunk.clone());

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_dynamic(dynamic: Dynamic, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // info(format!("{}COMPOSE DYNAMIC {ctx:?} {:?}", color!(), dynamic));
    let velocity: u8 = dynamic.as_f64() as u8;

    let thunk = Thunk::new(Box::new(move |ctx, state| {
        state.add(ctx, vec![Velocity(velocity)]);
    }));

    state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());
    state.add_thunk(ctx, LifeCycleEvent::Composition, thunk.clone());

    // state.add_thunk(
    //     ctx,
    //     LifeCycleEvent::PreComposition,
    //     Thunk::new(Box::new(move |ctx_, state| {
    //         state.add_thunk(ctx_, LifeCycleEvent::Composition, thunk.clone());
    //     })),
    // );

    // let next = state.next().unwrap_or_default();
    // match next {
    //     Exp::Compound(compound) => {
    //         // let exps = compound.to_vecdeque();
    //         // let exps: Vec<Exp> = exps
    //         //     .into_iter()
    //         //     .flat_map(|exp| {
    //         //         if matches!(exp, Exp::Simple(Simple::Scalar(Scalar::Pure(_)))) {
    //         //             vec![
    //         //                 Exp::Simple(Simple::Scalar(Scalar::Dynamic(dynamic.clone()))),
    //         //                 exp,
    //         //             ]
    //         //         } else {
    //         //             vec![exp]
    //         //         }
    //         //     })
    //         //     .collect();

    //         let velocity: u8 = dynamic.as_f64() as u8;

    //         let thunk = Thunk::new(Box::new(move |ctx, state| {
    //             state.add(ctx, vec![Velocity(velocity)]);
    //         }));
    //         state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());

    //         state.add_thunk(ctx, LifeCycleEvent::Composition, thunk);
    //     }
    //     Exp::Simple(simple) => {
    //         let velocity: u8 = dynamic.as_f64() as u8;
    //         let thunk = Thunk::new(Box::new(move |ctx, state| {
    //             state.add(ctx, vec![Velocity(velocity)]);
    //         }));
    //         state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());
    //         state.add_thunk(ctx, LifeCycleEvent::Composition, thunk);
    //     }
    //     Exp::Noop => todo!(),
    //     Exp::EOI => todo!(),
    //     Exp::Decl(decl) => todo!(),
    // }

    // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));
    // eprintln!("{:?}", state.velocities(ctx).unwrap_or_default());
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_ident(ident: Ident, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let exp = state.binding(ctx, &ident).unwrap_or_default();
    // dbg!(&exp);
    Monad::ret((exp, ctx))
}

fn compose_duration(duration: Duration, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    match duration {
        Duration::Fixed(fixed) => composed_fixed_duration(fixed, ctx, state),
        Duration::Fractional(fractional) => compose_fractional_duration(fractional, ctx, state),
    }
}

fn composed_fixed_duration(fixed: Fixed, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // info(format!(
    //     "{}COMPOSE FIXED DURATION {fixed:?} {ctx:?}",
    //     color!()
    // ));

    let thunk = Thunk::new(Box::new(move |ctx, state| {
        state.add::<Length>(ctx, vec![fixed.clone().into()]);
    }));
    state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());
    state.add_thunk(ctx, LifeCycleEvent::Composition, thunk);

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_fractional_duration(
    fractional: Fractional,
    ctx: Ctx,
    state: &mut State,
) -> Monad<(Exp, Ctx)> {
    // eprintln!(
    //     "{}COMPOSE FRACTIONAL DURATION {fractional:?} {ctx:?}",
    //     color!()
    // );
    let next = state.next().unwrap_or_default();

    match fractional {
        Fractional::Absolute(absolute) => {
            // dbg!(absolute.as_f64());
            let float = absolute.as_f64() / QUARTER_NOTE_RATIO;
            let mut microseconds =
                state.tempo().0.clone() / Ratio::<BigInt>::from_float(float).unwrap_or_default();

            let dur = QUARTER_NOTE_RATIO * microseconds.to_integer().to_f64().unwrap();
            let ratio = Ratio::<BigInt>::from_integer(BigInt::from_f64(dur).unwrap())
                * state.tempo().clone().0;
            // dbg!(dur, ratio);

            // pause();

            let thunk = Thunk::new(Box::new(move |ctx_, mut state| {
                let count = match state.scope(ctx_) {
                    Scope::Sequence => 1,
                    Scope::Stack => state
                        .pcs(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .unwrap_or_default()
                        .len(),
                    Scope::Set => todo!(),
                    Scope::None => todo!(),
                };
                state.add(ctx_, vec![Length::MicroSeconds(microseconds.clone())]);
                // print_state(state, ctx_);
                // pause();
            }));

            let thunk_clone = thunk.clone();

            // state.add_thunk(
            //     ctx,
            //     LifeCycleEvent::PreComposition,
            //     Thunk::new(Box::new(move |ctx_, state| {
            //         state.add_thunk(ctx_, LifeCycleEvent::PreComposition, thunk.clone());
            //         state.add_thunk(ctx_, LifeCycleEvent::Composition, thunk.clone());
            //     })),
            // );

            state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());
            state.add_thunk(ctx, LifeCycleEvent::Composition, thunk_clone.clone());

            Monad::ret((next, ctx))
        }
        Fractional::Tuplet(tuplet) => {
            let Tuplet { lhs, rhs } = tuplet;
            let len = Length::MicroSeconds(
                Ratio::<BigInt>::from_float(rhs.as_f64() / lhs.as_f64()).unwrap(),
            ) * state.get::<Length>(ctx).iter().cloned().min().unwrap();

            let thunk = Thunk::new(Box::new(move |ctx, state| {
                state
                    .get_last_mut(ctx)
                    .and_then(|length| {
                        std::mem::swap(length, &mut len.clone());
                        Some(())
                    })
                    .or_else(|| {
                        state.add(ctx, vec![len.clone()]);
                        Some(())
                    });
            }));
            state.add_thunk(ctx, LifeCycleEvent::Composition, thunk.clone());
            state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk);

            Monad::ret((next, ctx))
        }
        Fractional::Rational(rational) => {
            let Rational { num, den } = rational;
            let microseconds = Ratio::<BigInt>::from_integer(
                BigInt::from_f64(
                    num.as_f64() / den.as_f64()
                        * QUARTER_NOTE_RATIO
                        * state.get::<Tempo>(ctx).iter().cloned().min().unwrap(),
                )
                .unwrap(),
            );

            let thunk = Thunk::new(Box::new(move |ctx, state| {
                state.add(ctx, vec![Length::MicroSeconds(microseconds.clone())]);
                // print_state(state, ctx);
                // pause();
            }));

            let thunk_clone = thunk.clone();

            // state.add_thunk(
            //     ctx,
            //     LifeCycleEvent::PreComposition,
            //     Thunk::new(Box::new(move |ctx_, state| {
            //         state.add_thunk(ctx_, LifeCycleEvent::Composition, thunk.clone());
            //     })),
            // );
            state.add_thunk(ctx, LifeCycleEvent::Composition, thunk_clone.clone());
            state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk);

            Monad::ret((next, ctx))
        }
    }
}

fn compose_register(reg: Register, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // c! {{ eprintln!("{}COMPOSE REGISTER {reg:?} {ctx:?}", color!()); }}

    let next = state.next().unwrap_or_default();

    let thunk = Thunk::new(Box::new(move |ctx, state| state.add(ctx, vec![reg])));

    state.add_thunk(ctx, LifeCycleEvent::PreComposition, pre_comp(thunk.clone()));
    // state.add_thunk(ctx, LifeCycleEvent::PreComposition, thunk.clone());
    state.add_thunk(ctx, LifeCycleEvent::Composition, thunk);

    Monad::ret((next, ctx))
}

fn compose_rest(ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE REST {ctx:?}", color!());
    // print_state(state, ctx);

    // state.add_thunk(
    //     ctx,
    //     LifeCycleEvent::Composing,
    //     Thunk::new(Box::new(move |ctx, state| {
    //         state.add(ctx, vec![Pc::None]);
    //         state.pad(ctx, 1);
    //     })),
    // );

    state.add(ctx, vec![Pc::None]);
    state.call_thunks(ctx, ctx, LifeCycleEvent::Composition);
    state.pad(ctx);

    // eprint!("{}", color!());

    // dbg!();
    // print_state(state, ctx);
    // pause();

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn ctx_pc_count(ctx: Ctx, state: &State) -> usize {
    if state.children(ctx).is_empty() {
        state.pcs(ctx).unwrap_or_default().len()
    } else {
        state
            .children(ctx)
            .iter()
            .cloned()
            .map(|ctx| ctx_pc_count(ctx, state))
            .sum()
    }
}

fn sequence(state: &mut State, ctxs: Vec<Ctx>) {
    // trace(TextStyle::BoldCyan);
    // dbg!(&ctxs);
    if ctxs.is_empty() {
        return;
    }
    // dbg!();

    let parent = state.parent(ctxs.first().cloned().unwrap_or_default());

    // eprintln!("{:?} {:?}", parent, state.scope(parent));
    // eprint!("{}", color!());

    // c!{{
    //     print_state(state, parent);
    //     state
    //     .children(parent)
    //     .iter()
    //     .cloned()
    //     .for_each(|ctx| print_state(state, ctx));
    //     pause();
    // }}

    let parent_scope = state.scope(parent);

    match state.scope(parent) {
        Scope::None => {
            ctxs.iter().for_each(|ctx| {
                // print_state(state, *ctx);

                // pause();

                // state.combine_sequences(vec![*ctx]);
                sequence(state, state.children(*ctx));
                state.call_thunks(*ctx, *ctx, LifeCycleEvent::PostSequence);
            });
        }
        Scope::Sequence => ctxs.iter().cloned().for_each(|ctx| {
            // eprintln!("{Cyan}{:?} {:?}", state.scope(parent), state.scope(ctx));
            // print_state(state, ctx);
            let children = state.children(ctx);
            // dbg!();
            let scope = state.scope(ctx);
            // eprintln!("{:?} {:?} -> {:?} {:?}", parent, parent_scope, ctx, scope);
            // pause();
            match scope {
                Scope::Sequence => {
                    // dbg!();
                    if children.is_empty() {
                        // dbg!();
                        // pause();

                        // state.hoist(ctx, parent);
                        // state.call_thunks(parent, ctx, LifeCycleEvent::PreSequence);
                        state.combine_sequences(vec![ctx]);
                        state.drop(ctx);
                    } else {
                        sequence(state, children.clone());

                        children.iter().cloned().for_each(|ctx_| {
                            // sequence(state, vec![ctx_]);
                            // state.call_thunks(ctx, ctx_, LifeCycleEvent::PostSequence);
                            state.combine_sequences(vec![ctx_]);
                            state.drop(ctx_);
                        });
                        // state.call_thunks(parent, ctx, LifeCycleEvent::PostSequence);
                    }
                }
                Scope::Stack => {
                    // dbg!();
                    if children.is_empty() {
                        // dbg!();
                        // pause();
                        state.call_thunks(parent, ctx, LifeCycleEvent::PreSequence);
                        // state.combine_sequences(vec![ctx]);
                        // state.drop(ctx);
                    } else {
                        // dbg!();
                        // print_state(state, ctx);
                        // state
                        //     .children(parent)
                        //     .iter()
                        //     .cloned()
                        //     .for_each(|ctx| print_state(state, ctx));
                        // // pause();

                        // children.iter().cloned().for_each(|ctx| {
                        //     sequence(state, vec![ctx]);
                        // });
                        // dbg!();
                        // print_state(state, ctx);
                        // state
                        //     .children(parent)
                        //     .iter()
                        //     .cloned()
                        //     .for_each(|ctx| print_state(state, ctx));
                        // pause();

                        sequence(state, children.clone());

                        state.combine_sequences(children.clone());
                        // dbg!();
                        // print_state(state, parent);
                        // state
                        //     .children(parent)
                        //     .iter()
                        //     .cloned()
                        //     .for_each(|ctx| print_state(state, ctx));
                        // print_timeline(state);
                        // pause();
                        children.iter().cloned().for_each(|ctx_| {
                            state.drop(ctx_);
                        });
                        state.call_thunks(parent, ctx, LifeCycleEvent::PostSequence);

                        // dbg!();
                        // trace(TextStyle::IntensePurple);
                    }
                }
                _ => (),
            }
        }),
        Scope::Stack => {
            // sequence(state, state.children(ctx));
            // dbg!();
            // ctxs.iter().cloned().for_each(|ctx| {
            //     print_state(state, ctx);
            // });
            // pause();
            ctxs.iter().cloned().for_each(|ctx| {
                sequence(state, state.children(ctx));
            });
            state.combine_sequences(ctxs.clone());
            ctxs.iter().cloned().for_each(|ctx| {
                c! {{
                    eprintln!("{ctx:?}");
                }}
                state.call_thunks(parent, ctx, LifeCycleEvent::PostSequence);
                c! {{
                    print_state(state, ctx);
                }}
            });
            state.call_thunks(state.parent(parent), parent, LifeCycleEvent::PostSequence);
            ctxs.iter().cloned().for_each(|ctx| {
                state.drop(ctx);
            });

            // eprint!("{}", color!());
            // dbg!();
            // print_state(state, parent);
            // ctxs.iter().cloned().for_each(|ctx| {
            //     print_state(state, ctx);
            // });

            // pause();
        }
        _ => (),
    }
    // dbg!();
    // c! {{
    //         print_state(state, parent);
    //         state
    //             .children(parent)
    //             .iter()
    //             .cloned()
    //             .for_each(|ctx| print_state(state, ctx));
    //             pause();
    // }}
}

fn pre_comp(thunk: Thunk) -> Thunk {
    Thunk::new(Box::new(move |ctx_, state| {
        let parent = state.parent(ctx_);
        // state.call_thunks(parent, ctx_, LifeCycleEvent::Composition);
        state.add_thunk(ctx_, LifeCycleEvent::Composition, thunk.clone());
        state.add_thunk(
            ctx_,
            LifeCycleEvent::PreComposition,
            pre_comp(thunk.clone()),
        );
    }))
}
