#![allow(unused, const_item_mutation)]
use std::{
    clone,
    collections::{BTreeMap, HashSet},
    iter::{Cloned, Cycle, Iterator, Map, Peekable, Take, Zip, zip},
    ops::{Deref, DerefMut},
    path::PathBuf,
    slice::{Iter, IterMut},
    vec::{Drain, IntoIter},
};

use crate::compiler::{
    ast::*,
    codegen::{utils::*, *},
    functional::*,
};

use num_rational::BigRational;

const TERMINAL_WIDTH: usize = 130;

#[derive(Debug, Default)]
pub struct State {
    contexts: Vec<Ctx>,
    current_context: Ctx,
    parents: Vec<Ctx>,
    children: Vec<Vec<Ctx>>,
    garbage: Vec<Ctx>,
    stack: Vec<(Exp, Ctx)>,
    scope_types: Vec<ScopeType>,
    lengths: Vec<Vec<Length>>,
    pcs: Vec<Vec<Pc>>,
    tempos: Vec<Mpb>,
    bpms: Vec<Bpm>,
    registers: Vec<Register>,
    velocities: Vec<Vec<Velocity>>,
    programs: Vec<Prog>,
    bindings: Vec<BTreeMap<Ident, Exp>>,
}

impl State {
    pub fn push(&mut self, exp: Exp, ctx: Ctx) {
        eprintln!(
            "\x1b[0;32m{}\x1b[0m\n",
            format!("STACK <- {ctx:?} {exp}").to_uppercase()
        );
        self.stack.push((exp, ctx));
    }

    pub fn extend(&mut self, exps: Vec<(Exp, Ctx)>) {
        eprintln!(
            "\x1b[0;33mEXTEND {}\n\x1b[0m",
            exps.iter()
                .map(|(exp, ctx)| format!("{exp} {ctx:?}").to_uppercase())
                .collect::<Vec<String>>()
                .join(", ")
        );
        self.stack.extend(exps);
    }

    pub fn pop(&mut self) -> Option<(Exp, Ctx)> {
        eprintln!("POP\n");
        self.stack.pop()
    }

    pub fn take(&mut self, n: usize) -> Vec<(Exp, Ctx)> {
        let mut exps = Vec::<(Exp, Ctx)>::new();

        for _ in 0..n {
            if let Some(exp) = self.stack.pop() {
                exps.push(exp);
            }
        }
        exps.reverse();
        exps
    }

    fn append_child(&mut self, parent: Ctx) -> Ctx {
        let id = self.contexts_mut().len();
        let ctx = Ctx::Id(id);

        eprintln!("\x1b[0;32mAPPEND CHILD {parent:?} -> {ctx:?}\x1b[0m\n");
        let grandparent = self.parents.get(parent.to_usize());
        if grandparent.is_some() {
            let grandparent = grandparent.unwrap().clone();
            if matches!(self.lengths(parent)[0], Length::None) {
                let length = self.lengths[grandparent.to_usize()][0];
                if !matches!(length, Length::None) {
                    self.set_lengths(parent, vec![length]);
                }
            }
        }

        let pcs = Vec::<Pc>::new();
        let reg = self.register(parent);
        let velocity = self
            .velocities_mut()
            .get(parent.to_usize())
            .unwrap_or(&vec![Velocity(63)])
            .clone();

        let prog = if self.programs.get(parent.to_usize()).is_some() {
            std::mem::take(&mut self.programs[parent.to_usize()])
        } else {
            Prog(0)
        };

        let bpm = self
            .bpms_mut()
            .get(parent.to_usize())
            .unwrap_or(&Bpm(Absolute::UInt(120)))
            .clone();

        let tempo = if self.tempos.get(parent.to_usize()).is_some() {
            std::mem::take(&mut self.tempos[parent.to_usize()])
        } else {
            Mpb::default()
        };

        let lengths = self.lengths(parent).clone();

        self.contexts_mut().push(ctx);
        self.parents_mut().push(Ctx::Id(parent.to_usize()));
        self.children_mut().push(Vec::<Ctx>::new());
        if !matches!(parent, Ctx::Root) {
            self.children_mut()[parent.to_usize()].push(ctx);
        }
        self.scope_types_mut().push(ScopeType::None);
        self.lengths_mut().push(lengths);
        self.pcs_mut().push(pcs);
        self.bpms_mut().push(bpm);
        self.registers_mut().push(reg);
        self.velocities_mut().push(velocity);
        self.tempos_mut().push(tempo);
        self.programs_mut().push(prog);
        self.current_context = ctx;

        ctx
    }

    pub fn move_child(&mut self, child: Ctx, to: Ctx) {
        let parent = self.parent(child);
        eprintln!(
            "\x1b[0;35m{}\x1b[0m",
            format!("MOVE {parent:?} -> {child:?} => {to:?} -> {child:?}\n").to_uppercase()
        );
        let parent = self.parent(child);
        let siblings = &mut self.children(parent);
        siblings.sort_by(|c1, c2| c1.to_usize().cmp(&c2.to_usize()));
        let idx = siblings.binary_search(&child).unwrap();
        siblings.remove(idx);
        self.parents_mut()[child.to_usize()] = to;
        self.children_mut()[to.to_usize()].push(child);
    }

