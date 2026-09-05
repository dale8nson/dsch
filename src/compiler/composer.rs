#![allow(unused, const_item_mutation)]
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
    iter::{Iterator, Map, from_fn, repeat_n},
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
    os::unix::process::ExitStatusExt,
    path::absolute,
    process::exit,
    rc::Rc,
    vec::{self, IntoIter},
};

use crossterm::{cursor::*, execute, style::*, terminal::*};
use num_rational::Ratio;
use num_traits::FromPrimitive;

use crate::{
    color,
    compiler::{
        ast::*,
        codegen::{
            Scope,
            state::State,
            utils::{ColorIter as Color, TextStyle::*, *},
            *,
        },
        error,
        functional::*,
    },
    // thunk,
};

const QUARTER_NOTE_RATIO: f64 = 4.0;

pub fn compose_program(mut program: Program) -> State {
    let mut state = State::default();
    let ctx = Ctx::None;

    compose(
        ctx,
        vec![Exp::Compound(Compound::Parens(program.exps))],
        &mut state,
    );

    print_states(&state, ctx);
    // pause();

    let children = state.children(Ctx::Root);

    sequence(&mut state, children);

    eprintln!(
        "{}",
        format!("{:?}", state.timeline())
            .split_inclusive("): ")
            .collect::<Vec<_>>()
            .join("\n  ")
            .split_inclusive("Slice { ")
            .collect::<Vec<_>>()
            .join("\n    ")
            .split_inclusive("}, ")
            .collect::<Vec<_>>()
            .join("\n  ")
            .split_inclusive("}], ")
            .collect::<Vec<_>>()
            .join("\n")
            .split_inclusive("], ")
            .collect::<Vec<_>>()
            .join("\n    ")
    );
    // pause();

    state
}

