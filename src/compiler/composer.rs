use std::{
    iter::{Iterator, Peekable},
    slice::Iter,
    vec::IntoIter,
};

use crate::compiler::{
    ast::*,
    codegen::{utils::*, *},
    functional::*,
};

#[derive(Debug, Default)]
pub struct State {
    contexts: Vec<Ctx>,
    current_context: Ctx,
    parents: Vec<Ctx>,
    scope_types: Vec<ScopeType>,
    lengths: Vec<Length>,
    pcs: Vec<Vec<Pc>>,
    tempos: Vec<Mpb>, // in micro seconds
    bpms: Vec<Bpm>,   // quarter note beats
    registers: Vec<i8>,
    velocities: Vec<Velocity>,
    programs: Vec<Prog>,
    children: Vec<Vec<Ctx>>,
    ast: IntoIter<Exp>,
}

impl State {
    fn append_child(&mut self, parent: Ctx) -> Ctx {
        let id = self.contexts_mut().len();
        let ctx = Ctx::Id(id);

        self.contexts_mut().push(ctx);
        self.parents_mut().push(parent);
        self.children_mut().push(Vec::<Ctx>::new());
        if !matches!(parent, Ctx::None | Ctx::Root) {
            self.children_mut()[parent.to_usize()].push(ctx);
            let parent_tempo = self.get_tempo(parent);
            self.tempos_mut().push(parent_tempo);
        } else {
            self.tempos_mut()
                .push(Mpb(f64::round(1_000_000 as f64 / 120 as f64) as u64));
        }
        self.scope_types_mut().push(ScopeType::None);
        self.lengths_mut().push(Length::None);
        self.pcs_mut().push(Vec::<Pc>::new());
        self.bpms_mut().push(Bpm(Absolute::UInt(120)));
        self.registers_mut().push(4 as i8);
        self.velocities_mut().push(Velocity(63));
        self.programs_mut().push(Prog(0));
        self.current_context = ctx;
        ctx
    }

    fn current_context(&self) -> Ctx {
        self.current_context
    }

    fn get_length(&mut self, ctx: Ctx) -> Length {
        self.lengths[ctx.to_usize()]
    }

    fn parents_mut(&mut self) -> &mut Vec<Ctx> {
        &mut self.parents
    }

    fn scope_types_mut(&mut self) -> &mut Vec<ScopeType> {
        &mut self.scope_types
    }

    fn pcs_mut(&mut self) -> &mut Vec<Vec<Pc>> {
        &mut self.pcs
    }

    fn get_tempo(&self, ctx: Ctx) -> Mpb {
        self.tempos[ctx.to_usize()]
    }

    fn set_tempo(&mut self, ctx: Ctx, tempo: Mpb) {
        self.tempos[ctx.to_usize()] = tempo;
    }

    fn bpms_mut(&mut self) -> &mut Vec<Bpm> {
        &mut self.bpms
    }

    fn registers_mut(&mut self) -> &mut Vec<i8> {
        &mut self.registers
    }

    fn velocities_mut(&mut self) -> &mut Vec<Velocity> {
        &mut self.velocities
    }

    fn programs_mut(&mut self) -> &mut Vec<Prog> {
        &mut self.programs
    }

    fn children_mut(&mut self) -> &mut Vec<Vec<Ctx>> {
        &mut self.children
    }

    fn contexts_mut(&mut self) -> &mut Vec<Ctx> {
        &mut self.contexts
    }

    fn set_length(&mut self, ctx: Ctx, length: Length) {
        self.lengths[ctx.to_usize()] = length;
    }

    pub fn set_ast(&mut self, ast: IntoIter<Exp>) {
        self.ast = ast;
    }

    pub fn ast_mut(&mut self) -> &mut IntoIter<Exp> {
        &mut self.ast
    }

    pub fn set_scope_type(&mut self, ctx: Ctx, scope_type: ScopeType) {
        self.scope_types[ctx.to_usize()] = scope_type;
    }

    pub fn tempos_mut(&mut self) -> &mut Vec<Mpb> {
        &mut self.tempos
    }

    pub fn lengths_mut(&mut self) -> &mut Vec<Length> {
        &mut self.lengths
    }
}