    pub fn drop(&mut self, child: Ctx) {
        eprintln!("{}", format!("\x1b[0;31mDROP {child:?}\x1b[0m\n"));
        let index = child.to_usize();
        std::mem::take(&mut self.scope_types[index]);
        std::mem::take(&mut self.lengths[index]);
        std::mem::take(&mut self.pcs[index]);
        std::mem::take(&mut self.registers[index]);
        std::mem::take(&mut self.bpms[index]);
        std::mem::take(&mut self.tempos[index]);
        std::mem::take(&mut self.programs[index]);
        std::mem::take(&mut self.velocities[index]);

        let parent = self.parent(child);
        let mut siblings = std::mem::take(&mut self.children[parent.to_usize()]);

        siblings = siblings
            .into_iter()
            .filter(|c| c.to_usize() != child.to_usize())
            .collect();
        std::mem::take(&mut self.contexts[index]);
        std::mem::take(&mut self.parents[index]);
        self.children[parent.to_usize()] = siblings;
    }

    fn current_context(&self) -> Ctx {
        self.current_context
    }

    pub fn lengths(&mut self, ctx: Ctx) -> Vec<Length> {
        if let Some(lengths) = self.lengths.get(ctx.to_usize()) {
            lengths.clone()
        } else {
            vec![Length::None]
        }
    }

    fn parents_mut(&mut self) -> &mut Vec<Ctx> {
        &mut self.parents
    }

    fn scope_types_mut(&mut self) -> &mut Vec<ScopeType> {
        &mut self.scope_types
    }

    pub fn scope_type(&self, ctx: Ctx) -> ScopeType {
        self.scope_types[ctx.to_usize()]
    }

    fn pcs_mut(&mut self) -> &mut Vec<Vec<Pc>> {
        &mut self.pcs
    }

    pub fn pcs(&self, ctx: Ctx) -> Vec<Pc> {
        if let Some(pc) = self.pcs.get(ctx.to_usize()) {
            pc.clone()
        } else {
            vec![]
        }
    }

    pub fn get_context(&mut self, ctx: Ctx) -> Context {
        Context {
            ctx,
            parent: self.parent(ctx),
            children: self.children(ctx),
            scope: self.scope_type(ctx),
            register: self.register(ctx),
            pcs: self.pcs(ctx),
            velocities: self.velocities(ctx).clone(),
            bpm: self.bpm(ctx),
            lengths: self.lengths(ctx).clone(),
            tempo: self.tempo(ctx),
            program: self.program(ctx),
        }
    }

    pub fn tempo(&self, ctx: Ctx) -> Mpb {
        self.tempos[ctx.to_usize()]
    }

    fn set_tempo(&mut self, ctx: Ctx, tempo: Mpb) {
        self.tempos[ctx.to_usize()] = tempo;
    }

    fn bpms_mut(&mut self) -> &mut Vec<Bpm> {
        &mut self.bpms
    }

    pub fn bpm(&self, ctx: Ctx) -> Bpm {
        self.bpms[ctx.to_usize()]
    }

    pub fn set_bpm(&mut self, ctx: Ctx, bpm: Bpm) {
        self.bpms_mut()[ctx.to_usize()] = bpm;
    }

    fn registers_mut(&mut self) -> &mut Vec<Register> {
        &mut self.registers
    }

    fn set_register(&mut self, mut ctx: Ctx, register: Register) {
        self.registers_mut()[ctx.to_usize()] = register;
    }

    fn velocities_mut(&mut self) -> &mut Vec<Vec<Velocity>> {
        &mut self.velocities
    }

    pub fn velocities(&self, ctx: Ctx) -> &Vec<Velocity> {
        &self.velocities[ctx.to_usize()]
    }

    pub fn set_velocities(&mut self, ctx: Ctx, velocities: Vec<Velocity>) {
        self.velocities_mut()[ctx.to_usize()] = velocities;
    }

    fn programs_mut(&mut self) -> &mut Vec<Prog> {
        &mut self.programs
    }

    pub fn program(&self, ctx: Ctx) -> Prog {
        self.programs[ctx.to_usize()]
    }

    pub fn set_program(&mut self, ctx: Ctx, program: Prog) {
        self.programs_mut()[ctx.to_usize()] = program;
    }

    pub fn children_mut(&mut self) -> &mut Vec<Vec<Ctx>> {
        &mut self.children
    }

    fn contexts_mut(&mut self) -> &mut Vec<Ctx> {
        &mut self.contexts
    }

    pub fn contexts(&self) -> &[Ctx] {
        &self.contexts
    }

    fn set_lengths(&mut self, ctx: Ctx, lengths: Vec<Length>) {
        self.lengths[ctx.to_usize()] = lengths;
    }

    fn set_scope_type(&mut self, ctx: Ctx, scope_type: ScopeType) {
        eprintln!("\x1b[032mSET {ctx:?} TO SCOPE TYPE {scope_type:?}\x1b[0m\n");
        self.scope_types[ctx.to_usize()] = scope_type;
    }

    pub fn tempos_mut(&mut self) -> &mut Vec<Mpb> {
        &mut self.tempos
    }

    pub fn parent(&self, ctx: Ctx) -> Ctx {
        if let Some(ctx) = self.parents.get(ctx.to_usize()) {
            *ctx
        } else {
            Ctx::None
        }
    }

    pub fn lengths_mut(&mut self) -> &mut Vec<Vec<Length>> {
        &mut self.lengths
    }

    pub fn binding(&self, ctx: Ctx) -> &BTreeMap<Ident, Exp> {
        &self.bindings[ctx.to_usize()]
    }

