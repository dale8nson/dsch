#![allow(unused, const_item_mutation)]
#![forbid(
    clippy::infinite_loop,
    clippy::maybe_infinite_iter,
    unconditional_recursion
)]
use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, RefCell},
    clone,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    default,
    io::{stderr, stdout},
    iter::{
        Cloned, Cycle, FlatMap, FromFn, Iterator, Map, Peekable, Take, Zip, once_with, repeat,
        repeat_n, zip,
    },
    num::{NonZero, NonZeroU64},
    ops::{Deref, DerefMut},
    path::PathBuf,
    rc::{Rc, Weak},
    slice::{Iter, IterMut},
    thread::current,
    u64,
    vec::{Drain, IntoIter},
};

use crate::compiler::{
    ast::*,
    codegen::{
        utils::{TextStyle::*, *},
        *,
    },
    functional::*,
};

use colonnade::{self, Alignment, Colonnade};
use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::{self, MoveToNextLine, RestorePosition, position},
    execute,
    terminal::{self, ClearType, size},
};
use rust_sugiyama::{
    configure::{Config, RankingType},
    from_edges,
};

use num_rational::BigRational;

const TERMINAL_WIDTH: usize = 130;

#[derive(Debug, Default)]
pub struct State {
    contexts: BTreeSet<Ctx>,
    current_context_id: usize,
    parents: HashMap<Ctx, Ctx>,
    children: BTreeMap<Ctx, Vec<Ctx>>,
    garbage: Vec<Ctx>,
    rhs_stack: Vec<(Exp, Ctx)>,
    inbox_index: usize,
    lhs_stack: Vec<(Exp, Ctx)>,
    scope_types: HashMap<Ctx, ScopeType>,
    lengths: HashMap<Ctx, Vec<Length>>,
    pcs: HashMap<Ctx, Vec<Pc>>,
    tempos: HashMap<Ctx, Mpb>,
    bpms: HashMap<Ctx, Bpm>,
    registers: HashMap<Ctx, Register>,
    velocities: HashMap<Ctx, Vec<Velocity>>,
    programs: HashMap<Ctx, Prog>,
    bindings: HashMap<Ctx, HashMap<Ident, Exp>>,
}

impl State {
    pub fn push_left(&mut self, (exp, ctx): (Exp, Ctx)) {
        // out(
        //     0,
        //     0,
        //     format!(
        //         "{IntenseYellow}{}{ResetColor}\n",
        //         format!("STACK <- {ctx:?} {exp}").to_uppercase()
        //     ),
        // );

        self.lhs_stack.push((exp, ctx));
    }

    pub fn extend_left(&mut self, mut exps: Vec<(Exp, Ctx)>) {
        // let mut ss = Vec::<String>::new();
        // let mut s = String::new();
        // s.push_str(format!("{IntenseYellow}STACK <== ").as_str());
        // let mut exps_ = exps.clone().into_iter();
        // while let Some((exp, ctx)) = exps_.next() {
        //     ss.push(format!("{exp} {ctx:?}").to_uppercase());
        // }
        // s.push_str(ss.join(" <- ").as_str());
        // s.push_str(format!("{ResetColor}\n").as_str());
        // eprintln!("{s}");

        self.lhs_stack.extend(exps);
    }

    pub fn pop_left(&mut self) -> Option<(Exp, Ctx)> {
        self.lhs_stack.pop().inspect(|(exp, ctx)| {
            // out(
            //     0,
            //     0,
            //     format!(
            //         "{IntenseBlue}POP -> {}{ResetColor}\n",
            //         format!("{exp} {ctx:?}").to_uppercase()
            //     ),
            // );
        })
    }

    pub fn take_left(&mut self, n: usize) -> Vec<(Exp, Ctx)> {
        let mut exps = Vec::<(Exp, Ctx)>::new();

        for _ in 0..n {
            if let Some((exp, ctx)) = self.lhs_stack.pop() {
                exps.push((exp, ctx));
            }
        }
        exps.reverse();
        exps
    }

    pub fn push_right(&mut self, (exp, ctx): (Exp, Ctx)) {
        self.rhs_stack.push((exp, ctx));
    }

    pub fn extend_right(&mut self, mut exps: Vec<(Exp, Ctx)>) {
        self.rhs_stack.extend(exps);
    }

    pub fn pop_right(&mut self) -> Option<(Exp, Ctx)> {
        self.rhs_stack.pop().inspect(|(exp, ctx)| {
            // eprintln!(
            //     "{IntenseBlue}POP -> {}{ResetColor}\n",
            //     format!("{exp} {ctx:?}").to_uppercase()
            // )
        })
    }

    pub fn take_right(&mut self, n: usize) -> Vec<(Exp, Ctx)> {
        let mut exps = Vec::<(Exp, Ctx)>::new();

        for _ in 0..n {
            if let Some(exp) = self.rhs_stack.pop() {
                exps.push(exp);
            }
        }
        exps.reverse();
        exps
    }

    pub fn append_child(&mut self, parent: Ctx) -> Ctx {
        if matches!(parent, Ctx::Root) {
            self.contexts.insert(Ctx::Root);
            self.scope_types_mut().insert(parent, ScopeType::None);
            self.set_register(parent, Register::Reg(4));
            self.set_lengths(parent, vec![]);
            self.pcs_mut().insert(parent, Vec::<Pc>::new());
            self.velocities.insert(parent, vec![Velocity(63 as u8)]);
            self.bpms.insert(parent, Bpm(Absolute::UInt(120)));
            self.programs.insert(parent, Prog(0));
            self.tempos.insert(parent, Mpb(500_000));
            self.children.insert(parent, Vec::<Ctx>::new());
        }
        self.current_context_id += 1;
        let id = self.current_context_id;
        let ctx = Ctx::Id(id);
        out(
            0,
            0,
            format!("\x1b[0;32mAPPEND CHILD {parent:?} -> {ctx:?}\x1b[0m\n"),
        );
        let scope = self.scope_type(parent);
        match scope {
            ScopeType::Sequence => {
                let lengths = self.lengths(parent);
                self.set_lengths(ctx, lengths);
            }
            ScopeType::Stack => {
                let length = self.lengths(parent).iter().cloned().min();
                if let Some(length) = length {
                    self.set_lengths(ctx, vec![length]);
                }
            }
            _ => {}
        }

        let pcs = self.pcs(parent);
        let reg = self.register(parent);
        let velocity = self
            .velocities
            .get(&parent)
            .unwrap_or(&vec![Velocity(63)])
            .clone();

        let prog = if self.programs.get(&parent).is_some() {
            std::mem::take(&mut self.programs.get(&parent).unwrap().clone())
        } else {
            Prog(0)
        };

        let bpm = self
            .bpms_mut()
            .get(&parent)
            .unwrap_or(&Bpm(Absolute::UInt(120)))
            .clone();

        let tempo = if self.tempos.get(&parent).is_some() {
            std::mem::take(&mut self.tempos.get(&parent).unwrap().clone())
        } else {
            Mpb::default()
        };

        let mut lengths = self.lengths(parent).clone();

        self.contexts_mut().insert(ctx);
        self.parents_mut().insert(ctx, parent);
        self.children_mut().insert(ctx, Vec::<Ctx>::new());
        if let Some(children) = self.children.get_mut(&parent) {
            children.push(ctx);
        };
        self.scope_types_mut().insert(ctx, ScopeType::None);
        self.lengths_mut(ctx).append(&mut lengths);
        self.pcs_mut().insert(ctx, pcs);
        self.bpms_mut().insert(ctx, bpm);
        self.registers_mut().insert(ctx, reg);
        self.velocities_mut().insert(ctx, velocity);
        self.tempos_mut().insert(ctx, tempo);
        self.programs_mut().insert(ctx, prog);

        ctx
    }

    pub fn empty_child(&mut self, parent: Ctx) -> Ctx {
        self.current_context_id += 1;
        let id = self.current_context_id;
        let ctx = Ctx::Id(id);
        // out(
        //     0,
        //     0,
        //     format!("\x1b[0;32mAPPEND CHILD {parent:?} -> {ctx:?}\x1b[0m\n"),
        // );
        self.contexts.insert(ctx);
        self.scope_types.insert(ctx, ScopeType::None);
        self.parents.insert(ctx, parent);
        self.children.get_mut(&parent).unwrap().push(ctx);
        self.children.insert(ctx, Vec::<Ctx>::new());
        self.registers.insert(ctx, Register::None);
        self.lengths.insert(ctx, Vec::<Length>::new());
        self.pcs.insert(ctx, Vec::<Pc>::new());
        self.velocities.insert(ctx, Vec::<Velocity>::new());
        self.bpms.insert(ctx, Bpm(Absolute::UInt(0)));
        self.tempos.insert(ctx, Mpb(0));
        self.programs.insert(ctx, Prog(0));

        ctx
    }

    pub fn move_child(&mut self, child: Ctx, to: Ctx) {
        let parent = self.parent(child);
        // out(format!(
        //     "\x1b[0;35m{}\x1b[0m",
        //     format!("MOVE {parent:?} -> {child:?} => {to:?} -> {child:?}\n").to_uppercase()
        // ));
        let parent = self.parent(child);
        let mut siblings = self.children.get_mut(&parent).unwrap();
        *siblings = siblings
            .iter()
            .cloned()
            .filter(|sibling| *sibling != child)
            .collect::<Vec<Ctx>>();
        self.parents_mut().insert(child, to);
        self.children.get_mut(&to).unwrap().push(child);
        // print_state(self, child);
    }

    pub fn drop(&mut self, child: Ctx) {
        // out(format!("{}", format!("\x1b[0;31mDROP {child:?}\x1b[0m\n")));
        let index = child.to_usize();
        self.scope_types.remove(&child);
        self.lengths.remove(&child);
        self.pcs.remove(&child);
        self.registers.remove(&child);
        self.bpms.remove(&child);
        self.tempos.remove(&child);
        self.programs.remove(&child);
        self.velocities.remove(&child);

        let parent = self.parent(child);
        let mut siblings = std::mem::take(&mut self.children.get(&parent).unwrap().clone());

        siblings = siblings
            .into_iter()
            .filter(|c| c.to_usize() != child.to_usize())
            .collect();
        self.contexts.remove(&child);
        self.parents.remove(&child);
        *self.children.get_mut(&parent).unwrap() = siblings;
    }

    pub fn inbox(&self) -> &[(Exp, Ctx)] {
        &self.rhs_stack
    }

    pub fn lengths(&self, ctx: Ctx) -> Vec<Length> {
        if let Some(lengths) = self.lengths.get(&ctx) {
            lengths.clone()
        } else {
            Vec::<Length>::new()
        }
    }

    fn parents_mut(&mut self) -> &mut HashMap<Ctx, Ctx> {
        &mut self.parents
    }

    fn scope_types_mut(&mut self) -> &mut HashMap<Ctx, ScopeType> {
        &mut self.scope_types
    }

    pub fn scope_type(&self, ctx: Ctx) -> ScopeType {
        if let Some(scope) = self.scope_types.get(&ctx) {
            scope.clone()
        } else {
            ScopeType::None
        }
    }