fn compose(ctx: Ctx, mut exps: Vec<Exp>, state: &mut State) -> Monad<(Exp, Ctx)> {
    info(format!(
        "{}COMPOSE {ctx:?} {:?}",
        color!(),
        state.scope(ctx)
    ));
    dbg!();
    misc(format!(
        "{}{}",
        color!(),
        exps.iter()
            .map(|exp| format!("{exp}"))
            .collect::<Vec<String>>()
            .join("\n")
    ));

    // pause();

    let exps = if exps.len() > 1 {
        exps.into_iter().fold(Vec::<Exp>::new(), |mut exps, rhs| {
            if let Some(lhs) = exps.pop() {
                match (lhs, rhs) {
                    (Exp::Simple(Simple::Prefix(prefix)), rhs @ Exp::Simple(Simple::Scalar(_))) => {
                        state.push_back(rhs);
                        let mut exp = Exp::Noop;
                        compose_prefix(prefix, ctx, state).bind(|(exp_, _)| {
                            exp = exp_.clone();
                            Monad::ret((exp_, ctx))
                        });
                        if !matches!(exp, Exp::Noop) {
                            exps.push(exp);
                        }
                    }
                    (lhs, rhs) => {
                        exps.extend(vec![lhs, rhs]);
                    }
                }
            } else {
                exps.push(rhs);
            }
            exps
        })
    } else {
        exps
    };

    misc(format!(
        "{}{}",
        color!(),
        exps.iter()
            .map(|exp| format!("{exp}"))
            .collect::<Vec<String>>()
            .join("\n")
    ));

    // pause();
    let (exps_, index_) = state.set_exps(exps, 0);

    let mut stack = state.set_stack(Vec::new());
    // eprint!("{}", color!());
    // graph(state, Ctx::Root, 10);

    dbg!();
    print_stacks(state);
    // pause();
    let mut m = Monad::ret((state.next().unwrap_or_default(), ctx));
    // trace(TextStyle::BoldYellow);

    // dbg!(&m);
    // pause();

    loop {
        let mut should_break = false;
        m = m.bind(Box::new(|(exp, ctx_): (Exp, Ctx)| {
            misc(format!(
                "{}EXP: {exp}\n{}",
                color!(),
                state_str(state, ctx_)
            ));

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

    print_stacks(state);
    // print_state(state, ctx);
    // pause();

    Monad::ret((state.next().unwrap_or_default(), ctx))
    // Monad::ret((state.next().unwrap_or_default(), state.parent(ctx)))
}

fn compose_exp(exp: Exp, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    info(format!(
        "{}\nCOMPOSE EXP {exp} {ctx:?} {:?}",
        color!(),
        state.scope(ctx)
    ));

    // pause();

    // graph(state, Ctx::Root, 10);
    print_state(state, ctx);
    // pause();

    // print_stacks(state);

    let next = state.next().unwrap_or_default();

    log(format!("NEXT: {next}\n"));
    // eprint!("{}", color!());

    // pause();

    match next {
        next @ Exp::Simple(Simple::Infix(_)) | next @ Exp::Simple(Simple::Suffix(_)) => {
            // dbg!();
            // print_stacks(state);
            // dbg!(&exp);
            // pause();
            if matches!(exp, Exp::Simple(Simple::Ident(_))) {
                state.push_back(next);
            } else {
                state.store(exp);
                // dbg!();
                // print_stacks(state);
                // pause();
                return Monad::ret((next, ctx));
            }
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
        Exp::EOI => {
            // state.push_back(next);
            Monad::ret((Exp::EOI, ctx))
        }
        Exp::Simple(Simple::Ident(ident)) => {
            // state.push_back(next);
            let binding = state.binding(ctx, &ident).unwrap_or_default();

            Monad::ret((binding, ctx))
        }
        // Exp::Simple(Simple::Infix(_)) | Exp::Simple(Simple::Suffix(_)) => {
        //     state.store(exp);
        //     Monad::ret((state.next().unwrap_or_default(), ctx))
        // }
        Exp::Simple(simple) => {
            // state.store(exp);
            // Monad::ret((next, ctx))
            compose_simple(simple, ctx, state)
        }
        Exp::Compound(compound) => {
            // state.push_back(next);
            compose_compound(compound, ctx, state)
        }
        Exp::Decl(decl) => compose_decl(decl, ctx, state),
    }
}

fn compose_compound(compound: Compound, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    let ctx_ = state.append_child(ctx);
    state.set_scope(ctx_, compound.scope());

    let mut exps = compound.to_vecdeque();
    // let mut m = Monad::ret((exps.pop_front().unwrap_or_default(), ctx_));

    dbg!();
    print_state(state, ctx_);
    // pause();

    compose_thunks(ctx, ctx_, state);

    dbg!();
    print_state(state, ctx_);
    // pause();

    status(format!(
        "{}COMPOSE COMPOUND {ctx_:?} {:?}",
        color!(),
        state.scope(ctx_)
    ));

    // graph(state, Ctx::Root, 10);

    // eprint!("{}", color!());

    // dbg!(&m);
    dbg!();
    print_state(state, ctx_);

    // pause();

    let mut m = compose(ctx_, exps.into(), state);

    // dbg!(&m);

    // pause();

    dbg!();
    print_state(state, ctx_);

    // pause();

    execute!(stderr(), Clear(ClearType::FromCursorDown));

    misc(format!(
        "{}{}\n{}{}",
        color!(),
        state_str(state, Ctx::Root),
        color!(),
        state_str(state, ctx_)
    ));
    // pause();

    // dbg!(&m);

    log(format!(
        "{}{ctx:?}, {:?}, {ctx_:?}, {:?}",
        color!(),
        state.scope(ctx),
        state.scope(ctx_)
    ));

    // status(format!("{ctx:?} CHIL: {:?}", state.children(ctx_)));

    dbg!();
    print_state(state, ctx_);
    // pause();
    //

    if matches!(ctx_, Ctx::Id(1..)) {
        let children = state.children(ctx_);
        if children.is_empty() {
            compose_thunks(ctx_, ctx_, state);
            state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
            match (state.scope(ctx), state.scope(ctx_)) {
                (Scope::Sequence, Scope::Sequence | Scope::Stack)
                | (Scope::Stack, Scope::Stack) => {
                    state.hoist(ctx_, ctx);
                    state.drop(ctx_);
                }
                (Scope::Stack, Scope::Sequence) => (),
                _ => (),
            }
        } else {
            children.iter().cloned().for_each(|ctx| {
                compose_thunks(ctx, ctx, state);
                state.call_thunks(ctx, LifeCycleEvent::Sequencing);
            });

            compose_thunks(ctx_, ctx_, state);
            state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
        }
    }

    // match (ctx, state.scope(ctx), ctx_, state.scope(ctx_)) {
    //     (Ctx::Root | Ctx::Id(1..), Scope::Sequence, _, Scope::Sequence) => {
    //         if state.children(ctx_).len() > 0 {
    //             state.children(ctx_).iter().cloned().for_each(|ctx| {
    //                 compose_thunks(ctx, ctx, state);
    //                 state.call_thunks(ctx, LifeCycleEvent::Sequencing);
    //                 // state.combine_sequences(vec![ctx]);
    //                 // state.hoist(ctx, ctx_);
    //                 // state.drop(ctx);
    //                 print_states(state, ctx);
    //             });
    //         } else {
    //             compose_thunks(ctx_, ctx_, state);
    //             state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //             // state.hoist(ctx_, ctx);
    //             // state.drop(ctx_);
    //             // state.combine_sequences(vec![ctx_]);}
    //         }
    //     }
    //     (Ctx::Root | Ctx::Id(1..), Scope::Sequence, _, Scope::Stack) => {
    //         // state.clean_context(ctx_);

    //         if state.children(ctx_).len() > 0 {
    //             // state.set_scope(ctx_, Scope::Sequence);
    //             state.children(ctx_).iter().cloned().for_each(|ctx| {
    //                 compose_thunks(ctx, ctx, state);
    //                 state.call_thunks(ctx, LifeCycleEvent::Sequencing);
    //             });
    //             // compose_thunks(ctx_, ctx_, state);
    //             // state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //             // state.combine_sequences(state.children(ctx_));
    //             // state.children(ctx_).iter().cloned().for_each(|ctx| state.drop(ctx));
    //             // } else
    //         } else {
    //             compose_thunks(ctx_, ctx_, state);
    //             state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //             // state.hoist(ctx_, ctx);
    //             // state.drop(ctx_);
    //         }

    //         // }
    //         // else {
    //         //     compose_thunks(ctx_, ctx_, state);

    //         //     // state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //         //     // state.hoist(ctx_, ctx);
    //         //     // state.drop(ctx_);
    //         // }
    //         // state.combine_sequences(vec![ctx_]);
    //         // state.hoist(ctx_, ctx);
    //         // state.drop(ctx_);
    //         // }
    //         print_states(state, ctx);
    //         m = m.bind(Box::new(|(exp, ctx_)| Monad::ret((exp, ctx))));
    //         // pause();
    //     }
    //     (Ctx::Root | Ctx::Id(1..), Scope::Stack, _, Scope::Stack) => {
    //         let children = state.children(ctx_);
    //         // if children.is_empty() {
    //             compose_thunks(ctx_, ctx_, state);
    //             state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //         // } else {
    //             children.iter().cloned().for_each(|ctx| {
    //                 compose_thunks(ctx, ctx, state);
    //                 state.call_thunks(ctx, LifeCycleEvent::Sequencing);
    //             });
    //         // }

    //         // state.hoist(ctx_, ctx);
    //         // state.drop(ctx_);
    //     }
    //     (_, Scope::Stack, _, Scope::Sequence) => {
    //         // state.combine_sequences(vec![ctx_]);

    //         state.children(ctx_).iter().cloned().for_each(|ctx| {
    //             compose_thunks(ctx, ctx, state);
    //             state.call_thunks(ctx, LifeCycleEvent::Sequencing);
    //             // state.hoist(ctx, ctx_);
    //             // state.combine_sequences(vec![ctx]);
    //             // state.drop(ctx);
    //             // print_states(state, ctx);
    //         });
    //         compose_thunks(ctx_, ctx_, state);
    //         state.call_thunks(ctx, LifeCycleEvent::Sequencing);
    //         // state.call_thunks(ctx_, LifeCycleEvent::Sequencing);

    //         print_states(state, ctx);
    //     }
    //     (_, _, Ctx::Root, Scope::Sequence) => {
    //         state.children(ctx_).iter().cloned().for_each(|ctx| {
    //             compose_thunks(ctx, ctx, state);
    //             // misc(state_str(state, ctx_));
    //             print_states(state, ctx);
    //             // pause();
    //             // state.combine_sequences(vec![ctx]);
    //         });
    //         compose_thunks(ctx_, ctx_, state);
    //         state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //         //

    //         // print_state(state, ctx);
    //         // pause();

    //         print_state(state, ctx);
    //         // pause();

    //         // state.combine_sequences(vec![ctx_]);
    //         // state.hoist(ctx_, ctx);
    //         // state.drop(ctx_);

    //         print_states(state, ctx);
    //         // pause();
    //         // print_states(state, state.parent(ctx));
    //     }
    //     other => {
    //         dbg!(&other);
    //         // pause();
    //         compose_thunks(ctx_, ctx_, state);
    //         state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //         state.children(ctx_).iter().cloned().for_each(|ctx| {
    //             // state.combine_sequences(vec![ctx]);
    //             // });
    //             // } else {
    //             // state.call_thunks(ctx_, LifeCycleEvent::Sequencing);
    //             // state.combine_sequences(vec![ctx_]);
    //             // state.hoist(ctx_, ctx);
    //             // state.drop(ctx_);
    //             // }
    //             print_states(state, ctx);
    //         });
    //         // state.combine_sequences(vec![ctx_]);
    //         // state.hoist(ctx_, ctx);
    //         // state.drop(ctx_);
    //         print_states(state, ctx);
    //     }
    // }

    // eprintln!("{}{:?}", color!(), state.timeline());

    // pause();

    m.bind(Box::new(|(exp, _)| Monad::ret((exp, ctx))))

    // Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_simple(simple: Simple, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    status(format!("{}COMPOSE SIMPLE {simple:?} {ctx:?}", color!()));
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
    match decl {
        Decl::ImportDecl(import_decl) => todo!(),
        Decl::ExpDecl(exp_decl) => compose_expdecl(exp_decl, ctx, state),
        Decl::FuncDecl(func_decl) => todo!(),
    }
}

fn compose_expdecl(expdecl: ExpDecl, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    eprintln!("{}COMPOSE EXPDECL {expdecl:?} {ctx:?}", color!());
    let ExpDecl { ident, binding } = expdecl;
    state.add_binding(ctx, ident, *binding);
    // dbg!(ctx, state.bindings(ctx));
    // pause();
    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_prefix(prefix: Prefix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    info(format!("{}COMPOSE PREFIX {prefix:?} {ctx:?}", color!()));
    let rhs = state.next().unwrap_or_default();
    // trace(IntenseBoldGreen);
    dbg!(&rhs);
    // pause();
    match (prefix, rhs.clone()) {
        // (prefix, rhs @ Exp::Compound(_)) => {
        //     state.store(Exp::Simple(Simple::Prefix(prefix)));
        //     Monad::ret((rhs, ctx))
        // }
        (Prefix::Dur, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(abs))))) => {
            // let exp = Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
            //     Fractional::Absolute(abs),
            // ))));

            let duration = Duration::Fractional(Fractional::Absolute(abs));

            // compose_duration(duration, ctx, state)

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
            // compose_register(Register::Reg(abs.as_u64() as i8), ctx, state)
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
                Scope::Set => todo!(),
                Scope::None => todo!(),
            };

            Monad::ret((compound.to_exp(), ctx))
        }
        (Prefix::Reg, Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Relative(rel))))) => {
            compose_relative::<Register>(rel, ctx, state)
        }
        (Prefix::Prog, Exp::Simple(simple)) => todo!(),
        (_, Exp::Noop) => todo!(),
        (_, Exp::EOI) => todo!(),
        _ => {
            dbg!(&prefix, &rhs);
            todo!()
        }
    }
}