    pub fn add_binding(&mut self, ctx: Ctx, ident: Ident, binding: Exp) {
        self.bindings[ctx.to_usize()].insert(ident, binding);
    }

    pub fn children(&self, ctx: Ctx) -> Vec<Ctx> {
        self.children[ctx.to_usize()].clone()
    }

    pub fn tempos(&self) -> &[Mpb] {
        &self.tempos
    }

    pub fn register(&self, ctx: Ctx) -> Register {
        match self.registers.get(ctx.to_usize()) {
            Some(reg) => *reg,
            None => Register::None,
        }
    }

    pub fn set_current_context(&mut self, ctx: Ctx) {
        self.current_context = ctx;
    }

    pub fn discard(&mut self, ctx: Ctx) {
        eprintln!("\x1b[0;31mDISCARD {ctx:?}\x1b[0m");
        self.garbage.push(ctx);
    }

    pub fn collect_garbage(&mut self) {
        let garbage = self.garbage.clone();
        for ctx in garbage {
            self.drop(ctx);
        }
        self.garbage.clear();
    }
}

pub fn compose_program(ast: Program) -> State {
    let mut state = State::default();
    let _ = state.append_child(Ctx::Root);
    let mut exps = ast.exps;
    exps.push(Exp::EOS);

    exps.into_iter().fold(Monad::ret(NOOP), |m, rhs| {
        m.bind(Box::new(|mut lhs: Exp| {
            let ctx = state.current_context();
            let res = combine(lhs, rhs, &mut state, ctx);

            res
        }))
    });

    graph(&mut state, Ctx::Id(0), 0);
    state.collect_garbage();
    state
}