    fn pcs_mut(&mut self) -> &mut HashMap<Ctx, Vec<Pc>> {
        &mut self.pcs
    }

    pub fn pcs(&self, ctx: Ctx) -> Vec<Pc> {
        if let Some(pc) = self.pcs.get(&ctx) {
            pc.clone()
        } else {
            vec![]
        }
    }

    pub fn set_pcs(&mut self, ctx: Ctx, pcs: Vec<Pc>) {
        *self.pcs.get_mut(&ctx).unwrap() = pcs;
    }

    pub fn get_context(&mut self, mut ctx: Ctx) -> Context {
        if ctx == Ctx::Id(0) {
            ctx = Ctx::Root;
        }
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
        self.tempos[&ctx]
    }

    fn set_tempo(&mut self, ctx: Ctx, tempo: Mpb) {
        self.tempos.insert(ctx, tempo);
    }

    fn bpms_mut(&mut self) -> &mut HashMap<Ctx, Bpm> {
        &mut self.bpms
    }

    pub fn bpm(&self, ctx: Ctx) -> Bpm {
        self.bpms
            .get(&ctx)
            .unwrap_or(&Bpm(Absolute::UInt(120)))
            .clone()
    }

    pub fn set_bpm(&mut self, ctx: Ctx, bpm: Bpm) {
        self.bpms_mut().insert(ctx, bpm);
    }

    fn registers_mut(&mut self) -> &mut HashMap<Ctx, Register> {
        &mut self.registers
    }

    fn set_register(&mut self, mut ctx: Ctx, register: Register) {
        self.registers_mut().insert(ctx, register);
    }

    fn velocities_mut(&mut self) -> &mut HashMap<Ctx, Vec<Velocity>> {
        &mut self.velocities
    }

    pub fn velocities(&self, ctx: Ctx) -> Vec<Velocity> {
        self.velocities
            .get(&ctx)
            .unwrap_or(&vec![Velocity::default()])
            .clone()
            .clone()
    }

    pub fn set_velocities(&mut self, ctx: Ctx, velocities: Vec<Velocity>) {
        self.velocities_mut().insert(ctx, velocities);
    }

    fn programs_mut(&mut self) -> &mut HashMap<Ctx, Prog> {
        &mut self.programs
    }

    pub fn program(&self, ctx: Ctx) -> Prog {
        self.programs.get(&ctx).unwrap().clone()
    }

    pub fn set_program(&mut self, ctx: Ctx, program: Prog) {
        self.programs_mut().insert(ctx, program);
    }

    pub fn children_mut(&mut self) -> &mut BTreeMap<Ctx, Vec<Ctx>> {
        &mut self.children
    }

    fn contexts_mut(&mut self) -> &mut BTreeSet<Ctx> {
        &mut self.contexts
    }

    pub fn contexts(&self) -> &BTreeSet<Ctx> {
        &self.contexts
    }

    fn set_lengths(&mut self, ctx: Ctx, lengths: Vec<Length>) {
        self.lengths.insert(ctx, lengths);
    }

    pub fn lengths_mut(&mut self, ctx: Ctx) -> &mut Vec<Length> {
        if self.lengths.contains_key(&ctx) {
            self.lengths.get_mut(&ctx).unwrap()
        } else {
            self.lengths.insert(ctx, Vec::<Length>::new());
            self.lengths.get_mut(&ctx).unwrap()
        }
    }

    fn set_scope_type(&mut self, ctx: Ctx, scope_type: ScopeType) {
        // eprintln!("\x1b[032mSET {ctx:?} TO SCOPE TYPE {scope_type:?}\x1b[0m\n");
        self.scope_types.insert(ctx, scope_type);
        // print_state(self, ctx);
    }

    pub fn tempos_mut(&mut self) -> &mut HashMap<Ctx, Mpb> {
        &mut self.tempos
    }

    pub fn parent(&self, ctx: Ctx) -> Ctx {
        if let Some(ctx) = self.parents.get(&ctx) {
            *ctx
        } else {
            Ctx::None
        }
    }

    pub fn binding(&self, ctx: Ctx) -> &HashMap<Ident, Exp> {
        &self.bindings.get(&ctx).unwrap()
    }

    pub fn add_binding(&mut self, ctx: Ctx, ident: Ident, binding: Exp) {
        self.bindings
            .get(&ctx)
            .unwrap()
            .clone()
            .insert(ident, binding);
    }

    pub fn children(&self, ctx: Ctx) -> Vec<Ctx> {
        if self.contexts.contains(&ctx) {
            self.children.get(&ctx).unwrap().clone()
        } else {
            Vec::<Ctx>::new()
        }
    }

    pub fn tempos(&self) -> &HashMap<Ctx, Mpb> {
        &self.tempos
    }

    pub fn register(&self, ctx: Ctx) -> Register {
        match self.registers.get(&ctx) {
            Some(reg) => *reg,
            None => Register::None,
        }
    }