fn compose_prog(prog: Prog, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // eprintln!("{}COMPOSE PROG {prog:?} {ctx:?}", color!());

    state.add_thunk(
        ctx,
        LifeCycleEvent::Composing,
        Thunk::new(Box::new(move |ctx, state| {
            state.add(ctx, vec![prog]);
        })),
    );

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_infix(infix: Infix, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    eprintln!("{}COMPOSE INFIX {ctx:?} {infix:?}", color!());
    match infix {
        Infix::Colon => todo!(),
        Infix::Intercalate => todo!(),
        Infix::Range => todo!(),
        Infix::Interpolation(interpolation) => compose_interpolation(interpolation, ctx, state),
        Infix::Plus => todo!(),
        Infix::Minus => {
            todo!()
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
                                state.pad(ctx, 1);
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
                    // graph(state, state.parent(ctx), 4);
                    // print_state(state, ctx);
                    todo!()
                }
            }
        }
        Infix::Div => todo!(),
    }
}

fn compose_interpolation(
    interpolation: Interpolation,
    ctx: Ctx,
    state: &mut State,
) -> Monad<(Exp, Ctx)> {
    eprintln!(
        "{}COMPOSE INTERPOLATION {ctx:?} {interpolation:?}",
        color!()
    );
    print_stacks(state);
    // pause();
    // eprintln!("{}", std::backtrace::Backtrace::capture().to_string());
    // dbg!();
    // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));
    let lhs = state.load().unwrap_or_default();
    dbg!(&lhs);
    // pause();
    let mut data: Vec<Vec<Data>> =
        if let ref exp @ Exp::Simple(Simple::Scalar(ref lhs)) = lhs.clone() {
            let rhs = state.next().unwrap_or_default();
            // dbg!(&rhs);
            let mut data = Vec::<Vec<Data>>::new();
            match rhs.clone() {
                Exp::Simple(Simple::Scalar(rhs)) => {
                    eprintln!("{}{lhs:?} {interpolation} {rhs:?}", color!());
                    // pause();
                    match (lhs.clone(), rhs) {
                        (Scalar::Duration(lhs_duration), Scalar::Duration(rhs_duration)) => {
                            // dbg!(&lhs_duration, &rhs_duration);
                            let lhs_length = duration_to_length(lhs_duration, ctx, state);
                            let rhs_length = duration_to_length(rhs_duration, ctx, state);
                            // state.add_length(ctx, lhs_length);
                            // graph(state, state.parent(ctx), 3);
                            // print_state(state, ctx);

                            data.push(vec![lhs_length.into(), rhs_length.into()]);
                        }
                        (Scalar::Register(lhs), Scalar::Register(rhs)) => {
                            data.push(vec![lhs.into(), rhs.into()]);
                        }
                        (Scalar::Frequency(lhs), Scalar::Frequency(rhs)) => todo!(),
                        (Scalar::Pure(lhs), Scalar::Pure(rhs)) => todo!(),
                        (Scalar::Dynamic(lhs), Scalar::Dynamic(rhs)) => {
                            let lhs_velocity = Velocity::from(lhs);
                            let rhs_velocity = Velocity::from(rhs);
                            // graph(state, state.parent(ctx), 5);
                            // eprintln!("{}{}{ResetColor}", color!(), state_str(state, ctx));
                            data.push(vec![lhs_velocity.into(), rhs_velocity.into()]);
                        }
                        (Scalar::Tuplet(lhs), Scalar::Tuplet(rhs)) => todo!(),
                        (Scalar::Prog(lhs), Scalar::Prog(rhs)) => todo!(),
                        (Scalar::Rest, Scalar::Rest) => todo!(),
                        _ => todo!(),
                    }
                }
                _ => {
                    state.push_back(rhs);
                }
            }
            let mut next = state.next().unwrap_or_default();
            // dbg!(&next);
            while matches!(next, Exp::Simple(Simple::Infix(Infix::Interpolation(_)))) {
                if let Exp::Simple(Simple::Infix(Infix::Interpolation(lerp))) = next.clone() {
                    let rhs = state.next().unwrap_or_default();
                    // dbg!(&rhs);
                    if let Exp::Simple(Simple::Scalar(scalar)) = rhs.clone() {
                        let t: Data = match scalar {
                            Scalar::Duration(duration) => duration.into(),
                            Scalar::Frequency(absolute) => todo!(),
                            Scalar::Pure(pure) => match pure {
                                Pure::Relative(relative) => todo!(),
                                Pure::Absolute(absolute) => Pc::from(absolute).into(),
                                Pure::Rational(rational) => todo!(),
                            },
                            Scalar::Dynamic(dynamic) => Velocity::from(dynamic).into(),
                            Scalar::Tuplet(tuplet) => todo!(),
                            Scalar::Prog(prog) => prog.into(),
                            Scalar::Rest => todo!(),
                            Scalar::Register(register) => todo!(),
                        };
                        let last = data
                            .last()
                            .cloned()
                            .unwrap_or_default()
                            .last()
                            .cloned()
                            .unwrap_or_default();
                        data.push(vec![last, t])
                    } else {
                        state.push_back(rhs.clone());
                    }
                }
                *&mut next = state.next().unwrap_or_default();
            }
            state.push_back(next);
            data
        } else {
            vec![]
        };

    // dbg!(&data);

    let thunk = match data
        .first()
        .cloned()
        .unwrap_or_default()
        .first()
        .cloned()
        .unwrap_or_default()
    {
        Data::Pc(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Pc>(ctx, data.clone(), state)
        })),
        Data::Length(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Length>(ctx, data.clone(), state)
        })),
        Data::Velocity(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Velocity>(ctx, data.clone(), state)
        })),
        Data::Tempo(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Tempo>(ctx, data.clone(), state)
        })),
        Data::Register(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Register>(ctx, data.clone(), state)
        })),
        Data::Interpolation(interpolation) => todo!(),
        Data::Program(_) => Thunk::new(Box::new(move |ctx, state| {
            interpolate::<Prog>(ctx, data.clone(), state)
        })),
        Data::None => todo!(),
    };
    eprintln!("{}{:#?}", color!(), thunk);
    // pause();
    state.push_thunk(LifeCycleEvent::Sequencing, thunk);

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn interpolate<T: Interpolant + ToString>(ctx: Ctx, data: Vec<Vec<Data>>, state: &mut State)
where
    Data: From<T>,
{
    // eprintln!("{}INTERPOLATE {ctx:?}", color!());
    // data.iter().for_each(|data| eprintln!("{data:?}"));

    // print_state(state, ctx);
    misc(state_str(state, ctx));
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
    print_states(state, Ctx::Root);
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
        if let Some(ts_) = T::get_vec_mut(state, ctx) {
            // eprintln!("{}{ctx:?} TS_: {ts_:?}", color!());
            ts_.iter_mut().for_each(|ts| {
                let t_ = data_iter.next().unwrap_or_default();
                ts.iter_mut().for_each(|t| {
                    *t = t_.clone();
                });
            });
        }
    });

    print_states(
        state,
        state.parent(leaves.first().cloned().unwrap_or_default()),
    );
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
    eprintln!(
        "{}COMPOSE SUFFIX {suffix:?} {ctx:?} {:?}",
        color!(),
        state.scope(ctx)
    );
    // pause();
    let lhs = state.load().unwrap_or_default();
    if let Exp::Simple(Simple::Scalar(Scalar::Pure(Pure::Absolute(lhs)))) = lhs {
        // eprintln!("{}{lhs:?}", color!());
        match suffix {
            Suffix::Amp => state.add::<Velocity>(ctx, vec![lhs.into()]),
            Suffix::Bpm => {
                state.set_bpm(lhs);
                state.add(
                    ctx,
                    vec![Mpb(
                        Ratio::<u64>::from_f64(60_000_000. / lhs.as_f64()).unwrap()
                    )],
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
    info(format!("{}COMPOSE SCALAR {ctx:?} {scalar:?}", color!()));

    // let next = state.next().unwrap_or_default();
    // if let Exp::Compound(compound) = next {
    //     let scope = compound.scope();
    //     let exps = compound.to_vecdeque().into_iter().flat_map(|exp| {
    //         if matches!(exp, Exp::Simple(Simple::Scalar(Scalar::Pure(_)))) {
    //             vec![Exp::Simple(Simple::Scalar(scalar.clone())), exp]
    //         } else {
    //             vec![exp]
    //         }
    //     }).collect::<Vec<_>>();
    //     let compound = match scope {
    //         Scope::Sequence => Compound::Parens(exps),
    //         Scope::Stack => Compound::Brackets(exps),
    //         Scope::Set => todo!(),
    //         Scope::None => todo!(),
    //     };

    //      return Monad::ret((Exp::Compound(compound), ctx));

    // } else {
    //     state.push_back(next);
    // }

    match scalar {
        Scalar::Duration(duration) => compose_duration(duration, ctx, state),
        Scalar::Frequency(absolute) => compose_absolute(absolute, ctx, state),
        Scalar::Pure(pure) => compose_pure(pure, ctx, state),
        Scalar::Dynamic(dynamic) => {
            state.add::<Velocity>(ctx, vec![dynamic.into()]);
            Monad::ret((state.next().unwrap_or_default(), ctx))
        }
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
    }
}

fn compose_absolute(absolute: Absolute, mut ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    log(format!(
        "{}COMPOSE ABSOLUTE {absolute:?} {ctx:?} {:?}",
        color!(),
        state.scope(ctx)
    ));

    // compose_thunks(ctx, ctx, state);

    // state.add_thunk(ctx, LifeCycleEvent::Composing, Thunk::new(Box::new(move |ctx, state| {
    match absolute {
        Absolute::UInt(uint) => {
            state.add(ctx, vec![Pc::Class(uint as f64)]);
        }
        Absolute::Float(float) => {
            state.add(ctx, vec![Pc::Class(float)]);
        }
    }
    compose_thunks(ctx, ctx, state);
    state.pad(ctx, 1);
    // })));

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
            dbg!(&next);
            // pause();
            state.push_back(next);
            match pure {
                Pure::Relative(relative) => compose_relative::<Pc>(relative, ctx, state),
                Pure::Absolute(absolute) => compose_absolute(absolute, ctx, state),
                Pure::Rational(rational) => todo!(),
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
    let t: T = state.get(ctx).iter().cloned().min().unwrap();
    // dbg!(&relative);

    let Relative { sign, val } = relative;
    // dbg!(&sign, val, &t);
    let t_: T = match sign {
        Sign::Plus => t + T::from(val),
        Sign::Minus => t - T::from(val),
    };

    // dbg!(&t_);

    state.add_thunk(
        ctx,
        LifeCycleEvent::Composing,
        Thunk::new(Box::new(move |ctx, state| {
            state.add(ctx, vec![t_.clone()]);
        })),
    );

    Monad::ret((state.next().unwrap_or_default(), ctx))
}

fn compose_dynamic(dynamic: Dynamic, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    info(format!(
        "{}COMPOSE DYNAMIC {ctx:?} {}",
        color!(),
        dynamic.0.as_str()
    ));

    let next = state.next().unwrap_or_default();
    match next {
        Exp::Compound(compound) => {
            // let exps = compound.to_vecdeque();
            // let exps: Vec<Exp> = exps
            //     .into_iter()
            //     .flat_map(|exp| {
            //         if matches!(exp, Exp::Simple(Simple::Scalar(Scalar::Pure(_)))) {
            //             vec![
            //                 Exp::Simple(Simple::Scalar(Scalar::Dynamic(dynamic.clone()))),
            //                 exp,
            //             ]
            //         } else {
            //             vec![exp]
            //         }
            //     })
            //     .collect();

            let velocity: u8 = match dynamic.0.as_str() {
                "ppp" => 20,
                "pp" => 39,
                "p" => 57,
                "mp" => 71,
                "mf" => 84,
                "f" => 98,
                "ff" => 113,
                "fff" => 127,
                _ => 80,
            };

            state.add_thunk(
                ctx,
                LifeCycleEvent::Composing,
                Thunk::new(Box::new(move |ctx, state| {
                    state.add(ctx, vec![Velocity(velocity)]);
                })),
            );
        }
        Exp::Simple(simple) => {
            let velocity: u8 = match dynamic.0.as_str() {
                "ppp" => 20,
                "pp" => 39,
                "p" => 57,
                "mp" => 71,
                "mf" => 84,
                "f" => 98,
                "ff" => 113,
                "fff" => 127,
                _ => 80,
            };

            state.add_thunk(
                ctx,
                LifeCycleEvent::Composing,
                Thunk::new(Box::new(move |ctx, state| {
                    state.add(ctx, vec![Velocity(velocity)]);
                })),
            );
        }
        Exp::Noop => todo!(),
        Exp::EOI => todo!(),
        Exp::Decl(decl) => todo!(),
    }

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
    info(format!(
        "{}COMPOSE FIXED DURATION {fixed:?} {ctx:?}",
        color!()
    ));

    state.add_thunk(
        ctx,
        LifeCycleEvent::Composing,
        Thunk::new(Box::new(move |ctx, state| {
            state.add::<Length>(ctx, vec![fixed.clone().into()]);
        })),
    );

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
            let mut microseconds = state.tempo().0
                / Ratio::<u64>::from_f64(absolute.as_f64() / QUARTER_NOTE_RATIO as f64).unwrap();

            state.add_thunk(
                ctx,
                LifeCycleEvent::Composing,
                Thunk::new(Box::new(move |ctx_, mut state| {
                    // if ctx_ != ctx {
                    //     let len = state.pcs(ctx).unwrap_or_default().len() - state.lengths(ctx).unwrap_or_default().len();
                    //     state.add(ctx_, repeat_n(Length::MicroSeconds(microseconds), len).collect());
                    // } else
                    // {
                    state.add(ctx_, vec![Length::MicroSeconds(microseconds)]);
                    // }
                })),
            );

            Monad::ret((next, ctx))
        }
        Fractional::Tuplet(tuplet) => {
            let Tuplet { lhs, rhs } = tuplet;
            let microseconds = Ratio::<u64>::from_f64(
                rhs.as_f64() / lhs.as_f64()
                    * state.get::<Length>(ctx).iter().cloned().min().unwrap(),
            )
            .unwrap();

            state.add_thunk(
                ctx,
                LifeCycleEvent::Composing,
                Thunk::new(Box::new(move |ctx, state| {
                    state
                        .get_last_mut(ctx)
                        .and_then(|length| {
                            std::mem::swap(length, &mut Length::MicroSeconds(microseconds));
                            Some(())
                        })
                        .or_else(|| {
                            state.add(ctx, vec![Length::MicroSeconds(microseconds)]);
                            Some(())
                        });
                })),
            );

            Monad::ret((next, ctx))
        }
        Fractional::Rational(rational) => {
            let Rational { num, den } = rational;
            let microseconds = Ratio::<u64>::from_f64(
                num.as_f64() / den.as_f64()
                    * QUARTER_NOTE_RATIO
                    * state.get::<Tempo>(ctx).iter().cloned().min().unwrap(),
            )
            .unwrap();

            state.add_thunk(
                ctx,
                LifeCycleEvent::Composing,
                Thunk::new(Box::new(move |ctx, state| {
                    state.add(ctx, vec![Length::MicroSeconds(microseconds)]);
                })),
            );

            Monad::ret((next, ctx))
        }
    }
}

fn compose_register(reg: Register, ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    info(format!("{}COMPOSE REGISTER {reg:?} {ctx:?}", color!()));

    let next = state.next().unwrap_or_default();

    // let thunk =  Thunk::new(Box::new(|ctx| state.add(ctx, vec![reg])));

    state.add_thunk(
        ctx,
        LifeCycleEvent::Composing,
        Thunk::new(Box::new(move |ctx, state| state.add(ctx, vec![reg]))),
    );
    Monad::ret((next, ctx))
}

fn compose_rest(ctx: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    eprintln!("{}COMPOSE REST {ctx:?}", color!());
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
    // state.add(ctx, vec![Velocity(0)]);
    // let lengths = state.get::<Length>(ctx);
    // state.add(ctx, lengths);
    compose_thunks(ctx, ctx, state);

    state.pad(ctx, 1);

    print_state(state, ctx);
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
    let parent = state.parent(ctxs.first().cloned().unwrap_or_default());

    match state.scope(parent) {
        Scope::Sequence => ctxs.iter().cloned().for_each(|ctx| {
            let children = state.children(ctx);
            match state.scope(ctx) {
                Scope::Sequence => {
                    if children.is_empty() {
                        // state.hoist(ctx, parent);
                        // state.drop(ctx);
                        // state.combine_sequences(vec![ctx]);
                    } else {
                        // sequence(state, children.clone());
                        children.iter().cloned().for_each(|ctx| {
                            sequence(state, vec![ctx]);
                            state.combine_sequences(vec![ctx])
                        })
                        //     children
                        //         .iter()
                        //         .cloned()
                        //         .for_each(|ctx_| sequence(state, vec![ctx_]));
                    }
                }
                Scope::Stack => {
                    if children.is_empty() {
                        state.combine_sequences(vec![ctx]);
                        // state.hoist(ctx, parent);
                        // state.drop(ctx);
                    } else {
                        sequence(state, children.clone());
                        // sequence(state, children.clone());
                        state.combine_sequences(children);
                        // state.set_scope(ctx, Scope::Sequence);
                    }
                }
                _ => (),
            }
        }),
        Scope::Stack => {
            ctxs.iter().cloned().for_each(|ctx| {
                let children = state.children(ctx);
                match state.scope(ctx) {
                    Scope::Sequence => {
                        if children.is_empty() {
                            // state.hoist(ctx, parent);
                            // state.drop(ctx);
                        } else {
                            sequence(state, children.clone());
                            children
                                .iter()
                                .cloned()
                                .for_each(|ctx| state.combine_sequences(vec![ctx]));
                        }
                    }
                    Scope::Stack => {
                        if children.is_empty() {
                            // state.hoist(ctx, parent);
                            // state.drop(ctx);
                        } else {
                            sequence(state, children.clone());
                            state.combine_sequences(children);
                        }
                    }
                    _ => (),
                }

                // if children.is_empty() {
                //     state.combine_sequences(vec![ctx]);
                // } else {
                //     sequence(state, state.children(ctx));
                // }
            });
        }
        _ => (),
    }
    // ctxs.iter().cloned().for_each(|ctx| {
    //     let children = state.children(ctx);
    //     if children.len() > 0 {
    //         // children.iter().cloned().for_each(|ctx| sequence(state, vec![ctx]));
    //         // sequence(state, children.clone());
    //         // match (state.scope(state.parent(ctx)), state.scope(ctx)) {
    //         //     (Scope::Sequence, Scope::Sequence) => {
    //         //         children.iter().cloned().for_each(|ctx| state.combine_sequences(vec![ctx]));
    //         //     },
    //         //     (Scope::Sequence, Scope::Stack) => {
    //         //         state.combine_sequences(children);
    //         //     },
    //         //     (Scope::Stack, Scope::Sequence) => {

    //         //     }
    //         //     (Scope::Stack, Scope::Stack) => {

    //         //     }
    //         //     _ => ()
    //         // }
    //         match (state.scope(state.parent(ctx)), state.scope(ctx)) {
    //             // Scope::Sequence => {
    //             //     children.iter().cloned().for_each(|ctx_| {
    //             //         sequence(state, vec![ctx_]);
    //             //         // state.combine_sequences(vec![ctx_]);
    //             //     })
    //             // },
    //             // Scope::Stack => {
    //             //     sequence(state, children.clone());
    //             //     state.combine_sequences(children);
    //             // },
    //             // Scope::Set => todo!(),
    //             // Scope::None => todo!(),
    //             (Scope::None | Scope::Sequence, Scope::Sequence) => {
    //                 children.iter().cloned().for_each(|ctx_| {
    //                     sequence(state, vec![ctx_]);
    //                     // state.combine_sequences(vec![ctx_]);
    //                 });
    //             }
    //             (Scope::Sequence, Scope::Stack) => {
    //                 sequence(state, children.clone());
    //                 state.set_scope(ctx, Scope::Sequence);
    //                 state.combine_sequences(children);
    //             }
    //             (Scope::Stack, Scope::Sequence) => sequence(state, vec![ctx]),
    //             (Scope::Stack, Scope::Stack) => {
    //                 // sequence(state, children.clone());
    //                 // state.combine_sequences(children);
    //                 // children.iter().cloned().for_each(|ctx| state.combine_sequences(vec![ctx]));
    //                 // state.combine_sequences(vec![ctx]);
    //             }
    //             _ => (),
    //         }
    //         // children.iter().cloned().for_each(|ctx_| {
    //         //     match (state.scope(ctx), state.scope(ctx_)) {
    //         //         (Scope::Sequence, Scope::Sequence) => {
    //         //             sequence(state, vec![ctx_]);
    //         //             // state.combine_sequences(vec![ctx_]);
    //         //         },
    //         //         (Scope::Sequence, Scope::Stack) => {
    //         //             let children = state.children(ctx_);
    //         //             sequence(state, children.clone());
    //         //             state.combine_sequences(children);
    //         //             // sequence(state, ctx_);
    //         //         },
    //         //         (Scope::Stack, Scope::Sequence) => sequence(state, vec![ctx_]),
    //         //         (Scope::Stack, Scope::Stack) => sequence(state, vec![ctx_]),
    //         //         (Scope::Set, Scope::Sequence) => todo!(),
    //         //         (Scope::Set, Scope::Stack) => todo!(),
    //         //         (Scope::Set, Scope::Set) => todo!(),
    //         //         (Scope::Set, Scope::None) => todo!(),
    //         //         (Scope::None, Scope::Sequence) => todo!(),
    //         //         (Scope::None, Scope::Stack) => todo!(),
    //         //         (Scope::None, Scope::Set) => todo!(),
    //         //         (Scope::None, Scope::None) => todo!(),
    //         //         (Scope::Sequence, Scope::Set) => todo!(),
    //         //         (Scope::Sequence, Scope::None) => todo!(),
    //         //         (Scope::Stack, Scope::Set) => todo!(),
    //         //         (Scope::Stack, Scope::None) => todo!(),
    //         //     }
    //         // });
    //     } else {
    //         match state.scope(ctx) {
    //             Scope::Sequence => state.combine_sequences(vec![ctx]),
    //             Scope::Stack => state.combine_sequences(vec![ctx]),
    //             _ => todo!(),
    //         }
    //     }
    // });
}