fn combine(lhs: Exp, rhs: Exp, state: &mut State, mut ctx: Ctx) -> Monad<Exp> {
    let indent_count: usize = ctx.to_usize();
    let parent: Ctx = state.parent(ctx);
    let lhs_clone = lhs.clone();
    let rhs_clone = rhs.clone();

    print_state(state, ctx);
    dbg!();
    print_exps(&lhs_clone, &rhs_clone, ctx);

    let res = match (lhs, rhs) {
        (lhs @ Exp::Noop, rhs) => match rhs {
            ref rhs @ Exp::Compound(ref compound) => {
                let ctx = state.append_child(ctx);
                match **compound {
                    Compound::Parens(_) => state.set_scope_type(ctx, ScopeType::Sequence),
                    Compound::Braces(_) => state.set_scope_type(ctx, ScopeType::Stack),
                    _ => state.set_scope_type(ctx, ScopeType::None),
                }
                Monad::ret(rhs.clone()).bind(Box::new(|rhs| match rhs {
                    rhs => combine(rhs, NOOP, state, ctx),
                }))
            }
            rhs @ Exp::Simple(Simple::Prefix(_)) => Monad::ret(rhs),
            Exp::Noop => {
                // dbg!();
                if let Some((lhs, ctx)) = state.pop() {
                    eprintln!("\x1b[0;31m{}\x1b[0m\n", format!("{lhs} {ctx:?}"));
                    match lhs {
                        Exp::Compound(compound) => consume_compound(*compound, state, ctx),
                        _ => todo!(),
                    }
                } else {
                    Monad::ret(NOOP)
                }
            }
            rhs => {
                dbg!();
                if let Some((lhs, ctx)) = state.pop() {
                    combine(lhs, rhs, state, ctx)
                } else {
                    Monad::ret(rhs)
                }
            }
        },
        (lhs, Exp::Noop) => match lhs {
            ref lhs @ Exp::Compound(ref compound) => {
                consume_prefixes(*compound.clone(), state, ctx).bind(Box::new(|rhs: Exp| {
                    let parent = state.parent(ctx);
                    match state.scope_type(parent) {
                        ScopeType::Stack => {
                            if matches!(**compound, Compound::Parens(_)) {
                                state.push(lhs.clone(), ctx);
                                Monad::ret(NOOP)
                            } else {
                                if let Some((lhs, ctx_)) = state.pop() {
                                    dbg!();
                                    eprintln!("{lhs} {ctx_:?}");
                                    combine(lhs.clone(), rhs.clone(), state, ctx_).bind(Box::new(
                                        |lhs| {
                                            dbg!();
                                            combine(NOOP, lhs, state, ctx)
                                        },
                                    ))
                                } else {
                                    dbg!();
                                    eprintln!("{rhs} {ctx:?}");
                                    state.push(rhs, ctx);
                                    state.set_current_context(parent);
                                    Monad::ret(NOOP)
                                }
                            }
                        }
                        _ => {
                            if let Some((lhs, ctx_)) = state.pop() {
                                dbg!();
                                eprintln!("{lhs} {ctx_:?}");
                                combine(lhs.clone(), rhs.clone(), state, ctx_).bind(Box::new(
                                    |lhs| {
                                        eprintln!("{rhs} {ctx:?}");
                                        state.push(rhs.clone(), ctx);
                                        state.set_current_context(parent);
                                        combine(NOOP, lhs, state, parent)
                                    },
                                ))
                            } else {
                                dbg!();
                                eprintln!("{rhs} {ctx:?}");
                                state.push(rhs, ctx);
                                state.set_current_context(parent);
                                Monad::ret(NOOP)
                            }
                        }
                    }
                }))
            }
            _ => Monad::ret(lhs),
        },
        (lhs, Exp::EOS) => match lhs {
            Exp::Compound(compound) => {
                consume_prefixes(*compound, state, ctx).bind(Box::new(|lhs| match lhs {
                    Exp::Compound(compound) => consume_compound(*compound, state, ctx),
                    _ => Monad::ret(NOOP),
                }))
            }
            _ => Monad::ret(NOOP),
        },
        (Exp::Compound(lhs), Exp::Compound(rhs)) => match (*lhs.to_owned(), *rhs.to_owned()) {
            (Compound::Parens(lhs_exps), Compound::Parens(rhs_exps)) => {
                let lhs_ctx = ctx;
                let rhs_ctx = state.current_context();
                // if matches!(state.scope_type(parent), ScopeType::Stack) {
                //     let m = merge_sequences(lhs_exps, rhs_exps, state, lhs_ctx, rhs_ctx);
                //     m
                // } else {
                consume_compound(Compound::Parens(lhs_exps), state, ctx).bind(Box::new(|lhs| {
                    eprintln!("{lhs} {ctx:?}");

                    state.set_current_context(ctx);
                    Monad::ret(NOOP)
                }))
                // }
            }
            (lhs @ Compound::Braces(_), rhs @ Compound::Braces(_)) => {
                let parent_scope = state.scope_type(parent);

                match parent_scope {
                    ScopeType::Stack => consume_compound(lhs, state, ctx),
                    ScopeType::Sequence | _ => consume_compound(lhs, state, ctx),
                }
            }
            (mut lhs @ Compound::Parens(_), rhs @ Compound::Braces(_)) => {
                let parent_scope = state.scope_type(parent);
                match parent_scope {
                    ScopeType::Stack => consume_compound(lhs, state, ctx),
                    ScopeType::Sequence | _ => consume_compound(lhs, state, ctx),
                }
            }
            (lhs @ Compound::Braces(_), rhs @ Compound::Parens(_)) => {
                let parent_scope = state.scope_type(parent);

                match parent_scope {
                    ScopeType::Stack => consume_compound(lhs, state, ctx),
                    ScopeType::Sequence | _ => consume_compound(lhs, state, ctx),
                }
            }
            (lhs, rhs) => {
                todo!()
            }
            _ => todo!(),
        },
        (lhs @ Exp::Compound(_), Exp::Simple(simple)) => {
            if matches!(simple, Simple::Suffix(_)) {
                compose_simple(Monad::ret(simple), Monad::ret(lhs), state, ctx)
            } else {
                dbg!();
                eprintln!("{lhs} {ctx:?}");
                state.push(lhs, ctx);
                let ctx = state.parent(ctx);
                state.set_current_context(ctx);
                combine(Exp::Simple(simple), NOOP, state, ctx)
            }
        }
        (Exp::Simple(simple), rhs @ Exp::Compound(_)) => match simple {
            Simple::Scalar(Scalar::Duration(duration)) => {
                compose_duration(Monad::ret(duration), Monad::ret(rhs), state, ctx)
            }
            simple => compose_simple(Monad::ret(simple), Monad::ret(rhs), state, ctx),
        },
        (ref lhs @ Exp::Simple(ref s1), ref rhs @ Exp::Simple(ref s2)) => match (s1, s2) {
            (Simple::Prefix(prefix), Simple::Scalar(Scalar::Pure(_))) => match prefix {
                prefix @ Prefix::Dur | prefix @ Prefix::Pc => compose_prefix(
                    Monad::ret(prefix.clone()),
                    Monad::ret(rhs.clone()),
                    state,
                    ctx,
                ),
                _ => {
                    dbg!();
                    state.extend(vec![(lhs.clone(), ctx), (rhs.clone(), ctx)]);
                    Monad::ret(NOOP)
                }
            },
            _ => todo!(),
        },

        (lhs, rhs) => {
            if state.stack.len() > 0 {
                eprintln!(
                    "\x1b[0;33mStack:\n{}\n\n\x1b[0m",
                    align(&state.stack, indent_count, 80)
                );
            }

            todo!()
        }
    };

    res
}

fn drain_stack(rhs: Exp, state: &mut State, mut ctx: Ctx) -> (Monad<Exp>, Ctx) {
    eprintln!("DRAIN STACK\n");

    let mut m: Monad<Exp> = Monad::ret(rhs);

    while let Some((mut lhs_, ctx_)) = state.pop() {
        ctx = ctx_;
        m = m.bind(Box::new(|rhs| match lhs_ {
            Exp::Simple(Simple::Prefix(lhs)) => {
                compose_prefix(Monad::ret(lhs), Monad::ret(rhs), state, ctx)
            }
            Exp::Simple(Simple::Scalar(Scalar::Duration(duration))) => {
                compose_duration(Monad::ret(duration), Monad::ret(rhs), state, ctx)
            }
            Exp::Simple(Simple::Scalar(Scalar::Dynamic(dynamic))) => {
                compose_dynamic(Monad::ret(dynamic), Monad::ret(rhs), state, ctx)
            }
            Exp::Noop => combine(rhs, NOOP, state, ctx),
            mut lhs @ Exp::Compound(_) => combine(lhs, rhs, state, ctx),
            _ => {
                let (mut m_, ctx_) = drain_stack(lhs_.to_owned(), state, ctx);
                m_ = m_.bind(Box::new(|mut lhs| combine(lhs, rhs, state, ctx)));
                ctx = ctx_;
                m_
            }
        }));
    }

    state.set_current_context(ctx);
    (m, ctx)
}