pub fn compose_program(mut ast: Program) -> State {
    let mut state = State::default();
    let ctx = state.append_child(Ctx::Root);

    let mut exps = ast.exps.iter();
    let next = exps.next();

    let mut m = Monad(Exp::None);
    if let Some(lhs) = next {
        m = Monad(lhs.clone());
        while let Some(rhs) = exps.next() {
            let ctx = state.current_context();
            dbg!(&ctx);
            m = compose_exps(m, Monad(rhs.clone()), &mut state, ctx);
            dbg!(&m);
        }
        let ctx = state.current_context();
        m = compose_exps(m, Monad(Exp::None), &mut state, ctx);
        dbg!(&m);
    }

    dbg!(&state);
    state
}

fn compose_exps(lhs: Monad<Exp>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    dbg!(&lhs);
    dbg!(&rhs);
    dbg!(&ctx);
    lhs.bind(Box::new(|exp| match exp {
        Exp::Simple(simple) => compose_simple(Monad(simple), rhs, state, ctx),
        Exp::Compound(compound) => compose_compound(Monad(compound), rhs, state, ctx),
        Exp::None => rhs,
    }))
}

fn compose_simple(
    simple: Monad<Simple>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    simple.bind(Box::new(|simple| match simple {
        Simple::Scalar(scalar) => compose_scalar(Monad(scalar), rhs, state, ctx),
        Simple::Primitive(primitive) => compose_primitive(Monad(primitive), rhs, state, ctx),
        Simple::Op(op) => compose_op(Monad(op), rhs, state, ctx),
        Simple::Ident(ident) => compose_ident(Monad(ident), rhs, ctx),
    }))
}

fn compose_compound(
    compound: Monad<Compound>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    compound.bind(Box::new(|compound| match compound {
        Compound::Parens(exps) => {
            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Sequence;
            let m = compose_compound_exps(exps, state, ctx);
            compose_exps(m, rhs, state, ctx)
        }
        Compound::Braces(exps) => {
            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Stack;
            let m = compose_compound_exps(exps, state, ctx);
            compose_exps(m, rhs, state, ctx)
        }
        Compound::Brackets(exps) => {
            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Set;
            let m = compose_compound_exps(exps, state, ctx);
            compose_exps(m, rhs, state, ctx)
        }
        Compound::Ratio(abss) => {
            state.scope_types_mut()[ctx.to_usize()] = ScopeType::None;
            compose_ratio(Monad(abss), rhs, state, ctx)
        }
    }))
}

fn compose_scalar(
    scalar: Monad<Scalar>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    scalar.bind(Box::new(|scalar| match scalar {
        Scalar::Duration(duration) => compose_duration(Monad(duration), rhs, state, ctx),
        Scalar::Frequency(frequency) => compose_frequency(Monad(frequency), rhs, state, ctx),
        Scalar::Pure(pure) => compose_pure(Monad(pure), rhs, state, ctx),
    }))
}

fn compose_primitive(
    primitive: Monad<Primitive>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    primitive.bind(Box::new(|primitive| match primitive {
        Primitive::Prefix(prefix) => compose_prefix(Monad(prefix), rhs, state, ctx),
        Primitive::Suffix(suffix) => compose_suffix(Monad(suffix), rhs, state, ctx),
    }))
}