    pub fn discard(&mut self, ctx: Ctx) {
        // out(0, 0, format!("\x1b[0;31mDISCARD {ctx:?}\x1b[0m"));
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
    terminal::disable_raw_mode();
    let mut state = State::default();
    // let ctx = state.append_child(Ctx::Root);
    // state.set_current_context(ctx);
    let mut exps = ast.exps;
    exps.push(Exp::EOS);
    exps.reverse();
    let exps: Vec<(Exp, Ctx)> = exps.into_iter().map(|exp| (exp, Ctx::None)).collect();
    state.rhs_stack.extend(exps);

    let mut rhs_stack = state.rhs_stack.iter().rev();
    let mut lhs_stack = state.lhs_stack.iter().rev();
    // execute!(stderr(), cursor::Show);
    let mut m = Monad::ret((Exp::Noop, Ctx::Root));

    while let Some((rhs, rhs_ctx)) = state.rhs_stack.pop() {
        m = m.bind(Box::new(|(lhs, lhs_ctx)| {
            combine((lhs, lhs_ctx), (rhs, rhs_ctx), &mut state)
        }));
    }
    // let mut children = vec![Ctx::Root];

    // while children.len() > 0 {
    //     children = children
    //         .iter()
    //         .fold(Vec::<Ctx>::new(), |mut children_, ctx| {
    //             print_state(&state, *ctx);
    //             children_.extend(state.children(*ctx));

    //             children_
    //         });
    // }

    state.collect_garbage();
    // dbg!(&state);
    // graph(&mut state, Ctx::Id(0));
    // execute!(stderr(), cursor::Show);
    state
}

fn combine(lhs: (Exp, Ctx), rhs: (Exp, Ctx), state: &mut State) -> Monad<(Exp, Ctx)> {
    // execute!(stderr(), cursor::MoveTo(0, 0));
    let ((lhs, lhs_ctx), (rhs, rhs_ctx)) = (lhs.clone(), rhs.clone());
    let parent = state.parent(lhs_ctx);

    comment(
        format!(
            "combine {:?} {lhs_ctx:?} {:?} {rhs_ctx:?}",
            state.scope_type(lhs_ctx),
            state.scope_type(rhs_ctx)
        )
        .as_str(),
    );

    graph(state, Ctx::Root, 5);
    // execute!(
    //     stderr(),
    //     cursor::MoveTo(0, 25),
    //     terminal::Clear(ClearType::CurrentLine)
    // );

    print_exps(
        &(lhs.clone(), lhs_ctx),
        &(rhs.clone(), rhs_ctx),
        state,
        &state.lhs_stack,
        &state.rhs_stack,
    );

    // execute!(stderr(), RestorePosition);

    let res = match ((lhs, lhs_ctx), (rhs.clone(), rhs_ctx)) {
        (lhs @ (Exp::Noop, Ctx::None), rhs @ (Exp::Noop, Ctx::None)) => {
            if let Some((lhs, lhs_ctx)) = state.pop_right() {
                // dbg!((&lhs, lhs_ctx));
                if lhs_ctx != Ctx::None {
                    print_state(state, lhs_ctx);
                    // state.set_scope_type(Ctx::Root, ScopeType::Sequence);
                    consume_simples(state, Monad::ret((lhs, lhs_ctx))).bind(Box::new(|_| {
                        sequence_children(Ctx::Root, state);
                        Monad::ret(NOOP)
                    }))
                } else {
                    state.push_left((lhs, lhs_ctx));
                    Monad::ret(NOOP)
                }
            } else {
                Monad::ret(NOOP)
            }
        }
        ((lhs, _), (Exp::EOS, _)) => match lhs {
            Exp::Compound(compound) => consume_compound(*compound, state, lhs_ctx),
            Exp::Noop => {
                if let Some((lhs, lhs_ctx)) = state.pop_left() {
                    match lhs {
                        lhs @ Exp::Compound(_) => {
                            combine((lhs, lhs_ctx), (Exp::EOS, Ctx::None), state)
                        }
                        lhs => {
                            state.push_left((lhs, lhs_ctx));
                            combine((Exp::Noop, lhs_ctx), NOOP, state)
                        }
                    }
                } else {
                    Monad::ret(NOOP)
                }
            }
            _ => Monad::ret(NOOP),
        },
        ((lhs @ Exp::Noop, lhs_ctx_), (rhs, rhs_ctx_)) => match (lhs_ctx_, rhs, rhs_ctx_) {
            (lhs_ctx, ref rhs @ Exp::Compound(ref compound), mut rhs_ctx) => {
                let lhs_parent = state.parent(lhs_ctx);
                let rhs_parent = state.parent(rhs_ctx);

                let rhs_ctx_scope = state.scope_type(rhs_ctx);

                let rhs_scope = match **compound {
                    Compound::Parens(_) => ScopeType::Sequence,
                    Compound::Braces(_) => ScopeType::Stack,
                    _ => ScopeType::None,
                };

                if rhs_ctx_ == Ctx::None {
                    rhs_ctx = state.append_child(lhs_ctx);

                    state.set_scope_type(rhs_ctx, rhs_scope);
                } else if rhs_ctx_scope != rhs_scope {
                    rhs_ctx = state.append_child(rhs_ctx);
                    state.set_scope_type(rhs_ctx, rhs_scope);
                }

                combine((rhs.clone(), rhs_ctx), NOOP, state)
            }
            (lhs_ctx, rhs @ Exp::Simple(Simple::Prefix(_)), rhs_ctx) => match (lhs_ctx, rhs_ctx) {
                (Ctx::Root, Ctx::None) => combine((rhs, lhs_ctx), NOOP, state),
                (_, rhs_ctx) => combine((rhs, rhs_ctx), NOOP, state),
            },
            (lhs_ctx, rhs @ Exp::Noop, rhs_ctx) => {
                if let Some((rhs, rhs_ctx)) = state.rhs_stack.pop() {
                    combine((Exp::Noop, lhs_ctx), (rhs, rhs_ctx), state)
                } else {
                    while let Some(lhs) = state.lhs_stack.pop() {
                        if !matches!(lhs, (Exp::Compound(_), _)) {
                            state.push_left(lhs);
                            break;
                        }
                        state.push_right(lhs);
                    }
                    if let Some((Exp::Compound(compound), lhs_ctx)) = state.rhs_stack.pop() {
                        consume_compound(*compound, state, lhs_ctx)
                    } else {
                        while let Some(lhs @ (Exp::Simple(_), lhs_ctx)) = state.lhs_stack.pop() {
                            state.rhs_stack.push(lhs);
                        }

                        combine(NOOP, NOOP, state)
                    }
                }
            }
            (lhs_ctx, rhs, _) => combine((rhs, lhs_ctx), NOOP, state),
        },
        ((lhs, lhs_ctx), (rhs @ Exp::Noop, rhs_ctx)) => match lhs {
            ref lhs @ Exp::Compound(ref compound) => {
                consume_right_assoc_exps(*compound.clone(), state, lhs_ctx)
                    .bind(Box::new(|lhs| Monad::ret(lhs)))
                    .bind(Box::new(|(lhs, lhs_ctx)| {
                        if let Exp::Compound(compound) = lhs {
                            consume_compound(*compound, state, lhs_ctx)
                        } else {
                            Monad::ret((lhs, lhs_ctx))
                        }
                    }))
            }
            lhs => {
                state.push_left((lhs, lhs_ctx));
                combine((Exp::Noop, lhs_ctx), NOOP, state)
            }
        },
        ((Exp::Compound(lhs), _), (Exp::Compound(rhs), _)) => {
            match (*lhs.to_owned(), *rhs.to_owned()) {
                (Compound::Parens(lhs_exps), Compound::Parens(rhs_exps)) => {
                    if matches!(state.scope_type(parent), ScopeType::Stack) {
                        state.extend_left(vec![
                            (Exp::Compound(Box::new(Compound::Parens(lhs_exps))), lhs_ctx),
                            (Exp::Compound(Box::new(Compound::Parens(rhs_exps))), rhs_ctx),
                        ]);
                        // print_exps(&NOOP, &NOOP, state, &state.lhs_stack, &state.rhs_stack);
                        consume_sequences(parent, state)
                    } else {
                        consume_compound(Compound::Parens(lhs_exps), state, lhs_ctx)
                            .bind(Box::new(|(lhs, lhs_exp)| Monad::ret((Exp::Noop, lhs_ctx))))
                    }
                }
                (lhs @ Compound::Braces(_), rhs @ Compound::Braces(_)) => {
                    let parent_scope = state.scope_type(parent);

                    match parent_scope {
                        ScopeType::Stack => {
                            state
                                .rhs_stack
                                .push((Exp::Compound(Box::new(rhs)), rhs_ctx));
                            consume_compound(lhs, state, lhs_ctx)
                            // .bind(Box::new(|_lhs| {
                            // combine(NOOP, (Exp::Compound(Box::new(rhs.clone())), rhs_ctx), state)
                            // }))
                        }
                        ScopeType::Sequence | _ => {
                            state
                                .rhs_stack
                                .push((Exp::Compound(Box::new(rhs)), rhs_ctx));
                            consume_compound(lhs, state, lhs_ctx)
                            // .bind(Box::new(|_lhs| {
                            // combine(NOOP, (Exp::Compound(Box::new(rhs.clone())), rhs_ctx), state)
                            // }))
                        }
                    }
                }
                (mut lhs @ Compound::Parens(_), rhs @ Compound::Braces(_)) => {
                    let parent_scope = state.scope_type(parent);
                    match parent_scope {
                        ScopeType::Stack => consume_compound(lhs, state, lhs_ctx),
                        ScopeType::Sequence | _ => consume_compound(lhs, state, lhs_ctx),
                    }
                }
                (lhs @ Compound::Braces(_), rhs @ Compound::Parens(_)) => {
                    let parent_scope = state.scope_type(parent);

                    match parent_scope {
                        ScopeType::Stack => consume_compound(lhs, state, lhs_ctx),
                        ScopeType::Sequence | _ => consume_compound(lhs, state, lhs_ctx),
                    }
                }
                (lhs, rhs) => {
                    todo!()
                }
                _ => todo!(),
            }
        }
        ((lhs @ Exp::Compound(_), _), (Exp::Simple(simple), _)) => {
            if matches!(simple, Simple::Suffix(_)) {
                compose_simple(
                    Monad::ret(simple),
                    Monad::ret((lhs, lhs_ctx)),
                    state,
                    rhs_ctx,
                )
            } else {
                // dbg!();
                // eprintln!("{lhs} {lhs_ctx:?}");
                // eprintln!("{rhs} {rhs_ctx:?}");
                state.push_left((lhs.clone(), lhs_ctx));

                let ctx = state.parent(lhs_ctx);

                combine((Exp::Simple(simple), lhs_ctx), NOOP, state)
            }
        }
        ((Exp::Simple(simple), _), (rhs @ Exp::Compound(_), _)) => match simple {
            Simple::Scalar(Scalar::Duration(duration)) => compose_duration(
                Monad::ret(duration),
                Monad::ret((rhs, rhs_ctx)),
                state,
                rhs_ctx,
            ),
            simple => compose_simple(
                Monad::ret(simple),
                Monad::ret((rhs, rhs_ctx)),
                state,
                rhs_ctx,
            ),
        },
        ((ref lhs @ Exp::Simple(ref s1), _), (ref rhs @ Exp::Simple(ref s2), _)) => {
            match (s1, s2) {
                (Simple::Prefix(prefix), Simple::Scalar(Scalar::Pure(_))) => match prefix {
                    prefix @ Prefix::Dur => compose_prefix(
                        Monad::ret(prefix.clone()),
                        Monad::ret((rhs.clone(), rhs_ctx)),
                        state,
                        lhs_ctx,
                    ),
                    prefix @ Prefix::Pc => compose_prefix(
                        Monad::ret(*prefix),
                        Monad::ret((rhs.clone(), rhs_ctx)),
                        state,
                        lhs_ctx,
                    ),
                    prefix => {
                        // dbg!();
                        compose_prefix(
                            Monad::ret(*prefix),
                            Monad::ret((rhs.clone(), rhs_ctx)),
                            state,
                            lhs_ctx,
                        )
                    }
                },
                (Simple::Infix(infix), _) => compose_infix(
                    Monad::ret(*infix),
                    Monad::ret((rhs.clone(), rhs_ctx)),
                    state,
                    lhs_ctx,
                ),
                (Simple::Scalar(Scalar::Duration(duration)), rhs) => compose_duration(
                    Monad::ret(duration.clone()),
                    Monad::ret((Exp::Simple(rhs.clone()), rhs_ctx)),
                    state,
                    lhs_ctx,
                ),
                (simple, rhs) => {
                    state.push_left((Exp::Simple(simple.clone()), lhs_ctx));
                    combine(
                        (Exp::Noop, lhs_ctx),
                        (Exp::Simple(rhs.clone()), rhs_ctx),
                        state,
                    )
                }
            }
        }
        ((lhs, _), (rhs, _)) => {
            if state.lhs_stack.len() > 0 {
                // out(format!(
                //     "\x1b[0;33mStack:\n{}\n\n\x1b[0m",
                //     align(&state.lhs_stack, 0, 80)
                // ));
            }

            todo!()
        }
    };

    res
}

fn consume_simples(state: &mut State, mut m: Monad<(Exp, Ctx)>) -> Monad<(Exp, Ctx)> {
    if let Some((rhs, rhs_ctx)) = state.rhs_stack.pop() {
        // dbg!((&rhs, rhs_ctx));
        m = m.bind(Box::new(|lhs: (Exp, Ctx)| {
            out(
                0,
                0,
                format!(
                    "{IntenseGreen}CONSUME {} {}{ResetColor}",
                    lhs.0.to_string().to_uppercase(),
                    rhs.to_string().to_uppercase()
                ),
            );
            // dbg!((&lhs, (&rhs, rhs_ctx)));
            match (lhs, (rhs, rhs_ctx)) {
                ((Exp::Noop, _), (rhs, rhs_ctx)) => {
                    consume_simples(state, Monad::ret((rhs, rhs_ctx)))
                }
                ((Exp::Simple(simple), lhs_ctx), rhs) => {
                    compose_simple(Monad::ret(simple), Monad::ret(rhs), state, lhs_ctx).bind(
                        Box::new(|(lhs, lhs_ctx)| {
                            dbg!();
                            print_state(state, lhs_ctx);
                            consume_simples(state, Monad::ret((lhs, lhs_ctx)))
                        }),
                    )
                }

                ((lhs, lhs_ctx), rhs) => {
                    combine((lhs, lhs_ctx), rhs, state).bind(Box::new(|(lhs, lhs_ctx)| {
                        consume_simples(state, Monad::ret((lhs, lhs_ctx)))
                    }))
                }
            }
        }));
        // dbg!(&m);
        m
    } else {
        m.bind(Box::new(|(_, lhs_ctx)| Monad::ret((Exp::Noop, lhs_ctx))))
    }
}

fn sequence_children(parent: Ctx, state: &mut State) {
    out(
        0,
        36,
        format!(
            "{BoldPurple}SEQUENCE CHILDREN {}{ResetColor}",
            format!("{parent:?} {:?}", state.scope_type(parent)).to_uppercase()
        ),
    );
    print_state(state, parent);
    let mut children = state.children(parent).clone();

    if children.is_empty() {
        dbg!();
        // print_state(state, parent);
        return;
    }

    // let mut children_ = children.clone();
    // while children_.len() > 0 {
    //     // children_.iter().cloned().for_each(|ctx| {
    //     //     // print_state(state, ctx);
    //     // });

    //     children_ = children_
    //         .iter()
    //         .cloned()
    //         .flat_map(|ctx| state.children(ctx))
    //         .collect();
    // }
    dbg!(state.scope_type(parent));
    match state.scope_type(parent) {
        ScopeType::None => {
            // dbg!();
            children.iter().cloned().for_each(|ctx| {
                sequence_children(ctx, state);
            });
            return;
        }
        ScopeType::Sequence => {
            children.iter().cloned().for_each(|ctx| {
                sequence_children(ctx, state);
                return;
            });
        }
        ScopeType::Stack => {
            if children.len() > 0 {
                let mut iter = children.iter().cloned();
                while let Some(ctx) = iter.next() {
                    print_state(state, ctx);
                    dbg!(state.scope_type(ctx));
                    match state.scope_type(ctx) {
                        ScopeType::Sequence => {
                            fit(Ctx::None, ctx, state);
                            let mut ctxs = vec![ctx];
                            while let Some(rhs_ctx) = iter.next() {
                                if matches!(state.scope_type(rhs_ctx), ScopeType::Sequence) {
                                    if let Some(lhs_ctx) = ctxs.pop() {
                                        fit(lhs_ctx, rhs_ctx, state);
                                        print_state(state, rhs_ctx);
                                        ctxs.extend(vec![lhs_ctx, rhs_ctx]);
                                    }
                                }
                            }

                            // dbg!(&ctxs);
                            let lengths = state.lengths(ctxs[0]);
                            let len = lengths.len();
                            let end = state.lengths(ctxs[0]).iter().cloned().sum::<Length>();

                            let mut playhead = Length::MicroSeconds(0);

                            let mut ctxs: Vec<(Ctx, Length)> = ctxs
                                .into_iter()
                                .map(|ctx| (ctx, Length::MicroSeconds(0)))
                                .collect();

                            let mut iter =
                                children.iter().cloned().flat_map(|ctx| state.lengths(ctx));
                            let init = iter.next().unwrap();

                            let step = iter.fold(init, |l1, l2| gcd(l1, l2.clone()));

                            // comment(
                            //     format!(
                            //         "{}",
                            //         // IntenseRed,
                            //         format!("sequencing {ctxs:?}").to_uppercase().as_str(),
                            //         // ResetColor
                            //     )
                            //     .as_str(),
                            // );
                            while playhead < end {
                                let mut ctx = Ctx::None;

                                ctxs.iter_mut()
                                    .enumerate()
                                    .for_each(|(idx, (ctx_, counter))| {
                                        // print_state(state, *ctx_);

                                        if *counter == playhead && state.lengths(*ctx_).len() > 0 {
                                            out(
                                                0,
                                                0,
                                                format!(
                                                    "{IntenseGreen}T: {}:{}{ResetColor}",
                                                    playhead.as_u64(),
                                                    end.as_u64()
                                                ),
                                            );

                                            if matches!(ctx, Ctx::None) {
                                                ctx = state.append_child(parent);
                                                state.set_scope_type(ctx, ScopeType::Stack);
                                            }
                                            out(
                                                0,
                                                0,
                                                format!(
                                                    "{IntenseYellow}C: {}{ResetColor}",
                                                    counter.as_u64()
                                                ),
                                            );
                                            let ctx__ = state.empty_child(ctx);
                                            state.set_scope_type(ctx__, ScopeType::Stack);

                                            if state.children(*ctx_).is_empty() {
                                                dbg!();
                                                print_state(state, *ctx_);
                                                let length = take_note(state, *ctx_, ctx__);
                                                *counter += length;
                                                dbg!();
                                                print_state(state, *ctx_);
                                                print_state(state, ctx__);
                                            } else {
                                                dbg!();
                                                print_state(state, *ctx_);

                                                if let Some(children) =
                                                    state.children_mut().get_mut(ctx_)
                                                {
                                                    if let Some(ctx_) = children.pop() {
                                                        let length = take_note(state, ctx_, ctx__);

                                                        *counter += length;

                                                        dbg!();

                                                        print_state(state, ctx_);
                                                        print_state(state, ctx__);
                                                    }
                                                }
                                                // graph(state, *ctx_, 5);

                                                // let length = children.iter().cloned().fold(
                                                //     Length::default(),
                                                //     |length, from| {
                                                //         let ctx__ = state.empty_child(ctx);
                                                //         state.set_scope_type(
                                                //             ctx__,
                                                //             ScopeType::Stack,
                                                //         );
                                                //         let length_ = take_note(state, from, ctx__);
                                                //         *counter += length_;
                                                //         dbg!();
                                                //         print_state(state, from);
                                                //         print_state(state, ctx__);
                                                //         length + length_
                                                //     },
                                                // );

                                                // *counter += length;
                                            }
                                            // state.children_mut().insert(ctx__, children);

                                            // let pcs = state.pcs(ctx)
                                            // state.lengths_mut(parent).push(length);

                                            // graph(state, parent, 2);
                                            print_state(state, parent);
                                            print_state(state, ctx);
                                            print_state(state, *ctx_);
                                        }
                                    });
                                playhead += step;
                            }

                            state.set_scope_type(parent, ScopeType::Sequence);
                            ctxs.into_iter().for_each(|(ctx, _)| {
                                // if state.children(ctx).len() > 0 {
                                //     sequence_children(ctx, state);
                                // }
                                state.drop(ctx)
                            });
                        }
                        ScopeType::Stack => {
                            sequence_children(ctx, state);
                            let length = get_child_length(state, ctx);
                            state.lengths_mut(parent).push(length);
                            // let mut pc = Pc::None;
                            // if let Some(pcs) = state.pcs_mut().get_mut(&parent) {
                            //     let mut pcs = VecDeque::from_iter(pcs.into_iter());
                            //     if let Some(pc_) = pcs.pop_front() {
                            //         pc = pc_.clone();
                            //     }
                            // }
                            // state.pcs_mut().get_mut(&ctx).unwrap().push(pc);
                            // dbg!(&pc);

                            break;
                        }
                        _ => todo!(),
                    }
                }
            }
        }
        _ => todo!(),
    }
}

fn get_child_length(state: &mut State, ctx: Ctx) -> Length {
    let children = state.children(ctx);
    let mut children_iter = children.iter().cloned();
    children.into_iter().for_each(|ctx_| {
        let length = get_child_length(state, ctx_);
        state.lengths_mut(ctx).push(length);
    });
    let lengths = state.lengths(ctx);
    let mut lengths_iter = lengths.iter().cloned();

    match state.scope_type(ctx) {
        ScopeType::Sequence => state.lengths(ctx).iter().cloned().sum(),
        ScopeType::Stack => {
            let init = lengths_iter.next().unwrap();
            state
                .lengths(ctx)
                .iter()
                .fold(init, |a, b| a.min(b.clone()))
        }
        _ => todo!(),
    }
}

fn take_note(state: &mut State, from: Ctx, to: Ctx) -> Length {
    // print_state(state, from);
    // print_state(state, to);
    let register = state.register(from);
    let tempo = state.tempo(from);
    let program = state.program(from);

    state.set_register(to, register);
    state.set_tempo(to, tempo);
    state.set_program(to, program);

    let mut length = Length::None;

    match state.scope_type(from) {
        ScopeType::Sequence => {
            dbg!(from, to);
            length = state.lengths_mut(from).pop().unwrap_or_default();
            if let Some(pcs) = state.pcs_mut().get_mut(&from) {
                if let Some(pc) = pcs.pop() {
                    state.set_pcs(to, vec![pc]);
                }
            }

            let velocity = state
                .velocities_mut()
                .get_mut(&from)
                .unwrap()
                .pop()
                .unwrap_or_default();

            state.set_lengths(to, vec![length.clone()]);

            state.set_velocities(to, vec![velocity]);
        }
        ScopeType::Stack => {
            let lengths = state.lengths(from);
            let pcs = state.pcs(from);
            let velocities = state.velocities(from);
            state.set_lengths(to, lengths.clone());
            state.set_pcs(to, pcs);
            state.set_velocities(to, velocities);

            length = lengths.iter().cloned().min().unwrap();
        }

        _ => todo!(),
    }

    // print_state(state, from);
    let children = state.children(to);
    // children.iter().for_each(|ctx| print_state(state, *ctx));
    length
}

fn fit(lhs_ctx: Ctx, rhs_ctx: Ctx, state: &mut State) {
    dbg!();
    print_state(state, lhs_ctx);
    print_state(state, rhs_ctx);
    state.children_mut().get_mut(&rhs_ctx).unwrap();
    if matches!(lhs_ctx, Ctx::None) {
        expand_context(rhs_ctx, state);
        return;
    }
    let lhs_lengths = state.lengths_mut(lhs_ctx);
    let lhs_lengths_sum = lhs_lengths.iter().cloned().sum::<Length>().as_usize();
    let rhs_lengths = state.lengths_mut(rhs_ctx);
    rhs_lengths.reverse();
    let rhs_lengths_sum = rhs_lengths.iter().cloned().sum::<Length>().as_usize();
    *rhs_lengths = rhs_lengths
        .iter()
        .cloned()
        .cycle()
        .take(
            f64::round(lhs_lengths_sum as f64 / rhs_lengths_sum as f64 * rhs_lengths.len() as f64)
                as usize,
        )
        .collect();
    let rhs_lengths_len = rhs_lengths.len();
    let rhs_pcs = state.pcs_mut().get_mut(&rhs_ctx).unwrap();
    rhs_pcs.reverse();
    // dbg!(rhs_ctx, &rhs_pcs);
    *rhs_pcs = rhs_pcs
        .iter()
        .cloned()
        .cycle()
        .take(rhs_lengths_len)
        .collect();
    // dbg!(rhs_pcs);
    let rhs_velocities = state.velocities_mut().get_mut(&rhs_ctx).unwrap();
    rhs_velocities.reverse();
    *rhs_velocities = rhs_velocities
        .iter()
        .cloned()
        .cycle()
        .take(rhs_lengths_len)
        .collect();

    let lengths = state.lengths(rhs_ctx);
    let rhs_children = state.children(rhs_ctx);

    let len = rhs_children.len();
    lengths
        .iter()
        .zip(rhs_children.iter().cycle())
        .skip(len)
        .for_each(|(length, ctx)| {
            let ctx_ = state.append_child(rhs_ctx);
            let scope = state.scope_type(*ctx);
            let register = state.register(*ctx);
            let mut lengths = state.lengths(*ctx);
            lengths.reverse();
            let tempo = state.tempo(*ctx);
            let mut pcs = state.pcs(*ctx);
            // pcs.reverse();
            let mut velocities = state.velocities(*ctx);
            velocities.reverse();
            // let children = state.children_mut().get_mut(ctx).unwrap();
            // children.reverse();
            let children = state.children(*ctx);

            let program = state.program(*ctx);

            state.set_scope_type(ctx_, scope);
            state.set_register(ctx_, register);
            state.set_lengths(ctx_, lengths);
            state.set_tempo(ctx_, tempo);
            state.set_pcs(ctx_, pcs);
            state.set_velocities(ctx_, velocities);
            state.children_mut().insert(ctx_, children.clone());
            state.set_program(ctx_, program);
        });
}

fn expand_context(ctx: Ctx, state: &mut State) {
    let mut children = state.children(ctx);
    let pcs_len = state.pcs(ctx).len();
    let mut pcs = state.pcs_mut().get_mut(&ctx).unwrap();
    pcs.reverse();
    print_state(state, ctx);
    let m: usize = if children.len() > 0 {
        children
            .iter()
            .for_each(|ctx| sequence_children(*ctx, state));
        children.len()
        // TODO: align child sequence with parent sequence
    } else {
        1
    };

    let lengths = state.lengths_mut(ctx);

    *lengths = lengths
        .iter()
        .cloned()
        .cycle()
        .take((pcs_len * m).max(m))
        .collect();
    lengths.reverse();
    let lengths_sum = lengths.iter().cloned().sum::<Length>().as_usize();
    let velocities = state.velocities_mut().get_mut(&ctx).unwrap();
    *velocities = velocities.iter().cloned().cycle().take(pcs_len).collect();
    velocities.reverse();
    state.children_mut().get_mut(&ctx).unwrap().reverse();
}

// fn drain_stack(rhs: Exp, state: &mut State, mut ctx: Ctx) -> (Monad<Exp>, Ctx) {
//     eprintln!("DRAIN STACK\n");

//     let mut m: Monad<Exp> = Monad::ret(rhs);

//     while let Some((mut lhs_, ctx_)) = state.pop_left() {
//         ctx = ctx_;
//         m = m.bind(Box::new(|(rhs, rhs_ctx)| match lhs_ {
//             Exp::Simple(Simple::Prefix(lhs)) => {
//                 compose_prefix(Monad::ret(lhs), Monad::ret(rhs), state, ctx)
//             }
//             Exp::Simple(Simple::Scalar(Scalar::Duration(duration))) => {
//                 compose_duration(Monad::ret(duration), Monad::ret(rhs), state, ctx)
//             }
//             Exp::Simple(Simple::Scalar(Scalar::Dynamic(dynamic))) => {
//                 compose_dynamic(Monad::ret(dynamic), Monad::ret(rhs), state, ctx)
//             }
//             Exp::Noop => combine(rhs, NOOP, state, ctx),
//             mut lhs @ Exp::Compound(_) => combine(lhs, rhs, state, ctx),
//             _ => {
//                 let (mut m_, ctx_) = drain_stack(lhs_.to_owned(), state, ctx);
//                 m_ = m_.bind(Box::new(|mut lhs| combine(lhs, rhs, state, ctx)));
//                 ctx = ctx_;
//                 m_
//             }
//         }));
//     }

//     state.set_current_context(ctx);
//     (m, ctx)
// }

fn consume_right_assoc_exps(compound: Compound, state: &mut State, ctx: Ctx) -> Monad<(Exp, Ctx)> {
    let mut m = Monad::ret((Exp::Compound(Box::new(compound.clone())), ctx));

    while let Some((lhs, lhs_ctx)) = state.pop_left() {
        if matches!(lhs, Exp::Compound(_)) || lhs_ctx != state.parent(ctx) {
            state.push_left((lhs, lhs_ctx));
            break;
        }
        // comment(format!("consume_right_assoc_exps {ctx:?}").as_str());
        m = m.bind(Box::new(|(rhs, rhs_ctx)| {
            match (lhs, rhs) {
                (rhs_ @ Exp::Simple(Simple::Scalar(Scalar::Pure(_))), rhs @ Exp::Compound(_)) => {
                    if let Some((lhs, lhs_ctx_)) = state.lhs_stack.pop() {
                        combine((lhs, rhs_ctx), (rhs_, rhs_ctx), state).bind(Box::new(
                            |(lhs, lhs_ctx)| {
                                if matches!(lhs, Exp::Noop) {
                                    combine((rhs, rhs_ctx), NOOP, state)
                                } else {
                                    combine((lhs, rhs_ctx), (rhs, rhs_ctx), state)
                                }
                            },
                        ))
                    } else {
                        combine((rhs_, lhs_ctx), (rhs, rhs_ctx), state)
                    }
                }
                // (lhs @ Exp::Compound(_), rhs) => combine((lhs, lhs_ctx), (rhs, rhs_ctx), state),
                (lhs, rhs) => combine((lhs, lhs_ctx), (rhs, rhs_ctx), state),
            }
        }));
    }

    m.bind(Box::new(|(lhs, lhs_ctx): (Exp, Ctx)| {
        // dbg!(&lhs, ctx);

        Monad::ret((lhs, lhs_ctx))
        // }
    }))
}

fn consume_compound(mut compound: Compound, state: &mut State, ctx: Ctx) -> Monad<(Exp, Ctx)> {
    let compound_string =
        format!("{} {:?}", Exp::Compound(Box::new(compound.clone())), ctx).to_uppercase();
    // out(format!("\x1b[1;31mCONSUME {}\x1b[0m\n", compound_string));
    match compound {
        Compound::Parens(ref mut exps)
        | Compound::Braces(ref mut exps)
        | Compound::Brackets(ref mut exps) => {
            exps.reverse();
            let mut exps: Vec<(Exp, Ctx)> =
                exps.into_iter().map(|exp| (exp.clone(), ctx)).collect();
            state.rhs_stack.extend(exps.into_iter());

            let m = combine_subcomponents(state, ctx);

            // out(format!("\x1b[1;31m{compound_string} CONSUMED\x1b[0m\n"));

            m.bind(Box::new(|(lhs, ctx)| {
                // out(format!("{lhs}\n"));
                // Monad::ret((lhs, ctx))
                combine((Exp::Noop, ctx), (NOOP), state)
            }))
        }
        _ => todo!(),
    }
}

fn combine_subcomponents(state: &mut State, ctx: Ctx) -> Monad<(Exp, Ctx)> {
    // dbg!();
    let mut m = Monad::ret((Exp::Noop, ctx));
    if let Some((rhs, rhs_ctx)) = state.rhs_stack.pop() {
        m.bind(Box::new(|(mut lhs, lhs_ctx): (Exp, Ctx)| {
            // let lhs_parent = state.parent(lhs_ctx);
            // let rhs_parent = state.parent(rhs_ctx);
            // dbg!(lhs_parent, lhs_ctx, rhs_parent, rhs_ctx);
            // if rhs_ctx != Ctx::None && lhs_parent != rhs_parent {
            //     state.rhs_stack.push((rhs, rhs_ctx));
            //     combine(NOOP, NOOP, state)
            // } else {
            // dbg!();
            // print_exps(
            //     &(lhs.clone(), lhs_ctx),
            //     &(rhs.clone(), rhs_ctx),
            //     state,
            //     &state.lhs_stack,
            //     &state.rhs_stack,
            // );
            match ((lhs, ctx), (rhs, rhs_ctx)) {
                (lhs, rhs @ (_, Ctx::None)) | (lhs @ (Exp::Noop, _), rhs @ (Exp::Noop, _)) => {
                    state.rhs_stack.push(rhs);
                    // combine((Exp::Noop, ctx), NOOP, state)
                    // combine(lhs, NOOP, state)
                    Monad::ret(lhs)
                }
                (lhs, rhs) => {
                    // comment(format!("{IntenseRed}consume compound {ctx:?}{ResetColor}").as_str());
                    combine((Exp::Noop, ctx), rhs, state)
                }
            }
            // }
        }))
    } else {
        Monad::ret(NOOP)
    }
}

fn consume_sequences(parent: Ctx, state: &mut State) -> Monad<(Exp, Ctx)> {
    // out(format!("\x1b[1;35mCONSUME SEQUENCES\x1b[0m\n"));

    let parent_scope = state.scope_type(parent);
    // out(format!(
    //     "\x1b[0;35mPARENT: {parent_scope:?} {parent:?}\x1b[0m\n"
    // ));
    // out(format!(
    //     "\x1b[0;35mStack length: {}\x1b[0m\n",
    //     state.lhs_stack.len()
    // ));

    if !matches!(parent_scope, ScopeType::Stack) {
        // dbg!();
        Monad::ret(NOOP)
    } else {
        let mut sequences = Vec::<(Vec<Exp>, Ctx)>::new();
        while let Some(((exp @ Exp::Compound(_)), ctx)) = state.lhs_stack.pop() {
            sequences.push((exp_to_exps(exp), ctx));
        }

        if sequences.is_empty() {
            return Monad::ret(NOOP);
        }

        sequences.reverse();

        // out(format!(
        //     "{IntensePurple}sequences:\n{}{ResetColor}\n",
        //     format!("{sequences:?}")
        //         .split_inclusive(|c| matches!(c, ',' | '['))
        //         .collect::<Vec<&str>>()
        //         .join("\n")
        //         .to_string()
        // ));

        merge_sequences(parent, state, sequences);

        combine((Exp::Noop, parent), NOOP, state)
    }
}

fn merge_sequences<'a>(parent: Ctx, state: &'a mut State, mut sequences: Vec<(Vec<Exp>, Ctx)>) {
    // comment("merge_sequences");
    // let mut ctxs = sequences
    //     .into_iter()
    //     .fold(Vec::<Ctx>::new(), |mut ctxs, (seq, mut ctx)| {
    //         state.rhs_stack.extend(
    //             seq.into_iter()
    //                 .rev()
    //                 .map(|exp| (exp, ctx))
    //                 .collect::<Vec<(Exp, Ctx)>>(),
    //         );

    //         ctxs.push(ctx);
    //         let mut ctx_ = ctx;
    //         while ctx_ == ctx {
    //             if let Some((lhs, lhs_ctx)) = state.rhs_stack.pop() {
    //                 if matches!(lhs, Exp::Noop) {
    //                     continue;
    //                 }
    //                 ctx_ = lhs_ctx;
    //                 while let Some((rhs, rhs_ctx)) = state.rhs_stack.pop() {
    //                     if matches!(rhs, Exp::Noop) {
    //                         continue;
    //                     }
    //                     combine((lhs, lhs_ctx), (rhs, rhs_ctx), state);
    //                     break;
    //                 }
    //             }
    //         }

    //         ctxs
    //     });
    // state
    //     .rhs_stack
    //     .extend(sequences.into_iter().flat_map(|(exps, ctx)| {
    //         exps.into_iter()
    //             .map(|exp| (exp, ctx))
    //             .collect::<Vec<(Exp, Ctx)>>()
    //     }));
    let mut ctxs = sequences
        .iter()
        .fold(Vec::<Ctx>::new(), |mut ctxs, (sequence, ctx)| {
            let mut iter = sequence.iter().cloned();
            while let Some(lhs) = iter.next() {
                if matches!(lhs, Exp::Noop) {
                    continue;
                }
                if let Some(rhs) = iter.next() {
                    combine((lhs, *ctx), (rhs, *ctx), state);
                }
            }
            ctxs.push(*ctx);
            ctxs
        });

    if ctxs.is_empty() {
        return;
    }

    // dbg!(&ctxs);

    let pc_count = state.pcs(ctxs[0]).len();
    let lengths: Vec<Length> = state
        .lengths(ctxs[0])
        .iter()
        .cloned()
        .cycle()
        .take(pc_count)
        .collect();

    let seq_len: Length = lengths.iter().cloned().sum();

    let mut expanded_seqs = Vec::<Ctx>::new();

    for ctx_ in ctxs.iter().cloned() {
        // print_state(state, ctx_);
        let ctx_lengths = state.lengths(ctx_);

        let ctx_total_length = ctx_lengths.iter().cloned().sum();
        // dbg!(&ctx_lengths, &ctx_total_length);
        let m = (seq_len.clone() / ctx_total_length).as_usize(); // How many times each sequence fits into the overall sequence
        // dbg!(&m);
        let length_count = ctx_lengths.len();
        // dbg!(length_count);
        let lengths: Vec<Length> = ctx_lengths.iter().cloned().cycle().take(m).collect();
        // dbg!(&lengths);

        let pc_count = state.pcs(ctx_).len();
        let pcs: Vec<Pc> = state.pcs(ctx_).iter().cloned().cycle().take(m).collect();
        // dbg!(&pcs);

        let register = state.register(ctx_);

        let ctx_velocities = state.velocities(ctx_);
        let velocities: Vec<Velocity> = ctx_velocities.iter().cloned().cycle().take(m).collect();

        let program = state.program(ctx_);

        let expanded_ctx = state.append_child(ctx_);
        // print_state(state, expanded_ctx);
        state.set_scope_type(expanded_ctx, ScopeType::Sequence);
        state.move_child(expanded_ctx, parent);
        state.set_lengths(expanded_ctx, lengths);
        state.set_pcs(expanded_ctx, pcs);
        state.set_register(expanded_ctx, register);
        state.set_velocities(expanded_ctx, velocities);
        state.set_program(expanded_ctx, program);

        expanded_seqs.push(expanded_ctx);
        // print_state(state, expanded_ctx);
    }

    let mut sequence_map = BTreeMap::<u64, Ctx>::new();

    for ctx in expanded_seqs.iter().cloned() {
        // print_state(state, ctx);
        let mut t: u64 = 0;
        let lengths = state.lengths(ctx);
        // dbg!(&lengths);
        let pcs = state.pcs(ctx);
        // dbg!(&pcs);
        let velocities = state.velocities(ctx);
        // dbg!(&velocities);
        // dbg!(lengths.len());

        lengths
            .iter()
            .cloned()
            .zip(pcs.iter().cloned())
            .zip(velocities.iter().cloned())
            .for_each(|((length, pc), velocity)| {
                // dbg!(t, &length, &pc);
                if !sequence_map.contains_key(&t) {
                    let outer_stack = add_outer_stack(ctx, parent, state);
                    sequence_map.insert(t, outer_stack);
                }
                let outer_stack = sequence_map[&t];
                let inner_stack = add_inner_stack(outer_stack, state);
                let register = state.register(ctx);

                state.set_pcs(inner_stack, vec![pc]);
                state.set_lengths(inner_stack, vec![length.clone()]);
                state.set_register(inner_stack, register);
                state.set_velocities(inner_stack, vec![velocity]);

                // print_state(state, outer_stack);
                // print_state(state, inner_stack);

                // eprintln!(
                //     "{IntenseYellow}{:?}{ResetColor}",
                //     state.children(outer_stack)
                // );

                t += length.as_u64();
                // dbg!(t);
            });
    }
    expanded_seqs.into_iter().for_each(|ctx| state.discard(ctx));
    ctxs.into_iter().for_each(|ctx| state.discard(ctx));
    sequence_map.iter().for_each(|(t, ctx)| {
        // out(format!(
        //     "{Green}{t}: {IntenseCyan}{ctx:?} {IntenseYellow}{:?}{ResetColor}",
        //     state.children(*ctx)
        // ));
    });

    // dbg!(&sequence_map);

    state
        .children
        .insert(parent, sequence_map.into_values().collect::<Vec<Ctx>>());
    state.set_scope_type(parent, ScopeType::Sequence);
    // print_state(state, parent);
}