fn consume_prefixes(compound: Compound, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    let mut pair = state.take(2);

    let mut m = Monad::ret(Exp::Compound(Box::new(compound.clone())));

    while pair.len() == 2 {
        if let Exp::Simple(Simple::Prefix(prefix)) = pair[0].0 {
            m = m.bind(Box::new(|mut rhs| {
                eprintln!(
                    "\x1b[0;33m{}\x1b[0m\n",
                    format!(
                        "CONSUME {} <- {} {:?}",
                        Exp::Simple(Simple::Prefix(prefix)),
                        rhs,
                        ctx
                    )
                );
                compose_prefix(
                    Monad::ret(prefix),
                    Monad::ret(pair[1].0.clone()),
                    state,
                    ctx,
                )
                .bind(Box::new(|lhs| match lhs {
                    Exp::Noop => Monad::ret(rhs),
                    lhs => combine(lhs, rhs, state, ctx),
                }))
            }));
        } else {
            dbg!();
            state.extend(pair.clone());
            break;
        }

        pair = state.take(2);
    }

    // if pair.len() < 2 {

    if pair.len() == 1 {
        dbg!();
        eprintln!("{}", pair[0].0);
        state.push(pair[0].0.clone(), pair[0].1);
    }

    m
}

fn consume_compound(mut compound: Compound, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    let compound_string =
        format!("{} {:?}", Exp::Compound(Box::new(compound.clone())), ctx).to_uppercase();
    eprintln!("\x1b[1;31mCONSUME {}\x1b[0m\n", compound_string);
    match compound {
        Compound::Parens(ref mut exps)
        | Compound::Braces(ref mut exps)
        | Compound::Brackets(ref mut exps) => {
            consume_compound_exps(compound_string, exps, state, ctx)
        }
        _ => todo!(),
    }
}

fn consume_compound_exps(
    compound_string: String,
    exps: &mut Vec<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    let m = exps
        .iter()
        .cloned()
        .fold(Monad::ret(NOOP), |m, rhs| {
            m.bind(Box::new(|mut lhs| {
                dbg!();
                match lhs {
                    lhs => {
                        eprintln!("{lhs} <- {rhs} {ctx:?}\n");
                        combine(lhs, rhs, state, ctx)
                    }
                }
            }))
        })
        .bind(Box::new(|_| {
            dbg!();
            if let Some((lhs, ctx)) = state.pop() {
                let parent = state.parent(ctx);
                let parent_scope = state.scope_type(parent);
                let scope = state.scope_type(ctx);
                eprintln!("{parent_scope:?} {parent:?} -> {scope:?} {ctx:?}");
                match (parent_scope, scope) {
                    (ScopeType::Stack, ScopeType::Sequence) => {
                        state.push(lhs, ctx);
                        consume_sequences(parent, state)
                        // Monad::ret(NOOP)
                    }
                    _ => match lhs {
                        Exp::Compound(compound) => consume_compound(*compound, state, ctx),
                        _ => Monad::ret(NOOP),
                    },
                }
            } else {
                Monad::ret(NOOP)
            }
        }))
        .bind(Box::new(|_| consume_sequences(ctx, state)));

    eprintln!("\x1b[1;31m{compound_string} CONSUMED\x1b[0m\n");

    m.bind(Box::new(|lhs| {
        eprintln!("{lhs}\n");
        Monad::ret(lhs)
    }))
}

fn consume_sequences(parent: Ctx, state: &mut State) -> Monad<Exp> {
    eprintln!("\x1b[1;35mCONSUME SEQUENCES\x1b[0m\n");
    let parent_scope = state.scope_type(parent);
    eprintln!("\x1b[0;35mPARENT: {parent_scope:?} {parent:?}\x1b[0m\n");
    eprintln!("\x1b[0;35mStack length: {}\x1b[0m\n", state.stack.len());

    if !matches!(parent_scope, ScopeType::Stack) {
        dbg!();
        print_state(state, parent);
        Monad::ret(NOOP)
    } else {
        let mut sequences = Vec::<(IntoIter<Exp>, Ctx)>::new();
        while let Some(((exp @ Exp::Compound(_)), ctx)) = state.stack.pop() {
            sequences.push((exp_to_exps(exp).into_iter(), ctx));
        }

        if sequences.is_empty() {
            return Monad::ret(NOOP);
        }

        sequences.reverse();

        eprintln!(
            "\x1b[0;35msequences:\n{}\x1b[0m\n",
            format!("{sequences:?}")
                .split_inclusive(|c| matches!(c, ',' | '['))
                .collect::<Vec<&str>>()
                .join("\n")
                .to_string()
        );

        merge_sequences(parent, state, &mut sequences);

        Monad::ret(NOOP)
    }
}