fn compose_op(op: Monad<Op>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_ident(ident: Monad<Ident>, rhs: Monad<Exp>, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_compound_exps(exps: Vec<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    dbg!(&exps);
    dbg!(&ctx);
    let mut exps = exps.iter();
    let mut m = Monad(Exp::None);
    while let Some(exp) = exps.next() {
        m = Monad(exp.clone()).bind(Box::new(|exp: Exp| {
            compose_exps(m, Monad(exp.clone()), state, ctx)
        }));
    }
    m
}

fn compose_ratio(
    abss: Monad<Vec<Absolute>>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    todo!()
}

fn compose_range(range: Monad<Range>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_duration(
    duration: Monad<Duration>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    dbg!(&ctx);
    duration.bind(|duration| match duration {
        Duration::Fixed(Fixed { minutes, seconds }) => {
            let length = Length::MicroSeconds(
                minutes.as_u64() * 60 * 1_000_000 + seconds.as_u64() * 1_000_000,
            );
            let ctx = state.append_child(ctx);
            state.set_length(ctx, length);
            dbg!(&ctx);
            compose_exps(Monad(Exp::None), rhs, state, ctx)
        }
        Duration::Fractional(fractional) => compose_fractional(Monad(fractional), rhs, state, ctx),
    })
}

fn compose_fractional(
    fractional: Monad<Fractional>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    fractional.bind(|fractional| match fractional {
        Fractional::Absolute(abs) => {
            let parent_length = state.get_length(ctx);
            let parent_micros = parent_length.as_u64();
            let denominator = abs.as_f64();
            let tempo = state.get_tempo(ctx);
            let duration = f64::round(denominator / 4 as f64 * tempo.0 as f64) as u64;
            let duration_micros = u64::min(parent_micros, duration);
            state.set_length(
                ctx,
                Length::MicroSeconds(u64::max(0, parent_micros - duration_micros)),
            );

            let ctx = state.append_child(ctx);
            state.set_length(ctx, Length::MicroSeconds(duration_micros));
            compose_exps(Monad(Exp::None), rhs, state, ctx)
        }
        Fractional::Tuplet(Tuplet { lhs: num, rhs: den }) => {
            let dur = Monad(Duration::Fractional(Fractional::Absolute(den / num)));
            rhs.bind(Box::new(|exp| match exp {
                Exp::Simple(simple) => todo!(),
                Exp::Compound(compound) => {
                    let ctx = state.append_child(ctx);
                    match compound {
                        Compound::Parens(exps) => {
                            state.set_scope_type(ctx, ScopeType::Sequence);
                            let mut iter = exps.iter().cloned();
                            let init = compose_duration(
                                dur.clone(),
                                Monad(iter.next().unwrap()),
                                state,
                                ctx,
                            );
                            exps.iter().fold(init, |m, rhs| {
                                m.bind(Box::new(|lhs: Exp| {
                                    compose_exps(Monad(lhs), Monad(rhs.clone()), state, ctx)
                                }))
                            })
                        }
                        compound @ Compound::Braces(_) => {
                            state.set_scope_type(ctx, ScopeType::Sequence);
                            let mut m = Monad(Exp::None);

                            for _ in 0..num.as_u64() {
                                m = dur.clone().bind(Box::new(|duration: Duration| {
                                    let ctx = state.append_child(ctx);
                                    state.set_scope_type(ctx, ScopeType::Stack);
                                    compose_duration(
                                        Monad(duration),
                                        Monad(Exp::Compound(compound.clone())),
                                        state,
                                        ctx,
                                    )
                                }));
                            }
                            m
                        }
                        Compound::Brackets(exps) => todo!(),
                        Compound::Ratio(exps) => todo!(),
                    }
                }
                Exp::None => todo!(),
            }))
        }
    })
}

fn compose_frequency(
    frequency: Monad<Absolute>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    todo!()
}

fn compose_pure(pure: Monad<Pure>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_prefix(
    prefix: Monad<Prefix>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    dbg!(&prefix);
    dbg!(&ctx);
    prefix.bind(Box::new(|prefix| match prefix {
        Prefix::Pc => rhs.bind(Box::new(|exp| {
            dbg!(&exp);
            match exp {
                Exp::Simple(simple) => match simple {
                    Simple::Scalar(scalar) => match scalar {
                        Scalar::Pure(pure) => match pure {
                            Pure::Absolute(abs) => {
                                state.pcs_mut()[ctx.to_usize()].push(Pc(abs.as_u64() as i8));
                                Monad(Exp::None)
                            }
                            Pure::Relative(Relative { sign, val }) => {
                                let last = &mut state.pcs_mut()[ctx.to_usize()].last();
                                match last.cloned() {
                                    Some(last) => match sign {
                                        Sign::Plus => state.pcs_mut()[ctx.to_usize()]
                                            .push(Pc(last.clone().0 + val.as_u64() as i8)),
                                        Sign::Minus => state.pcs_mut()[ctx.to_usize()]
                                            .push(Pc(last.clone().0 - val.as_u64() as i8)),
                                    },
                                    None => {
                                        let parent = state.parents_mut()[ctx.to_usize()].clone();
                                        let last = state.pcs_mut()[parent.to_usize()].last();
                                        match last.cloned() {
                                            Some(last) => match sign {
                                                Sign::Plus => state.pcs_mut()[ctx.to_usize()]
                                                    .push(Pc(last.clone().0 + val.as_u64() as i8)),
                                                Sign::Minus => state.pcs_mut()[ctx.to_usize()]
                                                    .push(Pc(last.clone().0 - val.as_u64() as i8)),
                                            },
                                            None => state.pcs_mut()[ctx.to_usize()]
                                                .push(Pc(val.as_u64() as i8)),
                                        }
                                    }
                                }
                                Monad(Exp::None)
                            }
                        },
                        _ => Monad(Exp::None),
                    },
                    Simple::Primitive(primitive) => match primitive {
                        prefix @ Primitive::Prefix(_) => compose_exps(
                            Monad(Exp::None),
                            Monad(Exp::Simple(Simple::Primitive(prefix))),
                            state,
                            ctx,
                        ),
                        Primitive::Suffix(suffix) => {
                            todo!()
                        }
                    },
                    Simple::Op(op) => match op {
                        Op::Colon => todo!(),
                        Op::Intercalate => todo!(),
                        Op::Range => todo!(),
                    },
                    Simple::Ident(ident) => todo!(),
                },
                Exp::Compound(compound) => {
                    todo!()
                }
                Exp::None => {
                    todo!()
                }
            }
        })),
        Prefix::Dur => {
            todo!()
        }
        Prefix::Rest => {
            todo!()
        }
        prefix @ Prefix::Reg => {
            dbg!(&state);
            rhs.bind(Box::new(|exp| match exp {
                Exp::Simple(simple) => match simple {
                    Simple::Scalar(scalar) => match scalar {
                        Scalar::Pure(pure) => match pure {
                            Pure::Absolute(abs) => {
                                state.registers_mut()[ctx.to_usize()] = abs.as_u64() as i8;
                                Monad(Exp::None)
                            }
                            Pure::Relative(relative) => todo!(),
                        },
                        Scalar::Duration(duration) => todo!(),
                        Scalar::Pure(pure) => todo!(),
                        Scalar::Frequency(frequency) => todo!(),
                    },
                    Simple::Primitive(primitive) => match primitive {
                        Primitive::Prefix(prefix) => match prefix {
                            prefix @ Prefix::Pc => compose_exps(
                                Monad(Exp::None),
                                Monad(Exp::Simple(Simple::Primitive(Primitive::Prefix(prefix)))),
                                state,
                                ctx,
                            ),
                            Prefix::Dur => todo!(),
                            Prefix::Rest => todo!(),
                            Prefix::Reg => todo!(),
                        },
                        Primitive::Suffix(suffix) => todo!(),
                    },
                    Simple::Op(op) => todo!(),
                    Simple::Ident(ident) => todo!(),
                },
                Exp::Compound(compound) => {
                    let ctx = state.append_child(ctx);
                    match compound {
                        Compound::Parens(exps) => {
                            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Sequence;
                            compose_prefix_for_reg(prefix, exps, state, ctx)
                        }
                        Compound::Braces(exps) => {
                            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Stack;
                            compose_prefix_for_reg(prefix, exps, state, ctx)
                        }
                        Compound::Brackets(exps) => {
                            state.scope_types_mut()[ctx.to_usize()] = ScopeType::Set;
                            compose_prefix_for_reg(prefix, exps, state, ctx)
                        }
                        Compound::Ratio(abss) => {
                            state.scope_types_mut()[ctx.to_usize()] = ScopeType::None;
                            todo!()
                        }
                    }
                }
                Exp::None => todo!(),
            }))
        }
    }))
}

fn compose_prefix_for_reg(
    prefix: Prefix,
    exps: Vec<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    for exp in exps {
        let ctx = state.append_child(ctx);
        compose_exps(
            Monad(Exp::Simple(Simple::Primitive(Primitive::Prefix(prefix)))),
            Monad(exp),
            state,
            ctx,
        );
    }
    Monad(Exp::None)
}

fn compose_suffix(
    suffix: Monad<Suffix>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    suffix.bind(Box::new(|suffix| match suffix {
        Suffix::Bpm => {
            todo!()
        }
        Suffix::Amp => {
            todo!()
        }
        Suffix::Freq => {
            todo!()
        }
    }))
}