// fn print_sequences(sequences: Vec<Exp>) {}

fn add_outer_stack(from: Ctx, to: Ctx, state: &mut State) -> Ctx {
    // comment("add outer stack");
    let ctx = state.append_child(from);
    state.set_scope_type(ctx, ScopeType::Stack);
    state.move_child(ctx, to);
    ctx
}

fn add_inner_stack(parent: Ctx, state: &mut State) -> Ctx {
    // comment("add inner stack to outer stack");
    let ctx = state.append_child(parent);
    state.set_scope_type(ctx, ScopeType::Stack);
    ctx
}

fn exp_to_exps(seq: Exp) -> Vec<Exp> {
    let mut seq = match seq {
        Exp::Compound(compound) => match *compound {
            Compound::Parens(seq) => seq,
            Compound::Braces(stck) => stck,
            _ => todo!(),
        },
        _ => todo!(),
    };
    seq
}

fn compose_simple(
    simple: Monad<Simple>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    simple.bind(Box::new(|simple| match simple {
        Simple::Prefix(prefix) => compose_prefix(Monad::ret(prefix), rhs, state, ctx),
        Simple::Scalar(scalar) => compose_scalar(Monad::ret(scalar), rhs, state, ctx),
        Simple::Infix(infix) => compose_infix(Monad::ret(infix), rhs, state, ctx),
        Simple::Suffix(suffix) => compose_suffix(Monad::ret(suffix), rhs, state, ctx),
        Simple::Ident(ident) => compose_ident(Monad::ret(ident), rhs, state, ctx),
    }))
}