fn merge_sequences<'a>(
    parent: Ctx,
    state: &mut State,
    mut sequences: &mut Vec<(IntoIter<Exp>, Ctx)>,
) {
    let len = sequences.len();

    let iters = sequences.into_iter().fold(
        Vec::<(&mut IntoIter<Exp>, Ctx)>::new(),
        |mut iters, (iter_, ctx_)| {
            iters.push((iter_, ctx_.clone()));
            let len_ = iters.len();
            eprintln!("\x1b[0;34mlen_: {len_}  len: {len}\x1b[0m\n");
            if len_ == len {
                let mut ctx_ = state.append_child(parent);
                state.set_scope_type(ctx_, ScopeType::Stack);

                'merge: loop {
                    for (iter, ctx__) in &mut iters {
                        if let Some(lhs) = (*iter).next() {
                            if let Some(rhs) = (*iter).next() {
                                let mut ctx__ = state.append_child(*ctx__);
                                state.set_scope_type(ctx__, ScopeType::Stack);
                                state.move_child(ctx__, ctx_);
                                combine(lhs, rhs, state, ctx__);
                            }
                        } else {
                            state.discard(ctx_);
                            break 'merge;
                        }
                    }
                    ctx_ = state.append_child(parent);
                    state.set_scope_type(ctx_, ScopeType::Stack);
                }
            }

            iters
        },
    );

    iters.into_iter().for_each(|(_, ctx)| state.discard(ctx));
    state.set_scope_type(parent, ScopeType::Sequence);
}

fn exp_to_exps(seq: Exp) -> Vec<Exp> {
    let mut seq = match seq {
        Exp::Compound(compound) => match *compound {
            Compound::Parens(seq) => seq,
            _ => todo!(),
        },
        _ => todo!(),
    };
    seq
}

pub fn print_state(state: &mut State, ctx: Ctx) {
    let parent = state.parent(ctx);
    eprintln!(
        "\x1b[1;36m{:?} {:?} -> {:?} {:?}\x1b[0m\n\x1b[0;36mPCs : {:?}\nReg : {:?}\nLens: {:?}\nChil: {:?}\nStck: {}\x1b[0m\n",
        parent,
        state.scope_type(parent),
        ctx,
        state.scope_type(ctx),
        state.pcs(ctx),
        state.register(ctx),
        state.lengths(ctx),
        state.children(ctx),
        format!(
            "[{}]",
            state
                .stack
                .iter()
                .map(|(exp, ctx)| format!("({exp}, {ctx:?})"))
                .collect::<Vec<String>>()
                .join(", ")
        )
    );
}

fn compose_simple(
    simple: Monad<Simple>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    simple.bind(Box::new(|simple| match simple {
        Simple::Prefix(prefix) => compose_prefix(Monad::ret(prefix), rhs, state, ctx),
        Simple::Scalar(scalar) => compose_scalar(Monad::ret(scalar), rhs, state, ctx),
        Simple::Infix(infix) => compose_infix(Monad::ret(infix), rhs, state, ctx),
        Simple::Suffix(suffix) => compose_suffix(Monad::ret(suffix), rhs, state, ctx),
        Simple::Ident(ident) => compose_ident(Monad::ret(ident), rhs, state, ctx),
    }))
}

fn compose_decl(decl: Monad<Decl>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    decl.bind(Box::new(|Decl { ident, binding }| {
        state.add_binding(ctx, ident, *binding);
        rhs
    }))
}

fn compose_scalar(
    scalar: Monad<Scalar>,

    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    scalar.bind(Box::new(|scalar| match scalar {
        Scalar::Duration(duration) => compose_duration(Monad::ret(duration), rhs, state, ctx),
        Scalar::Dynamic(dynamic) => compose_dynamic(Monad::ret(dynamic), rhs, state, ctx),
        Scalar::Frequency(frequency) => compose_frequency(Monad::ret(frequency), rhs, state, ctx),
        Scalar::Tempo(absolute) => compose_tempo(Monad::ret(absolute), state, ctx),
        Scalar::Pure(pure) => compose_pure(Monad::ret(pure), rhs, state, ctx),
    }))
}