fn compose_decl(
    decl: Monad<Decl>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    decl.bind(Box::new(|Decl { ident, binding }| {
        state.add_binding(ctx, ident, *binding);
        rhs
    }))
}

fn compose_scalar(
    scalar: Monad<Scalar>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    scalar.bind(Box::new(|scalar| match scalar {
        Scalar::Duration(duration) => compose_duration(Monad::ret(duration), rhs, state, ctx),
        Scalar::Dynamic(dynamic) => compose_dynamic(Monad::ret(dynamic), rhs, state, ctx),
        Scalar::Frequency(frequency) => compose_frequency(Monad::ret(frequency), rhs, state, ctx),
        Scalar::Tempo(absolute) => compose_tempo(Monad::ret(absolute), state, ctx),
        Scalar::Pure(pure) => compose_pure(Monad::ret(pure), rhs, state, ctx),
    }))
}

fn compose_tempo(absolute: Monad<Absolute>, state: &mut State, ctx: Ctx) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_dynamic(
    dynamic: Monad<String>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_infix(
    infix: Monad<Infix>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    infix.bind(Box::new(|infix| {
        rhs.bind(Box::new(|(rhs, rhs_ctx)| {
            if let Some((lhs, ctx)) = state.pop_left() {
                match infix {
                    Infix::Mul => match (lhs, rhs) {
                        (
                            Exp::Compound(compound),
                            Exp::Simple(Simple::Scalar(Scalar::Pure(pure))),
                        ) => match pure {
                            Pure::Absolute(abs) => match *compound {
                                Compound::Parens(exps) => {
                                    let len = exps.len();
                                    let exps: Vec<Exp> = exps
                                        .iter()
                                        .cloned()
                                        .cycle()
                                        .take(len * abs.as_usize())
                                        .collect();
                                    combine(
                                        (Exp::Noop, ctx),
                                        (Exp::Compound(Box::new(Compound::Parens(exps))), rhs_ctx),
                                        state,
                                    )
                                }
                                _ => todo!(),
                            },
                            _ => todo!(),
                        },
                        _ => todo!(),
                    },
                    _ => todo!(),
                }
            } else {
                state.push_left((rhs.clone(), ctx));
                // print_exps(&Exp::Simple(Simple::Infix(infix)), &rhs, state, ctx);
                combine((Exp::Simple(Simple::Infix(infix)), ctx), NOOP, state)
            }
        }))
    }))
}

fn compose_ident(
    ident: Monad<Ident>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    ident.bind(Box::new(|ident| {
        Monad::ret((
            state.binding(ctx).get(&ident).unwrap_or(&Exp::Noop).clone(),
            ctx,
        ))
    }))
}

fn compose_ratio(
    abss: Monad<Vec<Absolute>>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_range(
    range: Monad<Range>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_duration(
    duration: Monad<Duration>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    // eprintln!("COMPOSE DURATION\n");

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
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    let res = fractional.bind(|fractional| match fractional {
        fractional @ Fractional::Absolute(abs) => {
            let denominator = abs.as_f64();
            let tempo = state.tempo(ctx);
            let beats = 4 as f64 / denominator;
            let duration_micros = (beats * tempo.0 as f64) as u64;

            rhs.bind(Box::new(|(rhs, rhs_ctx)| {
                let res = {
                    match rhs {
                        Exp::Compound(compound) => {
                            state
                                .lengths_mut(rhs_ctx)
                                .push(Length::MicroSeconds(duration_micros));
                            // print_state(state, ctx);
                            combine((Exp::Compound(compound), ctx), NOOP, state)
                        }

                        ref rhs @ Exp::Simple(ref simple) => {
                            state
                                .lengths_mut(ctx)
                                .push(Length::MicroSeconds(duration_micros));
                            // match simple {
                            //     Simple::Prefix(_) => {
                            //         state.push_left((
                            //             Exp::Simple(Simple::Scalar(Scalar::Duration(
                            //                 Duration::Fractional(fractional.clone()),
                            //             ))),
                            //             ctx,
                            //         ));
                            //         // print_exps(&NOOP, rhs, state, ctx);
                            //         return combine(
                            //             (Exp::Noop, ctx),
                            //             (rhs.clone(), rhs_ctx),
                            //             state,
                            //         );
                            //     }
                            //     _ => todo!(),
                            // }
                            combine((rhs.clone(), rhs_ctx), NOOP, state)
                        }

                        Exp::Noop => Monad::ret((
                            Exp::Simple(Simple::Scalar(Scalar::Duration(Duration::Fractional(
                                fractional,
                            )))),
                            ctx,
                        )),
                        Exp::EOS => todo!(),
                    }
                };
                res
            }))
        }
        Fractional::Tuplet(Tuplet { lhs: num, rhs: den }) => {
            let dur = Monad::ret(Duration::Fractional(Fractional::Absolute(den / num)));
            rhs.bind(Box::new(|(exp, rhs_ctx)| match exp {
                Exp::Simple(simple) => todo!(),
                Exp::Compound(compound) => match *compound {
                    Compound::Parens(exps) => {
                        state.set_scope_type(ctx, ScopeType::Sequence);
                        let mut iter = exps.iter().cloned();
                        let init = compose_duration(
                            dur.clone(),
                            Monad::ret((iter.next().unwrap(), rhs_ctx)),
                            state,
                            ctx,
                        );
                        exps.into_iter().fold(init, |m, rhs| {
                            m.bind(Box::new(|(mut lhs, lhs_ctx)| {
                                combine((lhs, lhs_ctx), (rhs, rhs_ctx), state)
                            }))
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
                                    Monad::ret((
                                        Exp::Compound(Box::new(compound.clone())),
                                        rhs_ctx,
                                    )),
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
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    todo!()
}

fn compose_pure(
    pure: Monad<Pure>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    // match rhs {}
    todo!()
}

fn compose_prefix(
    prefix: Monad<Prefix>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    mut lhs_ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    prefix.bind(Box::new(|prefix| {
        // eprint!(
        //     "{Red}COMPOSE PREFIX {} ∘ ",
        //     format!("{}", Exp::Simple(Simple::Prefix(prefix)).to_string()).to_uppercase()
        // );

        match prefix {
            pc @ Prefix::Pc => rhs.bind(Box::new(|(rhs, rhs_ctx): (Exp, Ctx)| {
                // out(
                //     0,
                //     0,
                //     format!("{} {ResetColor}\n", rhs.to_string().to_uppercase()),
                // );
                match rhs {
                    Exp::Simple(simple) => match simple {
                        Simple::Scalar(scalar) => match scalar {
                            Scalar::Pure(pure) => match pure {
                                Pure::Absolute(abs) => {
                                    state
                                        .pcs_mut()
                                        .get_mut(&lhs_ctx)
                                        .unwrap()
                                        .push(Pc::Class(abs.as_u64() as i8));
                                    Monad::ret((Exp::Noop, lhs_ctx))
                                }
                                Pure::Relative(Relative { sign, val }) => {
                                    let pcs = &mut state.pcs_mut().get_mut(&lhs_ctx).unwrap();
                                    let last = pcs.pop().unwrap_or_default();

                                    match last {
                                        Pc::Class(last) => match sign {
                                            Sign::Plus => pcs.insert(
                                                lhs_ctx.to_usize(),
                                                Pc::Class(last.clone() + val.as_u64() as i8),
                                            ),
                                            Sign::Minus => pcs.insert(
                                                lhs_ctx.to_usize(),
                                                Pc::Class(last.clone() - val.as_u64() as i8),
                                            ),
                                        },
                                        Pc::None => {
                                            let parent =
                                                state.parents_mut().get(&lhs_ctx).unwrap().clone();
                                            let pcs =
                                                &mut state.pcs_mut().get(&parent).unwrap().clone();
                                            let last = pcs.pop().unwrap_or_default();

                                            match last {
                                                Pc::Class(last) => match sign {
                                                    Sign::Plus => pcs.insert(
                                                        lhs_ctx.to_usize(),
                                                        Pc::Class(
                                                            last.clone() + val.as_u64() as i8,
                                                        ),
                                                    ),

                                                    Sign::Minus => {
                                                        pcs.insert(
                                                            lhs_ctx.to_usize(),
                                                            Pc::Class(
                                                                last.clone() + val.as_u64() as i8,
                                                            ),
                                                        );
                                                    }
                                                },
                                                Pc::None => {
                                                    pcs.insert(
                                                        lhs_ctx.to_usize(),
                                                        Pc::Class(val.as_u64() as i8),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    combine((Exp::Noop, lhs_ctx), NOOP, state)
                                }
                            },
                            _ => combine((Exp::Noop, lhs_ctx), NOOP, state),
                        },
                        Simple::Prefix(prefix) => todo!(),

                        Simple::Infix(infix) => match infix {
                            Infix::Colon => todo!(),
                            Infix::Intercalate => todo!(),
                            Infix::Range => todo!(),
                            Infix::Interpolation(interpolation) => todo!(),
                            Infix::Plus => todo!(),
                            Infix::Minus => todo!(),
                            Infix::Mul => todo!(),
                            Infix::Div => todo!(),
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
                            let parent = state.parent(lhs_ctx);

                            let parent_scope = state.scope_type(parent);

                            Monad::ret((exp, lhs_ctx))
                        }
                        Compound::Braces(exps) => {
                            let exp = Exp::Compound(Box::new(Compound::Braces(
                                exps.into_iter()
                                    .flat_map(|exp| vec![Exp::Simple(Simple::Prefix(pc)), exp])
                                    .collect(),
                            )));

                            combine((exp, lhs_ctx), NOOP, state)
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
                }
            })),
            Prefix::Dur => rhs.bind(Box::new(|(rhs, rhs_ctx): (Exp, Ctx)| {
                // eprintln!("{} {ResetColor}\n", rhs.to_string().to_uppercase());
                match rhs {
                    Exp::Simple(simple) => match simple {
                        Simple::Scalar(scalar) => match scalar {
                            Scalar::Pure(pure) => match pure {
                                Pure::Absolute(abs) => Monad::ret((
                                    Exp::Simple(Simple::Scalar(Scalar::Duration(
                                        Duration::Fractional(Fractional::Absolute(abs)),
                                    ))),
                                    rhs_ctx,
                                )),

                                _ => todo!(),
                            },
                            _ => todo!(),
                        },
                        _ => todo!(),
                    },
                    Exp::Noop => combine(
                        (Exp::Noop, lhs_ctx),
                        (Exp::Simple(Simple::Prefix(Prefix::Dur)), rhs_ctx),
                        state,
                    ),
                    rhs => {
                        todo!()
                    }
                }
            })),
            Prefix::Rest => {
                todo!()
            }
            reg @ Prefix::Reg => rhs.bind(Box::new(|(rhs, rhs_ctx): (Exp, Ctx)| {
                // out(
                //     0,
                //     0,
                //     format!("{} {ResetColor}\n", rhs.to_string().to_uppercase()),
                // );
                match rhs {
                    Exp::Simple(simple) => match simple {
                        Simple::Scalar(scalar) => match scalar {
                            Scalar::Pure(pure) => match pure {
                                Pure::Absolute(abs) => {
                                    let register = Register::Reg(abs.as_u64() as i8);
                                    state.set_register(rhs_ctx, register);
                                    Monad::ret((Exp::Noop, rhs_ctx))
                                }
                                Pure::Relative(relative) => todo!(),
                            },
                            Scalar::Duration(duration) => todo!(),
                            Scalar::Tempo(abs) => todo!(),
                            Scalar::Dynamic(string) => todo!(),
                            Scalar::Frequency(frequency) => todo!(),
                        },
                        Simple::Prefix(prefix) => match prefix {
                            prefix @ Prefix::Pc => {
                                Monad::ret((Exp::Simple(Simple::Prefix(prefix)), rhs_ctx))
                            }
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
                            state.set_scope_type(rhs_ctx, ScopeType::Sequence);
                            todo!()
                        }
                        Compound::Braces(exps) => {
                            state.set_scope_type(rhs_ctx, ScopeType::Stack);
                            todo!()
                        }
                        Compound::Brackets(exps) => {
                            state.scope_types_mut().insert(rhs_ctx, ScopeType::Set);
                            todo!()
                        }
                        Compound::Ratio(abss) => {
                            state.scope_types_mut().insert(rhs_ctx, ScopeType::None);
                            todo!()
                        }
                        decl @ Compound::Decl(_) => {
                            Monad::ret((Exp::Compound(Box::new(decl)), rhs_ctx))
                        }
                    },
                    Exp::Noop => return Monad::ret((Exp::Simple(Simple::Prefix(reg)), rhs_ctx)),
                    Exp::EOS => todo!(),
                }
            })),
        }
    }))
}

fn compose_suffix(
    suffix: Monad<Suffix>,
    rhs: Monad<(Exp, Ctx)>,
    state: &mut State,
    ctx: Ctx,
) -> Monad<(Exp, Ctx)> {
    suffix.bind(Box::new(|suffix| match suffix {
        Suffix::Bpm => rhs.bind(Box::new(|(exp, rhs_ctx)| match exp {
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

fn graph_dimensions(
    children: Vec<Ctx>,
    state: &mut State,
    width: usize,
    mut height: usize,
) -> (usize, usize) {
    (
        width.max(children.len()).max(
            children
                .iter()
                .map(|ctx| {
                    let (w, h) = graph_dimensions(state.children(*ctx), state, width, height + 1);
                    height = h;
                    w
                })
                .sum(),
        ),
        height,
    )
}

fn table_index_of(ctx: Ctx, state: &mut State) -> (usize, usize) {
    // dbg!(ctx);
    if ctx == Ctx::None {
        return (0, 0);
    }
    let mut r = 0;
    let mut ctx_ = ctx;
    while !matches!(ctx_, Ctx::None) {
        // dbg!(ctx_);
        r += 1;
        // dbg!(&state);
        ctx_ = state.parent(ctx_);
    }
    let parent = state.parent(ctx);
    let siblings = state.children(parent);
    let c = if let Some(c) = siblings.iter().position(|s| {
        // dbg!(s, ctx, matches!(*s, ctx));
        matches!(*s, ctx)
    }) {
        c
    } else {
        0
    };
    // dbg!(r, c);
    (r, c)
}

fn to_edges(ctx: Ctx, state: &State, depth: usize) -> Vec<(u32, u32)> {
    let mut edges = Vec::<(u32, u32)>::new();
    let mut children = state.children(ctx);

    let mut edges: Vec<(u32, u32)> = edges
        .into_iter()
        .chain(
            repeat_n(
                children
                    .iter()
                    .flat_map(|ctx_| {
                        vec![(ctx.to_u32(), ctx_.to_u32())]
                            .into_iter()
                            .chain(to_edges(*ctx_, state, depth))
                    })
                    .collect::<Vec<(u32, u32)>>(),
                depth,
            )
            .flatten()
            .collect::<Vec<(u32, u32)>>(),
        )
        // .take(tiers)
        .collect();

    edges.sort();

    edges
}

fn graph(state: &mut State, ctx: Ctx, depth: usize) {
    let width = size().unwrap().0;

    let edges = to_edges(ctx, state, depth);
    // dbg!(&edges);
    let layouts = from_edges(
        edges.as_slice(),
        &Config {
            minimum_length: 1,
            vertex_spacing: 1.,
            dummy_vertices: true,
            dummy_size: 0.5,
            ranking_type: RankingType::MinimizeEdgeLength,
            ..Config::default()
        },
    );

    if let Some((layout, width, height)) = layouts.iter().next() {
        // dbg!(width);
        let (c, r) = size().unwrap();
        let (c, r) = (c as usize, r as usize);
        let width = layout
            .iter()
            .max_by(|(_, (x1, _)), (_, (x2, _))| {
                (f64::ceil(*x1) as usize).cmp(&(f64::ceil(*x2) as usize))
            })
            .unwrap()
            .1
            .0;

        let mut columns = f64::ceil(width) as usize;

        columns += 1;

        let column_width = f64::floor(c as f64 / columns as f64) as usize;
        let height = f64::ceil(*height) as usize * 3 + 1;

        let mut table = Vec::<Vec<String>>::from_iter(repeat_n(
            Vec::<String>::from_iter(repeat_n("\u{00A0}".repeat(column_width), columns)),
            height,
        ));

        let mut prev: (usize, (usize, usize)) = (0, (0, 0));
        for (node, (x, y)) in layout {
            let ctx = if *node == 0 {
                Ctx::Root
            } else {
                Ctx::Id(*node)
            };
            // dbg!(node, x, y);
            let mut x_ = x.clone();
            let parent = state.parent(ctx);
            let gp = state.parent(parent);
            let pibling_count = if parent.to_usize() == 0 {
                1
            } else {
                state.children(gp).len()
            };
            let sibling_count = if *node == 0 {
                1
            } else {
                state.children(parent).len()
            };

            let (x, y) = (f64::round(x_) as usize, f64::round(*y) as usize);
            // if Ctx::Id(prev.0) == state.parent(ctx) {
            //     table[y * 2 + 1][x] = format!("{0:^1$}", "|", column_width)
            // }
            // dbg!(x, y);
            let scope = match state.scope_type(ctx) {
                ScopeType::Sequence => "SEQ",
                ScopeType::Stack => "ST",
                _ => " ",
            };
            table[y * 3][x] = format!("{0:^1$}", ctx.to_usize(), column_width);
            table[y * 3 + 1][x] = format!("{0:^1$}", scope, column_width);
            // table[y * 2 + 3][x] = format!("{0:^1$?}", state.scope_type(ctx), column_width);
            // table[y * 2 + 2][x] = format!("{0:^1$}", "|", column_width);
            prev = (*node, (x, y));
        }

        for mut row in &mut table {
            row.reverse();
        }

        if let Ok(mut colonnade) = Colonnade::new(columns, c) {
            colonnade.hyphenate(false);
            colonnade.fixed_width(column_width);
            colonnade.padding_horizontal(1);
            colonnade.left_margin(0);
            colonnade.alignment(Alignment::Center);
            execute!(stderr(), cursor::MoveTo(0, 0));
            let mut table = colonnade.tabulate(table).unwrap();
            let lines = table
                .into_iter()
                .map(|line| format!("{IntenseBoldBlue}{line}{ResetColor}"))
                .collect();
            place_lines(lines, 0.0, 0.0);

            // for lines in table {
            //     eprintln!("{IntenseBoldBlue}{lines}{ResetColor}");
            // }
        }
    }
    // execute!(
    //     stderr(),
    //     terminal::Clear(ClearType::FromCursorDown),
    //     cursor::RestorePosition
    // );
    // execute!(stderr(), cursor::RestorePosition);
}

fn start_idx(columns: usize, row: &Vec<Ctx>) -> usize {
    let node_count = row.len();

    let start_idx: usize = columns / 2 - node_count / 2;

    start_idx
}

pub fn print_state(state: &State, ctx: Ctx) {
    let parent = state.parent(ctx);
    let text = format!(
        "\x1b[1;36m{:?} {:?} -> {:?} {:?}\x1b[0m\n\x1b[0;36mPCs : {:?}\nReg : {:?}\nLens: {:?}\nChil: {:?}\x1b[0m\n",
        parent,
        state.scope_type(parent),
        ctx,
        state.scope_type(ctx),
        state.pcs(ctx),
        state.register(ctx),
        state.lengths(ctx),
        state.children(ctx),
    );
    let mut lines: Vec<String> = text.split("\n").map(str::to_string).collect();
    let (col, row) = (0.4, 0.8);

    place_lines(lines, col, row);
}

fn place_lines(lines: Vec<String>, col: f64, row: f64) {
    let (c, r) = size().unwrap();

    // execute!(
    //     stderr(),
    //     cursor::SavePosition,
    //     cursor::Hide,
    //     cursor::MoveTo(
    //         f64::floor(c as f64 * col) as u16,
    //         f64::floor(r as f64 * row) as u16
    //     )
    // );
    lines.into_iter().for_each(|line| {
        eprintln!("{}", line);
        // execute!(
        //     stderr(),
        //     cursor::MoveToColumn(f64::floor(c as f64 * col) as u16)
        // );
    });
    // execute!(
    //     stderr(),
    //     terminal::Clear(ClearType::FromCursorDown),
    //     cursor::RestorePosition,
    //     cursor::Show
    // );
}

fn print_exps(
    lhs: &(Exp, Ctx),
    rhs: &(Exp, Ctx),
    state: &State,
    lhs_stack: &Vec<(Exp, Ctx)>,
    rhs_stack: &Vec<(Exp, Ctx)>,
) {
    let ((lhs, lhs_ctx), (rhs, rhs_ctx)) = (lhs.clone(), rhs.clone());
    let mut colonnade = Colonnade::new(3, size().unwrap().0 as usize).unwrap();
    let width = size().unwrap().0 as usize - 3;
    let column_width = f64::floor(width as f64 / 3.) as usize;
    colonnade.fixed_width(column_width);
    colonnade.columns[0].alignment(Alignment::Left);
    // colonnade.columns[2].clear_limits();
    colonnade.columns[1].alignment(Alignment::Left);
    colonnade.columns[2].alignment(Alignment::Left);

    let parent = state.parent(lhs_ctx);
    let ctx_state: Vec<String> = format!(
        "\x1b[1;36m{:?} {:?} -> {:?} {:?}\x1b[0m\n\x1b[0;36mReg : {:?}\nLens: {:?}\nPCs : {:?}\nVel: {:?}\nChil: {:?}\x1b[0m",
        parent,
        state.scope_type(parent),
        lhs_ctx,
        state.scope_type(lhs_ctx),
        state.register(lhs_ctx),
        state.lengths(lhs_ctx),
        state.pcs(lhs_ctx),
        state.velocities(lhs_ctx),
        state.children(lhs_ctx),
    ).split("\n").map(str::to_string).collect();

    let exps: Vec<Exp> = lhs_stack.to_vec().iter().map(|(v, _)| v.clone()).collect();
    let stack_len_ = stack_len(&exps);

    let mut lhs_stack = lhs_stack.iter().rev();
    let mut rhs_stack = rhs_stack.iter().rev();

    let components = format!(
        "{} {} {}",
        lhs.to_string().to_uppercase(),
        '∘',
        rhs.to_string().to_uppercase()
    );

    let mut top_exp = Exp::Noop;

    let rhs_stack_top = if let Some((exp, rhs_ctx)) = rhs_stack.next() {
        top_exp = exp.clone();
        format!("{exp} {rhs_ctx:?}").to_uppercase()
    } else {
        " ".to_string()
    };

    let mut rhs_stack_ = Vec::<Vec<String>>::new();
    let mut v = Vec::<Vec<String>>::new();

    if let Exp::Compound(compound) = top_exp {
        match *compound.clone() {
            Compound::Parens(mut exps)
            | Compound::Braces(mut exps)
            | Compound::Brackets(mut exps) => {
                rhs_stack_.push(
                    exps.iter()
                        // .rev()
                        .map(|exp| format!("\u{00A0}┗━►{}", exp.to_string()).to_uppercase())
                        .collect::<Vec<String>>(),
                );
            }
            _ => todo!(),
        }
    }
    rhs_stack_.extend(
        rhs_stack
            .clone()
            .flat_map(|(exp, rhs_ctx)| {
                let mut v = Vec::<Vec<String>>::new();
                v.push(vec![format!("{exp} {rhs_ctx:?}").to_uppercase()]);
                if let Exp::Compound(compound) = exp {
                    match *compound.clone() {
                        Compound::Parens(mut exps)
                        | Compound::Braces(mut exps)
                        | Compound::Brackets(mut exps) => {
                            v.push(
                                exps.iter()
                                    // .rev()
                                    .map(|exp| {
                                        format!("\u{00A0}┗━►{}", exp.to_string()).to_uppercase()
                                    })
                                    .collect::<Vec<String>>(),
                            );
                        }
                        _ => todo!(),
                    }
                }
                v
            })
            .collect::<Vec<Vec<String>>>(),
    );

    let mut lhs_stack_ = Vec::<Vec<String>>::new();
    let mut top_exp = Exp::Noop;

    let lhs_stack_top: String = if let Some((exp, lhs_ctx)) = lhs_stack.next() {
        top_exp = exp.clone();
        format!("{exp} {lhs_ctx:?}").to_uppercase()
    } else {
        "\u{00A0}".repeat(column_width)
    };

    if let Exp::Compound(compound) = top_exp {
        match *compound {
            Compound::Parens(mut exps)
            | Compound::Braces(mut exps)
            | Compound::Brackets(mut exps) => {
                lhs_stack_.push(
                    exps.iter()
                        .rev()
                        .flat_map(|exp| {
                            vec![format!("\u{00A0}┗━►{}", exp.to_string()).to_uppercase()]
                        })
                        .collect::<Vec<String>>(),
                );
            }
            _ => todo!(),
        }
    }

    // dbg!(&lhs_stack_);

    lhs_stack_.extend(
        lhs_stack
            .clone()
            .flat_map(|(exp, lhs_ctx)| {
                let mut v = Vec::<Vec<String>>::new();
                v.push(vec![format!("{exp} {lhs_ctx:?}").to_uppercase()]);
                if let Exp::Compound(compound) = exp {
                    match *compound.clone() {
                        Compound::Parens(mut exps)
                        | Compound::Braces(mut exps)
                        | Compound::Brackets(mut exps) => {
                            v.push(
                                exps.iter()
                                    .rev()
                                    .map(|exp| {
                                        format!("\u{00A0}┗━►{}", exp.to_string()).to_uppercase()
                                    })
                                    .collect::<Vec<String>>(),
                            );
                        }
                        _ => todo!(),
                    }
                }
                v
            })
            .collect::<Vec<Vec<String>>>(),
    );

    // dbg!(&lhs_stack_);

    let lhs_stack_len = lhs_stack_.iter().flatten().count();
    let ctx_state_len = ctx_state.len();
    let rhs_stack_len = rhs_stack_.iter().flatten().count();
    let max_len = lhs_stack_len.max(ctx_state_len).max(rhs_stack_len);

    let lhs_stack: Vec<String> = lhs_stack_
        .into_iter()
        .flatten()
        .chain(repeat_n(
            "\u{00A0}".repeat(column_width),
            max_len - lhs_stack_len,
        ))
        .collect();

    let ctx_state: Vec<String> = ctx_state
        .into_iter()
        .chain(repeat_n(
            "\u{00A0}".repeat(column_width),
            max_len - ctx_state_len,
        ))
        .collect();

    let rhs_stack: Vec<String> = rhs_stack_
        .into_iter()
        .flatten()
        // .map(|(exp, ctx)| format!("{exp} {ctx:?}").to_uppercase())
        .chain(repeat_n(
            "\u{00A0}".repeat(column_width),
            max_len - rhs_stack_len,
        ))
        .collect();

    let data = lhs_stack
        .into_iter()
        .zip(ctx_state.into_iter().zip(rhs_stack))
        .map(|(l, (c, r))| {
            let v = vec![l, c, r];
            // eprintln!("{IntenseGreen}{v:?}{ResetColor}");
            v
        })
        .collect::<Vec<Vec<String>>>();

    // dbg!(&data);

    let (c, r) = size().unwrap();

    let mut table = vec![vec![lhs_stack_top, components, rhs_stack_top]];
    table.extend(data);
    // dbg!(&table);
    let table = colonnade.macerate(table);

    // dbg!(&table);

    if let Ok(table) = table {
        // execute!(stderr(), cursor::MoveToColumn(0));
        let (_, y) = position().unwrap();
        for r_ in y..(r - 15) {
            // execute!(
            //     stderr(),
            //     cursor::MoveTo(0, r),
            //     terminal::Clear(ClearType::CurrentLine)
            // );
        }

        // execute!(stderr(), terminal::Clear(ClearType::CurrentLine));
        for (row, rows) in table.iter().enumerate() {
            for (_, cols) in rows.iter().enumerate() {
                let mut lines = Vec::<String>::new();
                for (col, (margin, text)) in cols.iter().enumerate() {
                    let color = match (row, col) {
                        (0, 0) => IntenseBoldYellow,
                        (0, 1) => IntenseBoldBlue,
                        (0, 2) => IntenseBoldWhite,
                        (_, 0) => Yellow,
                        (_, 1) => Cyan,
                        (_, 2) => White,
                        _ => Black,
                    };
                    lines.push(format!("{color}{margin}{text}{ResetColor}",));
                }
                // place_lines(lines, 0., 0.5);
                eprintln!("{}", lines.join(""));
            }
        }
    }

    // eprintln!("\n");

    // execute!(stderr(), cursor::RestorePosition);
}

fn stack_len(stack: &Vec<Exp>) -> usize {
    let mut len: usize = stack.len();
    for exp in stack {
        if let Exp::Compound(compound) = exp {
            len += stack_len(&(*compound).to_vec())
        }
    }

    len
}

fn comment(c: &str) {
    let (cols, r) = size().unwrap();

    out(
        0,
        0,
        format!(
            "{0}{1:^2$}{3}\n",
            IntenseBoldBlack,
            c.to_string().to_uppercase(),
            cols as usize,
            ResetColor
        ),
    );
}