fn compose_tempo(absolute: Monad<Absolute>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_dynamic(
    dynamic: Monad<String>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    todo!()
}

fn compose_infix(infix: Monad<Infix>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    todo!()
}

fn compose_ident(ident: Monad<Ident>, rhs: Monad<Exp>, state: &mut State, ctx: Ctx) -> Monad<Exp> {
    ident.bind(Box::new(|ident| {
        Monad::ret(state.binding(ctx).get(&ident).unwrap_or(&NOOP).clone())
    }))
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
    eprintln!("COMPOSE DURATION\n");

    duration.bind(|duration| match duration {
        Duration::Fixed(Fixed { minutes, seconds }) => {
            let length = Length::MicroSeconds(
                minutes.as_u64() * 60 * 1_000_000 + seconds.as_u64() * 1_000_000,
            );

            state.set_lengths(ctx, vec![length]);

            rhs
        }
        Duration::Fractional(fractional) => {
            compose_fractional(Monad::ret(fractional), rhs, state, ctx)
        }
    })
}

fn compose_fractional(
    fractional: Monad<Fractional>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    let res = fractional.bind(|fractional| match fractional {
        fractional @ Fractional::Absolute(abs) => {
            let parent_length = state.lengths(ctx)[0];
            let parent_micros = parent_length.as_u64();
            let denominator = abs.as_f64();
            let tempo = state.tempo(ctx);
            let beats = 4 as f64 / denominator;
            let duration = (beats * tempo.0 as f64) as u64;
            let duration_micros = u64::min(parent_micros, duration);

            rhs.bind(Box::new(|rhs| {
                let res = match rhs {
                    Exp::Compound(compound) => {
                        let ctx = state.append_child(ctx);
                        state.set_lengths(ctx, vec![Length::MicroSeconds(duration_micros)]);
                        match *compound {
                            Compound::Parens(_) => state.set_scope_type(ctx, ScopeType::Sequence),
                            Compound::Braces(_) => state.set_scope_type(ctx, ScopeType::Stack),
                            Compound::Brackets(_) => state.set_scope_type(ctx, ScopeType::Set),
                            _ => todo!(),
                        }
                        combine(Exp::Compound(compound), NOOP, state, ctx)
                    }

                    mut simple @ Exp::Simple(_) => {
                        state.set_lengths(ctx, vec![Length::MicroSeconds(duration_micros)]);
                        combine(simple, NOOP, state, ctx)
                    }

                    Exp::Noop => Monad::ret(Exp::Simple(Simple::Scalar(Scalar::Duration(
                        Duration::Fractional(fractional),
                    )))),
                    Exp::EOS => todo!(),
                };
                res
            }))
        }
        Fractional::Tuplet(Tuplet { lhs: num, rhs: den }) => {
            let dur = Monad::ret(Duration::Fractional(Fractional::Absolute(den / num)));
            rhs.bind(Box::new(|exp| match exp {
                Exp::Simple(simple) => todo!(),
                Exp::Compound(compound) => match *compound {
                    Compound::Parens(exps) => {
                        state.set_scope_type(ctx, ScopeType::Sequence);
                        let mut iter = exps.iter().cloned();
                        let init = compose_duration(
                            dur.clone(),
                            Monad::ret(iter.next().unwrap()),
                            state,
                            ctx,
                        );
                        exps.into_iter().fold(init, |m, rhs| {
                            m.bind(Box::new(|mut lhs: Exp| combine(lhs, rhs, state, ctx)))
                        })
                    }
                    compound @ Compound::Braces(_) => {
                        state.set_scope_type(ctx, ScopeType::Stack);
                        let mut m = Monad::ret(NOOP);

                        for _ in 0..num.as_u64() {
                            m = dur.clone().bind(Box::new(|duration: Duration| {
                                state.set_scope_type(ctx, ScopeType::Stack);
                                compose_duration(
                                    Monad::ret(duration),
                                    Monad::ret(Exp::Compound(Box::new(compound.clone()))),
                                    state,
                                    ctx,
                                )
                            }));
                        }
                        m
                    }
                    Compound::Brackets(exps) => todo!(),
                    Compound::Ratio(exps) => todo!(),
                    Compound::Decl(decl) => todo!(),
                },
                Exp::Noop => todo!(),
                Exp::EOS => todo!(),
            }))
        }
    });

    res
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
    graph(state, Ctx::Id(0), 0);
    todo!()
}

fn compose_prefix(
    prefix: Monad<Prefix>,
    rhs: Monad<Exp>,
    state: &mut State,
    mut ctx: Ctx,
) -> Monad<Exp> {
    // eprintln!("COMPOSE PREFIX\n");

    prefix.bind(Box::new(|prefix| match prefix {
        pc @ Prefix::Pc => rhs.bind(Box::new(|rhs| match rhs {
            Exp::Simple(simple) => match simple {
                Simple::Scalar(scalar) => match scalar {
                    Scalar::Pure(pure) => match pure {
                        Pure::Absolute(abs) => {
                            state.pcs_mut()[ctx.to_usize()].push(Pc::Class(abs.as_u64() as i8));
                            combine(NOOP, NOOP, state, ctx)
                        }
                        Pure::Relative(Relative { sign, val }) => {
                            let pcs = &mut state.pcs_mut()[ctx.to_usize()];
                            let last = pcs.pop().unwrap_or_default();

                            match last {
                                Pc::Class(last) => match sign {
                                    Sign::Plus => pcs.insert(
                                        ctx.to_usize(),
                                        Pc::Class(last.clone() + val.as_u64() as i8),
                                    ),
                                    Sign::Minus => pcs.insert(
                                        ctx.to_usize(),
                                        Pc::Class(last.clone() - val.as_u64() as i8),
                                    ),
                                },
                                Pc::None => {
                                    let parent = state.parents_mut()[ctx.to_usize()].clone();
                                    let pcs = &mut state.pcs_mut()[parent.to_usize()];
                                    let last = pcs.pop().unwrap_or_default();

                                    match last {
                                        Pc::Class(last) => match sign {
                                            Sign::Plus => pcs.insert(
                                                ctx.to_usize(),
                                                Pc::Class(last.clone() + val.as_u64() as i8),
                                            ),

                                            Sign::Minus => {
                                                pcs.insert(
                                                    ctx.to_usize(),
                                                    Pc::Class(last.clone() + val.as_u64() as i8),
                                                );
                                            }
                                        },
                                        Pc::None => {
                                            pcs.insert(
                                                ctx.to_usize(),
                                                Pc::Class(val.as_u64() as i8),
                                            );
                                        }
                                    }
                                }
                            }
                            combine(NOOP, NOOP, state, ctx)
                        }
                    },
                    _ => combine(NOOP, NOOP, state, ctx),
                },
                Simple::Prefix(prefix) => todo!(),

                Simple::Infix(infix) => match infix {
                    Infix::Colon => todo!(),
                    Infix::Intercalate => todo!(),
                    Infix::Range => todo!(),
                    Infix::Interpolation(interpolation) => todo!(),
                },
                Simple::Suffix(suffix) => todo!(),
                Simple::Ident(ident) => todo!(),
            },

            Exp::Compound(compound) => match *compound {
                Compound::Parens(exps) => {
                    let mut exp = Exp::Compound(Box::new(Compound::Parens(
                        exps.into_iter()
                            .flat_map(|rhs| vec![Exp::Simple(Simple::Prefix(pc)), rhs])
                            .collect::<Vec<Exp>>(),
                    )));
                    let parent = state.parent(ctx);

                    let parent_scope = state.scope_type(parent);
                    combine(NOOP, exp, state, ctx)
                }
                Compound::Braces(exps) => {
                    let exp = Exp::Compound(Box::new(Compound::Braces(
                        exps.into_iter()
                            .flat_map(|exp| vec![Exp::Simple(Simple::Prefix(pc)), exp])
                            .collect(),
                    )));

                    combine(NOOP, exp, state, ctx)
                }
                Compound::Brackets(exps) => {
                    todo!()
                }
                Compound::Ratio(abss) => {
                    todo!()
                }
                Compound::Decl(decl) => {
                    todo!()
                }
            },
            Exp::Noop => todo!(),
            Exp::EOS => todo!(),
        })),
        Prefix::Dur => rhs.bind(Box::new(|exp| match exp {
            Exp::Simple(simple) => match simple {
                Simple::Scalar(scalar) => match scalar {
                    Scalar::Pure(pure) => match pure {
                        Pure::Absolute(abs) => combine(
                            Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
                                Fractional::Absolute(abs),
                            )))),
                            NOOP,
                            state,
                            ctx,
                        ),
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
                _ => todo!(),
            },
            Exp::Noop => combine(NOOP, Exp::Simple(Simple::Prefix(Prefix::Dur)), state, ctx),
            rhs => {
                todo!()
            }
        })),
        Prefix::Rest => {
            todo!()
        }
        reg @ Prefix::Reg => rhs.bind(Box::new(|rhs| match rhs {
            Exp::Simple(simple) => match simple {
                Simple::Scalar(scalar) => match scalar {
                    Scalar::Pure(pure) => match pure {
                        Pure::Absolute(abs) => {
                            let register = Register::Reg(abs.as_u64() as i8);
                            state.set_register(ctx, register);
                            state.set_current_context(ctx);
                            Monad::ret(NOOP)
                        }
                        Pure::Relative(relative) => todo!(),
                    },
                    Scalar::Duration(duration) => todo!(),
                    Scalar::Tempo(abs) => todo!(),
                    Scalar::Dynamic(string) => todo!(),
                    Scalar::Frequency(frequency) => todo!(),
                },
                Simple::Prefix(prefix) => match prefix {
                    prefix @ Prefix::Pc => Monad::ret(Exp::Simple(Simple::Prefix(prefix))),
                    Prefix::Dur => todo!(),
                    Prefix::Rest => todo!(),
                    Prefix::Reg => todo!(),
                },
                Simple::Infix(infix) => todo!(),
                Simple::Suffix(suffix) => todo!(),
                Simple::Ident(ident) => todo!(),
            },
            Exp::Compound(compound) => match *compound {
                Compound::Parens(exps) => {
                    state.set_scope_type(ctx, ScopeType::Sequence);
                    todo!()
                }
                Compound::Braces(exps) => {
                    state.set_scope_type(ctx, ScopeType::Stack);
                    todo!()
                }
                Compound::Brackets(exps) => {
                    state.scope_types_mut()[ctx.to_usize()] = ScopeType::Set;
                    todo!()
                }
                Compound::Ratio(abss) => {
                    state.scope_types_mut()[ctx.to_usize()] = ScopeType::None;
                    todo!()
                }
                decl @ Compound::Decl(_) => Monad::ret(Exp::Compound(Box::new(decl))),
            },
            Exp::Noop => return Monad::ret(Exp::Simple(Simple::Prefix(reg))),
            Exp::EOS => todo!(),
        })),
    }))
}

fn compose_suffix(
    suffix: Monad<Suffix>,
    rhs: Monad<Exp>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<Exp> {
    suffix.bind(Box::new(|suffix| match suffix {
        Suffix::Bpm => rhs.bind(Box::new(|exp| match exp {
            Exp::Compound(compound) => match *compound {
                Compound::Parens(exps) => {
                    todo!()
                }
                _ => todo!(),
            },
            _ => todo!(),
        })),
        Suffix::Amp => {
            todo!()
        }
        Suffix::Freq => {
            todo!()
        }
    }))
}

fn pause() {
    let _ = std::io::stdin().read_line(&mut String::new());
}

fn graph(state: &mut State, ctx: Ctx, mut indent: usize) {
    eprintln!(
        "\x1b[0;36m{0:^1$}\x1b[0m",
        format!("{:?}", ctx),
        TERMINAL_WIDTH
    );
    eprintln!("\x1b[0;36m{0:^1$}\x1b[0m", "|", TERMINAL_WIDTH);
    let mut node_count = 0;
    let mut visited = HashSet::<usize>::new();

    let branch_width = TERMINAL_WIDTH / (node_count + 1);
    let children = state.children(ctx);
    let child_count = children.len();

    let branchline = format!("{0:1$}", "_", branch_width).repeat(child_count - 1);
    eprintln!("{branchline:^0$}", TERMINAL_WIDTH);
}

fn print_exps(lhs: &Exp, rhs: &Exp, ctx: Ctx) {
    eprintln!(
        "\x1b[1;36m{} <- {}\x1b[0m\n",
        format!("{}", lhs).to_uppercase(),
        format!("{}", rhs).to_uppercase()
    );
}
