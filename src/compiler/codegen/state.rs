use crossterm::{
    cursor::*,
    execute,
    style::*,
    terminal::{ClearType::UntilNewLine, *},
};
use num_rational::BigRational;
use pest::state;

use crate::{
    c, color,
    compiler::{
        ast::Scalar::Rest,
        codegen::{
            Data,
            utils::{TextStyle::*, *},
            *,
        },
        error,
    },
    pest_parser::Rule::controller,
};
use std::{
    any::{Any, TypeId},
    backtrace, clone,
    collections::{BTreeMap, HashMap, HashSet},
    convert,
    io::stderr,
    iter::{Cloned, Enumerate, Map, from_fn, repeat_n, zip},
    ops::{DerefMut, Index, IndexMut},
    process::exit,
    slice::{Iter, IterMut},
    vec::IntoIter,
};

#[derive(Debug)]
pub struct State {
    index_: usize,
    stack_indexes: Vec<usize>,
    peeked: Vec<Exp>,
    exps: Vec<Exp>,
    ctx_count: usize,
    ctxs: HashSet<Ctx>,
    parents: HashMap<Ctx, Ctx>,
    children: HashMap<Ctx, Vec<Ctx>>,
    scopes: HashMap<Ctx, Scope>,
    layers: HashMap<Ctx, Layer>,
    program: Option<Vec<Prog>>,
    programs_: HashMap<Ctx, Vec<Vec<Prog>>>,
    register: Option<Vec<Register>>,
    registers_: HashMap<Ctx, Vec<Vec<Register>>>,
    pcs_: HashMap<Ctx, Vec<Vec<Pc>>>,
    length: Option<Vec<Length>>,
    lengths_: HashMap<Ctx, Vec<Vec<Length>>>,
    velocity: Option<Vec<Velocity>>,
    velocities_: HashMap<Ctx, Vec<Vec<Velocity>>>,
    tempo: Mpb,
    tempos_: HashMap<Ctx, Vec<Vec<Mpb>>>,
    controllers: HashMap<Ctx, Vec<Vec<Controller>>>,
    pedal: HashMap<u8, bool>,
    bindings: HashMap<Ctx, HashMap<Ident, Exp>>,
    instruments: HashMap<Prog, Vec<u8>>,
    timeline: Vec<(u64, Ctx)>,
    timeline_: BTreeMap<Length, Vec<Slice>>,
    playhead: Length,
    stack: Vec<Exp>,
    thunks: HashMap<Ctx, HashMap<LifeCycleEvent, Vec<Thunk>>>,
    thunk_stack: Vec<(LifeCycleEvent, Thunk)>,
    lead_counter: Length,
}

impl Default for State {
    fn default() -> Self {
        let exps = Vec::new();

        let mut controllers = HashMap::<Ctx, Vec<Vec<Controller>>>::new();
        let controllers = vec![Vec::<Controller>::new()];
        // controllers.insert(Ctx::Root, vec![controller_vec]);

        Self {
            index_: 1,
            stack_indexes: Vec::new(),
            peeked: Vec::new(),
            exps,
            ctx_count: 0,
            ctxs: HashSet::new(),
            parents: HashMap::new(),
            children: HashMap::new(),
            scopes: HashMap::from_iter([(Ctx::Root, Scope::Sequence)]),
            layers: HashMap::new(),
            program: None,
            programs_: HashMap::default(),
            register: None,
            registers_: HashMap::default(),
            pcs_: HashMap::new(),
            length: None,
            lengths_: HashMap::new(),
            velocity: None,
            velocities_: HashMap::new(),
            tempo: Mpb::default(),
            tempos_: HashMap::default(),
            controllers: HashMap::from_iter(vec![(
                Ctx::Root,
                vec![vec![HashSet::from_iter(vec![(0x40, 0)])]],
            )]),
            pedal: HashMap::from_iter(vec![(0, false)]),
            bindings: HashMap::new(),
            instruments: HashMap::new(),
            timeline: Vec::new(),
            timeline_: BTreeMap::new(),
            playhead: Length::default(),
            stack: Vec::new(),
            thunks: HashMap::new(),
            thunk_stack: Vec::new(),
            lead_counter: Length::default(),
        }
    }
}

impl Iterator for State {
    type Item = Exp;
    fn next(&mut self) -> Option<Self::Item> {
        if self.peeked.len() > 0 {
            self.peeked.pop()
        } else {
            let index = self.index_;
            self.index_ += 1;
            self.exps.get(index).cloned()
        }
    }
}

impl ExactSizeIterator for State {
    fn len(&self) -> usize {
        self.exps.len() - (self.index_)
    }
}

impl State {
    pub fn append_child(&mut self, parent: Ctx) -> Ctx {
        // dbg!(parent, &self.ctxs);
        let ctx = if self.ctxs.is_empty() {
            self.ctxs.insert(parent);
            self.ctx_count += 1;
            self.set_parent(Ctx::Root, Ctx::None);
            Ctx::Root
            // let ctx = self.new_ctx();
            // self.set_parent(ctx, parent);
            // ctx
        } else {
            let ctx = self.new_ctx();
            self.set_parent(ctx, parent);
            ctx
        };

        // info(format!("{}APPEND CHILD {parent:?} -> {ctx:?}", color!(),));

        self.set_scope(ctx, Scope::None);
        self.programs_.insert(ctx, Vec::new());
        self.layers.insert(ctx, Layer::Heterogenous);
        self.registers_.insert(ctx, Vec::new());
        self.pcs_.insert(ctx, Vec::new());
        self.lengths_.insert(ctx, Vec::new());
        self.velocities_.insert(ctx, Vec::new());
        self.tempos_.insert(ctx, Vec::new());
        self.controllers.insert(ctx, Vec::new());
        self.bindings.insert(ctx, HashMap::new());

        while let Some((event, thunk)) = self.thunk_stack.pop() {
            self.add_thunk(ctx, event, thunk);
        }

        // graph(self, Ctx::Root, 2);

        // print_state(self, ctx);
        // pause();

        ctx
    }

    pub fn new_ctx(&mut self) -> Ctx {
        let id = self.ctx_count;
        self.ctx_count += 1;
        let ctx = Ctx::Id(id);
        self.ctxs.insert(ctx);
        ctx
    }

    pub fn index(&self) -> usize {
        self.index_
    }

    pub fn exps(&self) -> &Vec<Exp> {
        &self.exps
    }

    pub fn set_exps(&mut self, exps: Vec<Exp>, index: usize) -> (Vec<Exp>, usize) {
        // exps_.reverse();
        let index_ = self.index_ - self.peeked.len();
        self.peeked.clear();
        let exps_ = std::mem::replace(&mut self.exps, exps);
        self.index_ = index;
        // print_stacks(self);
        // pause();
        // dbg!(self.len());
        (exps_, index_)
    }

    pub fn set_stack(&mut self, stack: Vec<Exp>) -> Vec<Exp> {
        std::mem::replace(&mut self.stack, stack)
    }

    pub fn scope(&self, ctx: Ctx) -> Scope {
        self.scopes.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn set_scope(&mut self, ctx: Ctx, scope: Scope) {
        self.scopes.insert(ctx, scope);
    }

    pub fn layer(&self, ctx: Ctx) -> Layer {
        self.layers.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn set_layer(&mut self, ctx: Ctx, layer: Layer) {
        if let Some(layer_) = self.layers.get_mut(&ctx) {
            *layer_ = layer
        }
    }

    pub fn parent(&self, ctx: Ctx) -> Ctx {
        self.parents.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn set_parent(&mut self, ctx: Ctx, parent: Ctx) {
        self.parents.insert(ctx, parent);
        if let Some(children) = self.children.get_mut(&parent) {
            children.push(ctx);
        } else {
            self.children.insert(parent, vec![ctx]);
        }
    }

    #[inline(always)]
    pub fn children(&self, ctx: Ctx) -> Vec<Ctx> {
        self.children.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn children_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Ctx>> {
        self.children.get_mut(&ctx)
    }

    pub fn move_child(&mut self, child: Ctx, parent: Ctx) {
        let child_parent = self.parent(child);
        if let Some(children) = self.children.get_mut(&parent) {
            *children = children
                .iter()
                .cloned()
                .filter(|ctx| ctx.to_usize() != child.to_usize())
                .collect();
        }
        self.parents.remove(&child);
        self.set_parent(child, parent);
    }

    pub fn move_children(&mut self, src: Ctx, dest: Ctx) {
        let children = self.children(src);
        children
            .into_iter()
            .for_each(|ctx| self.move_child(ctx, dest));
    }

    fn add_child(&mut self, parent: Ctx, child: Ctx) {
        if let Some(children) = self.children.get_mut(&parent) {
            children.push(child);
        }
    }

    #[inline(always)]
    pub fn programs(&self, ctx: Ctx) -> Option<Vec<Vec<Prog>>> {
        self.programs_.get(&ctx).cloned()
    }

    pub fn programs_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Prog>> {
        self.programs_.entry(ctx).or_default()
    }

    pub fn add_program(&mut self, ctx: Ctx, program: Prog) {
        let pc_len = self.pcs(ctx).unwrap_or_default().len();
        let program_len = self.programs(ctx).unwrap_or_default().len();
        if let Some(programs) = self.programs_.get_mut(&ctx) {
            if program_len > pc_len && (program_len - pc_len) >= 1 {
                programs.remove(0);
            }
            programs.push(vec![program]);
        }
    }

    #[inline(always)]
    pub fn add_programs(&mut self, from: Ctx, to: Ctx) {
        let programs = std::mem::take(self.programs_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.programs(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}

        self.programs_mut(to).splice(idx.., programs);
        // let idx = self.pcs(ctx).unwrap_or_default().len().max(1) - 1;
        // if let Some(programs_) = self.programs_.get_mut(&ctx) {
        //     programs_.insert(idx, programs);
        // } else {
        //     self.programs_.insert(ctx, vec![programs]);
        // }
    }

    pub fn instruments(&self) -> &HashMap<Prog, Vec<u8>> {
        &self.instruments
    }

    pub fn instruments_mut(&mut self) -> &mut HashMap<Prog, Vec<u8>> {
        &mut self.instruments
    }

    #[inline(always)]
    pub fn registers(&self, ctx: Ctx) -> Option<Vec<Vec<Register>>> {
        self.registers_.get(&ctx).cloned()
    }

    pub fn registers_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Register>> {
        self.registers_.entry(ctx).or_default()
    }

    pub fn add_register(&mut self, ctx: Ctx, register: Register) {
        // let pc_len = self.pcs(ctx).unwrap_or_default().len();
        // let register_len = self.registers(ctx).len();

        if let Some(registers) = self.registers_.get_mut(&ctx) {
            registers.push(vec![register]);
        }
    }

    #[inline(always)]
    pub fn add_registers(&mut self, from: Ctx, to: Ctx) {
        let registers = std::mem::take(self.registers_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.registers(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}

        self.registers_mut(to).splice(idx.., registers);

        // let len = self.pcs(ctx).unwrap_or_default().len();
        // if let Some(registers_) = self.registers_.get_mut(&ctx) {
        //     registers_.resize(len, registers);
        //     // registers_.push(registers);
        // } else {
        //     self.registers_.insert(ctx, vec![registers]);
        // }
    }

    #[inline(always)]
    pub fn pcs(&self, ctx: Ctx) -> Option<Vec<Vec<Pc>>> {
        self.pcs_.get(&ctx).cloned()
    }

    pub fn pcs_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Pc>> {
        self.pcs_.entry(ctx).or_default()
    }

    pub fn add_pc(&mut self, ctx: Ctx, pc: Pc) {
        if let Some(pcs) = self.pcs_.get_mut(&ctx) {
            pcs.push(vec![pc]);
        }

        if ctx.to_usize() == 1 {
            // trace(TextStyle::IntenseBoldGreen);
            // misc(state_str(self, ctx));
            // pause();
        }
        // misc(state_str(self, ctx));
    }

    #[inline(always)]
    pub fn hoist_pcs(&mut self, from: Ctx, to: Ctx) {
        let pcs = std::mem::take(self.pcs_mut(from));

        self.pcs_mut(to).extend(pcs);
        // let idx = self.pcs(to).unwrap_or_default().len().max(1) - 1;
        // let pcs__ = self.pcs_mut(to);
        // pcs__.get_mut(idx).and_then(|pcs_| {*pcs_ = pcs; Some(()) });
    }

    #[inline(always)]
    pub fn velocities(&self, ctx: Ctx) -> Option<Vec<Vec<Velocity>>> {
        self.velocities_.get(&ctx).cloned()
    }

    pub fn velocities_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Velocity>> {
        self.velocities_.entry(ctx).or_default()
    }

    pub fn velocity(&self) -> Option<Vec<Velocity>> {
        self.velocity.clone()
    }

    pub fn set_velocity(&mut self, velocity: Velocity) {
        self.velocity = Some(vec![velocity]);
    }

    #[inline(always)]
    pub fn add_velocities(&mut self, from: Ctx, to: Ctx) {
        // dbg!(&velocities);
        // trace(TextStyle::IntenseBoldCyan);
        let velocities = std::mem::take(self.velocities_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.velocities(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}
        // pause();

        self.velocities_mut(to).splice(idx.., velocities);

        // misc(state_str(self, ctx));
    }

    #[inline(always)]
    pub fn add<T: Default + Debug + Clone + Into<Data> + From<Data>>(
        &mut self,
        ctx: Ctx,
        ts: Vec<T>,
    ) where
        Data: From<T>,
    {
        // log(format!(
        //     "{}ADD {ts:?} TO {ctx:?} {:?}\n",
        //     color!(),
        //     self.scope(ctx)
        // ));

        // c! {{
        //     print_state(self, ctx);
        //     pause();
        // }}

        match self.scope(ctx) {
            Scope::Sequence => match <T as Into<Data>>::into(T::default()) {
                Data::Pc(_) => {
                    self.pcs_mut(ctx)
                        .push(convert_vec::<Data, Pc>(convert_vec::<T, Data>(ts)));
                    // self.add_pcs(ctx, convert_vec::<Data, Pc>(convert_vec::<T, Data>(ts)))
                }
                Data::Length(_) => {
                    self.lengths_mut(ctx)
                        .push(convert_vec::<Data, Length>(convert_vec::<T, Data>(ts)));
                    // self.add_lengths(ctx, convert_vec::<Data, Length>(convert_vec::<T, Data>(ts)));
                }
                Data::Velocity(_) => {
                    // c! {{
                    //     print_state(self, ctx);
                    //     pause();
                    // }}
                    self.velocities_mut(ctx)
                        .push(convert_vec::<Data, Velocity>(convert_vec::<T, Data>(ts)));
                    // c! {{
                    //     print_state(self, ctx);
                    //     pause();
                    // }}
                    //     self.add_velocities(
                    //     ctx,
                    //     convert_vec::<Data, Velocity>(convert_vec::<T, Data>(ts)),
                    // )
                }
                // self.add_velocities(
                //     ctx,
                //     convert_vec::<Data, Velocity>(convert_vec::<T, Data>(ts)),
                // ),
                Data::Tempo(_) => {
                    self.tempos_mut(ctx)
                        .push(convert_vec::<Data, Tempo>(convert_vec::<T, Data>(ts)));

                    // self.add_tempos(ctx, convert_vec::<Data, Tempo>(convert_vec::<T, Data>(ts)))
                }
                Data::Register(_) => {
                    self.registers_mut(ctx)
                        .push(convert_vec::<Data, Register>(convert_vec::<T, Data>(ts)));

                    //     self.add_registers(
                    //     ctx,
                    //     convert_vec::<Data, Register>(convert_vec::<T, Data>(ts)),
                    // )
                }
                Data::Interpolation(_) => todo!(),
                Data::Program(_) => {
                    self.programs_mut(ctx)
                        .push(convert_vec::<Data, Prog>(convert_vec::<T, Data>(ts)));

                    // self.add_programs(ctx, convert_vec::<Data, Prog>(convert_vec::<T, Data>(ts)))
                }
                Data::None => todo!(),
                Data::Controller(_) => {
                    // dbg!(ts.len());
                    self.controllers_mut(ctx)
                        .push(convert_vec::<Data, Controller>(convert_vec::<T, Data>(ts)));

                    // self.add_controllers(
                    //     ctx,
                    //     convert_vec::<Data, Controller>(convert_vec::<T, Data>(ts)),
                    // );
                }
            },
            Scope::Stack => self.extend(ctx, vec![ts]),
            Scope::Set => todo!(),
            Scope::None => {
                trace(color!());

                // pause();
                todo!()
            }
        }
        // dbg!();
        // print_state(self, ctx);
        // misc(state_str(self, ctx));
    }

    /// Expand individual vectors of a parameter
    /// E.g. ```[[1], [2]] extended with [[3], [4]] becomes [[1, 3], [2, 4]]```
    #[inline(always)]
    pub fn extend<T: Default + Clone + Into<Data> + From<Data>>(
        &mut self,
        ctx: Ctx,
        ts: Vec<Vec<T>>,
    ) where
        Data: From<T>,
    {
        // eprintln!("{}EXTEND {ctx:?} {:?}\n", color!(), self.scope(ctx));
        // eprint!("{}", color!());
        // misc(state_str(self, ctx));
        // trace(color!());
        match <T as Into<Data>>::into(T::default()) {
            Data::Pc(_) => {
                let ts: Vec<Vec<Pc>> = ts
                    .into_iter()
                    .map(convert_vec::<T, Data>)
                    .map(convert_vec::<Data, Pc>)
                    .collect();

                if let Some(ts_) = self.pcs_.get_mut(&ctx) {
                    if ts_.is_empty() {
                        *ts_ = ts;
                    } else {
                        ts_.iter_mut().zip(ts.iter().cloned()).for_each(|(t_, t)| {
                            t_.extend(t);
                        });
                    }
                }
            }
            Data::Length(_) => {
                let ts: Vec<Vec<Length>> = ts
                    .into_iter()
                    .map(convert_vec::<T, Data>)
                    .map(convert_vec::<Data, Length>)
                    .collect();

                if let Some(ts_) = self.lengths_.get_mut(&ctx) {
                    // eprintln!("{}{ctx:?} ts_: {ts_:?}", color!());
                    // pause();
                    if ts_.is_empty() {
                        *ts_ = ts;
                    } else {
                        ts_.iter_mut().zip(ts.iter().cloned()).for_each(|(t_, t)| {
                            t_.extend(t);
                        });
                    }
                }
                // print_notes(ctx, self);
            }
            Data::Velocity(_) => {
                let ts: Vec<Vec<Velocity>> = ts
                    .into_iter()
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Velocity as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                if let Some(ts_) = self.velocities_.get_mut(&ctx) {
                    if ts_.is_empty() {
                        *ts_ = ts;
                    } else {
                        ts_[0].extend(ts[0].clone());
                    }
                }
            }
            Data::Tempo(_) => {
                let ts: Vec<Vec<Tempo>> = ts
                    .into_iter()
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Mpb as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                if let Some(ts_) = self.tempos_.get_mut(&ctx) {
                    if ts_.is_empty() {
                        *ts_ = ts;
                    } else {
                        ts.iter()
                            .cloned()
                            .zip(ts_.iter_mut())
                            .for_each(|(from, to)| {
                                to.extend(from);
                            });
                    }
                }
            }
            Data::Register(_) => {
                let ts: Vec<Vec<Register>> = ts
                    .into_iter()
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Register as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                if let Some(ts_) = self.registers_.get_mut(&ctx) {
                    if ts_.is_empty() {
                        *ts_ = ts;
                    } else {
                        ts.iter()
                            .cloned()
                            .zip(ts_.iter_mut())
                            .for_each(|(from, to)| {
                                to.extend(from);
                            });
                    }
                }
            }
            Data::Interpolation(_) => todo!(),
            Data::Program(_) => {
                let ts: Vec<Vec<Prog>> = ts
                    .into_iter()
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Prog as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                // misc(state_str(self, ctx));
                if let Some(ts_) = self.programs_.get_mut(&ctx) {
                    // dbg!(&ts, &ts_);

                    if ts_.is_empty() {
                        *ts_ = ts.clone();
                    } else {
                        ts_[0].extend(ts[0].clone());
                    }
                    // dbg!(&ts, &ts_);
                }
                // misc(state_str(self, ctx));
                // pause();
            }
            Data::None => todo!(),
            Data::Controller(_) => {
                let ts: Vec<_> = ts
                    .into_iter()
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Controller as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                // dbg!(&ts);
                // pause();
                let ts_ = self.controllers_mut(ctx);
                if ts_.is_empty() {
                    *ts_ = ts;
                } else {
                    ts.iter()
                        .cloned()
                        .zip(ts_.iter_mut())
                        .for_each(|(from, to)| {
                            to.extend(from);
                        });
                }

                // dbg!();
                // print_state(self, ctx);
                // pause();
            }
        }

        // dbg!();
        // misc(state_str(self, ctx));
    }

    pub fn set_velocities(&mut self, ctx: Ctx, velocities: Vec<Vec<Velocity>>) {
        if let Some(velocities_) = self.velocities_.get_mut(&ctx) {
            *velocities_ = velocities;
        }
    }

    #[inline(always)]
    pub fn lengths(&self, ctx: Ctx) -> Option<Vec<Vec<Length>>> {
        self.lengths_.get(&ctx).cloned()
    }

    pub fn lengths_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Length>> {
        self.lengths_.entry(ctx).or_insert(Vec::new())
    }

    pub fn add_length(&mut self, ctx: Ctx, length: Length) {
        // eprintln!("{BoldRed}{ctx:?}: {length:?}{ResetColor}");
        // trace(Green);
        let pc_len = self.pcs(ctx).unwrap_or_default().len();
        let length_len = self.lengths(ctx).unwrap_or_default().len();

        if let Some(lengths) = self.lengths_.get_mut(&ctx) {
            if length_len > pc_len && (length_len - pc_len) >= 1 {
                lengths.remove(0);
            }
            lengths.push(vec![length])
        }
    }

    pub fn hoist_lengths(&mut self, from: Ctx, to: Ctx) {
        // eprintln!("{}ADD LENGTHS: {lengths:?}", color!());
        // c! {{
        //     print_state(self, to);
        //     print_state(self, from);
        // }}
        // pause();
        let lengths = std::mem::take(self.lengths_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.lengths(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}
        // pause();

        self.lengths_mut(to).splice(idx.., lengths);

        // lengths_.get_mut(idx).and_then(|lens| {
        //     *lens = lengths;
        //     Some(())
        // });

        // c! {{
        //     dbg!();
        //     print_state(self, to);
        //     print_state(self, from);
        // }}
        // pause();
        // misc(state_str(self, ctx));
        // print_notes(ctx, self);
    }

    pub fn set_lengths(&mut self, ctx: Ctx, lengths: Vec<Vec<Length>>) {
        if let Some(lengths_) = self.lengths_.get_mut(&ctx) {
            *lengths_ = lengths;
        }
    }

    #[inline(always)]
    pub fn tempos(&self, ctx: Ctx) -> Option<Vec<Vec<Mpb>>> {
        self.tempos_.get(&ctx).cloned()
    }

    pub fn tempos_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Tempo>> {
        self.tempos_.entry(ctx).or_insert(Vec::new())
    }

    pub fn set_bpm(&mut self, bpm: Absolute) {
        let mpb = match bpm {
            Absolute::Float(float) => {
                Ratio::<BigInt>::from_integer(BigInt::from_f64(60_000_000. / float).unwrap())
            }
            Absolute::UInt(uint) => {
                Ratio::<BigInt>::from_integer(BigInt::from_f64(60_000_000. / uint as f64).unwrap())
            }
        };
        self.tempo = Mpb(mpb);
    }

    pub fn tempo(&self) -> &Tempo {
        &self.tempo
    }

    pub fn set_tempo(&mut self, tempo: Tempo) {
        // self.tempo = Mpb(tempo.0 - tempo.0 % 128);
        self.tempo = tempo;
    }

    pub fn add_tempo(&mut self, ctx: Ctx, tempo: Mpb) {
        let pc_len = self.pcs(ctx).unwrap_or_default().len();
        let tempo_len = self.tempos(ctx).unwrap_or_default().len();

        if let Some(tempos) = self.tempos_.get_mut(&ctx) {
            if tempo_len > pc_len && (tempo_len - pc_len) >= 1 {
                tempos.remove(0);
            }
            tempos.push(vec![Mpb(tempo.0.clone())]);
            // tempos.push(vec![tempo]);
        }
    }

    pub fn add_tempos(&mut self, from: Ctx, to: Ctx) {
        let tempos = std::mem::take(self.tempos_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.tempos(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}

        self.tempos_mut(to).splice(idx.., tempos);
        // let len = self.pcs(ctx).unwrap_or_default().len();
        // if let Some(tempos_) = self.tempos_.get_mut(&ctx) {
        //     tempos_.resize(len, tempos);
        //     // tempos_.get_mut(idx).and_then(|tmps| { *tmps = tempos; Some(()) });
        // } else {
        //     self.tempos_.insert(
        //         ctx,
        //         vec![
        //             tempos
        //                 .into_iter()
        //                 .map(|tempo| {
        //                     Mpb(tempo.0.clone()
        //                         - tempo.0
        //                             % Ratio::<BigInt>::from_integer(BigInt::from_u64(128).unwrap()))
        //                 })
        //                 .collect(),
        //         ],
        //     );
        // }
    }

    pub fn controllers(&self, ctx: Ctx) -> Option<Vec<Vec<Controller>>> {
        self.controllers.get(&ctx).cloned()
    }

    pub fn controllers_mut(&mut self, ctx: Ctx) -> &mut Vec<Vec<Controller>> {
        let ped = self.pedal(0);
        let controllers = self.controllers.entry(ctx).or_insert_with(|| match ped {
            true => vec![vec![HashSet::from_iter(vec![(PEDAL, 127)])]],
            false => vec![vec![HashSet::from_iter(vec![(PEDAL, 0)])]],
        });
        // .or_insert(controller_vec_vec);
        // dbg!(&controllers);
        controllers
    }

    pub fn add_controllers(&mut self, from: Ctx, to: Ctx) {
        // eprint!("{}", color!());
        // dbg!();
        // print_state(self, ctx);

        let controllers = std::mem::take(self.controllers_mut(from));
        let idx = (self.pcs(to).unwrap_or_default().len().max(1) - 1)
            .min(self.controllers(to).unwrap_or_default().len());
        // c! {{
        //     dbg!(idx);
        // }}

        self.controllers_mut(to).splice(idx.., controllers);

        // let len = self.pcs(ctx).unwrap_or_default().len();
        // if let Some(controllers_) = self.controllers.get_mut(&ctx) {
        //     controllers_.resize(len, controllers);
        // controllers_.get_mut(idx).and_then(|ctls| {
        //     *ctls = controllers;
        //     Some(())
        // });

        // eprint!("{}", color!());
        // print_state(self, ctx);
        // pause();
    }

    pub fn bindings(&self, ctx: Ctx) -> HashMap<Ident, Exp> {
        self.bindings.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn binding(&self, mut ctx: Ctx, ident: &Ident) -> Option<Exp> {
        // dbg!(&self.bindings);
        let mut binding = Exp::Noop;
        while !matches!(ctx, Ctx::None) {
            // dbg!(ctx);
            if let Some(bindings) = self.bindings.get(&ctx) {
                if let Some(binding_) = bindings.get(ident) {
                    binding = binding_.clone();
                    break;
                } else {
                    ctx = self.parent(ctx);
                }
            } else {
                ctx = self.parent(ctx);
            }
        }

        if matches!(binding, Exp::Noop) {
            error(format!("{IntenseBoldRed}Undefined Symbol: \"{}\"", ident.0));
            // trace(TextStyle::BoldGreen);
            std::process::exit(1);
        }

        Some(binding)
    }

    pub fn add_binding(&mut self, ctx: Ctx, ident: Ident, exp: Exp) {
        if let Some(bindings) = self.bindings.get_mut(&ctx) {
            if let Exp::Simple(Simple::Scalar(Scalar::Prog(prog))) = exp.clone() {
                self.instruments.insert(prog, ident.0.clone().into_bytes());
            }
            bindings.insert(ident, exp);
        } else {
            self.bindings
                .insert(ctx, HashMap::from_iter([(ident, exp)]));
        }
        // dbg!(&self.bindings);
    }

    pub fn add_bindings(&mut self, ctx: Ctx, bindings: HashMap<Ident, Exp>) {
        if let Some(bindings_) = self.bindings.get_mut(&ctx) {
            bindings_.extend(bindings);
        } else {
            self.bindings.insert(ctx, bindings);
        }
        // dbg!(&self.bindings);
    }

    pub fn thunks(&self, ctx: Ctx) -> HashMap<LifeCycleEvent, Vec<Thunk>> {
        self.thunks.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn thunks_mut(&mut self, ctx: Ctx) -> &mut HashMap<LifeCycleEvent, Vec<Thunk>> {
        self.thunks.entry(ctx).or_insert(HashMap::new())
    }

    pub fn push_thunk(&mut self, event: LifeCycleEvent, thunk: Thunk) {
        self.thunk_stack.push((event, thunk));
    }

    pub fn add_thunk(&mut self, ctx: Ctx, life_cycle_event: LifeCycleEvent, thunk: Thunk) {
        // c! {{
        //     eprintln!("ADD THUNK: {ctx:?} {life_cycle_event:?}");
        //     pause();
        // }}
        // pause();
        // misc(format!("{:?}", self.thunks(ctx)));
        // pause();
        if let Some(thunks) = self.thunks.get_mut(&ctx) {
            if let Some(event) = thunks.get_mut(&life_cycle_event) {
                event.push(thunk);
            } else {
                thunks.insert(life_cycle_event, vec![thunk]);
            }
        } else {
            self.thunks.insert(
                ctx,
                HashMap::<LifeCycleEvent, Vec<Thunk>>::from_iter(vec![(
                    life_cycle_event,
                    vec![thunk],
                )]),
            );
        }
        // misc(format!("{:?}", self.thunks(ctx)));
        // pause();
    }

    pub fn call_thunks(&mut self, from: Ctx, to: Ctx, life_cycle_event: LifeCycleEvent) {
        // eprintln!(
        //     "CALL THUNKS {life_cycle_event:?} from {from:?} to {to:?}"
        // );

        // c! {{
        //     print_state(self, to);
        //     // pause();
        // }}

        match life_cycle_event {
            LifeCycleEvent::PreComposition => {
                let thunks = self
                    .thunks_mut(from)
                    .entry(LifeCycleEvent::PreComposition)
                    .or_default()
                    .clone();
                thunks.into_iter().for_each(|thunk| {
                    thunk.call(to, self);
                });

                self.thunks_mut(from)
                    .entry(LifeCycleEvent::PreComposition)
                    .or_default()
                    .clear();

                self.thunks_mut(from)
                    .entry(LifeCycleEvent::Composition)
                    .or_default()
                    .clear();
            }
            LifeCycleEvent::Composition => {
                let thunks = self
                    .thunks_mut(from)
                    .entry(LifeCycleEvent::Composition)
                    .or_default()
                    .clone();
                thunks.into_iter().for_each(|thunk| {
                    thunk.call(to, self);
                });

                self.thunks_mut(from)
                    .entry(LifeCycleEvent::PreComposition)
                    .or_default()
                    .clear();

                self.thunks_mut(from)
                    .entry(LifeCycleEvent::Composition)
                    .or_default()
                    .clear();
            }

            _ => {
                let thunk_map = self.thunks_mut(from);
                let thunks = thunk_map
                    .get(&life_cycle_event)
                    .cloned()
                    .unwrap_or_default();
                thunks.iter().cloned().for_each(|mut thunk| {
                    thunk.call(to, self);
                });
            }
        }

        // c! {{
        //     print_state(self, to);
        //     pause();
        // }}
    }

    // pub fn call_thunks(&mut self, from: Ctx, to: Ctx, life_cycle_event: LifeCycleEvent) {
    //     // eprintln!(
    //     //     "CALL THUNKS {life_cycle_event:?} from {from:?} to {to:?}"
    //     // );
    //     // pause();

    //     match life_cycle_event {
    //         LifeCycleEvent::Composition => {
    //             let thunk_map = self.thunks.entry(from).or_default();
    //             let thunks = thunk_map.entry(LifeCycleEvent::Composition).or_default();
    //             thunks.iter_mut().for_each(|thunk| {
    //                 thunk.call(to, self);
    //             });
    //         }
    //         _ => {
    //             let thunk_map = self.thunks.get(&from).cloned().unwrap_or_default();
    //             let thunks = thunk_map
    //                 .get(&life_cycle_event)
    //                 .cloned()
    //                 .unwrap_or_default();
    //             thunks.iter().cloned().for_each(|mut thunk| {
    //                 thunk.call(to, self);
    //             });
    //         }
    //     }

    //     // thunks.iter().for_each(|thunk| eprintln!("{thunk:?}"));
    // }

    pub fn clear_thunks(&mut self, ctx: Ctx, life_cycle_event: LifeCycleEvent) {
        self.thunks
            .entry(ctx)
            .or_default()
            .get_mut(&life_cycle_event)
            .and_then(|thunks| {
                thunks.clear();
                Some(())
            });
    }

    pub fn push_back(&mut self, exp: Exp) {
        // dbg!(self.index);
        self.peeked.push(exp);
        // eprintln!("{}INDEX: {}", color!(), self.index_);
    }

    pub fn store(&mut self, exp: Exp) {
        self.stack.push(exp);
    }

    /// Load expression from stack
    pub fn load(&mut self) -> Option<Exp> {
        self.stack.pop()
    }

    pub fn push(&mut self) {
        // eprintln!("{}PUSH {exp}", color!());
        self.stack_indexes.push(self.index_);
        self.index_ += 1;
    }

    pub fn pop(&mut self) -> Option<Exp> {
        self.stack.pop()
    }

    pub fn stack(&self) -> &Vec<Exp> {
        &self.stack
    }

    pub fn timeline(&self) -> &BTreeMap<Length, Vec<Slice>> {
        &self.timeline_
    }

    pub fn timeline_mut(&mut self) -> &mut BTreeMap<Length, Vec<Slice>> {
        &mut self.timeline_
    }

    #[inline(always)]
    pub fn playhead(&mut self) -> &mut Length {
        &mut self.playhead
    }

    pub fn max_len(&self, ctx: Ctx) -> usize {
        vec![
            self.pcs(ctx).unwrap_or_default().len(),
            self.lengths(ctx).unwrap_or_default().len(),
            self.velocities(ctx).unwrap_or_default().len(),
            self.registers(ctx).unwrap_or_default().len(),
        ]
        .iter()
        .cloned()
        .max()
        .unwrap_or_default()
    }

    pub fn cycle_fill(&mut self, ctx: Ctx, len: Length) {
        c! {{
            eprintln!(
            "{}CYCLE FILL {ctx:?} LEN: {:?}",
            color!(),
            len
        );
        }}

        // dbg!();
        // print_state(self, ctx);
        // print_states(self, ctx);
        // misc(state_str(self, ctx));
        // pause();

        let children_len = self.children(ctx).len();
        if children_len > 0 && len.as_ratio() > BigRational::ZERO {
            // dbg!();
            let clone = self.children(ctx);
            let mut cycle = clone.iter().cloned().cycle();
            while self.get_ctx_length(ctx) < len {
                // dbg!(ctx, self.get_ctx_length(ctx).as_u64(), len.as_u64());
                // pause();
                self.children_mut(ctx)
                    .unwrap()
                    .extend(cycle.by_ref().take(1));
            }
            // dbg!();
        } else {
            // dbg!();
            let len_clone = self.lengths(ctx).unwrap_or_default();
            if len_clone.len() == 0 {
                return;
            }
            let mut len_cycle = len_clone.iter().cloned().cycle();
            let prog_clone = self.programs(ctx).unwrap_or_default();
            let mut prog_cycle = prog_clone.iter().cloned().cycle();
            let reg_clone = self.registers(ctx).unwrap_or_default();
            let mut reg_cycle = reg_clone.iter().cloned().cycle();
            let pc_clone = self.pcs(ctx).unwrap_or_default();
            let mut pc_cycle = pc_clone.iter().cloned().cycle();
            let vel_clone = self.velocities(ctx).unwrap_or_default();
            let mut vel_cycle = vel_clone.iter().cloned().cycle();
            let temp_clone = self.tempos(ctx).unwrap_or_default();
            let mut temp_cycle = temp_clone.iter().cloned().cycle();
            let controllers_clone = self.controllers(ctx).unwrap_or_default();
            // dbg!(&controllers_clone.len());
            let mut controllers_cycle = controllers_clone.iter().cloned().cycle();
            // print_state(self, self.parent(ctx));
            // print_state(self, ctx);
            // pause();
            let mut ctx_len = self.get_ctx_length(ctx);

            while ctx_len.as_ratio() < len.as_ratio() {
                c!{{
                    eprintln!("ctx_len: {} len: {:?} rate: {:?}",
                        ctx_len.as_ratio().to_f64().unwrap_or_default(),
                        len.as_ratio().to_f64().unwrap_or_default(),
                        (ctx_len.clone().as_ratio() / len.clone().as_ratio()).to_f64().unwrap_or_default());
                }}
                let len_ = len_cycle
                    .by_ref()
                    .next()
                    .unwrap_or_default()
                    .iter()
                    .cloned()
                    .min()
                    .unwrap_or_default();
                let length = if ctx_len.clone() + len_.clone() <= len {
                    len_.clone()
                } else {
                    len.clone() - ctx_len.clone()
                };

                c! {{
                    eprintln!("length: {length:?}");
                }}

                self.add::<Length>(ctx, vec![length.clone()]);
                ctx_len = self.get_ctx_length(ctx);
            }
            // }

            let length_len = self.lengths(ctx).unwrap_or_default().len();

            c! {{
                eprintln!("length_len: {length_len}");
            }}

            self.programs_mut(ctx)
                .extend(prog_cycle.take(length_len.max(prog_clone.len()) - prog_clone.len()));

            self.registers_mut(ctx)
                .extend(reg_cycle.take(length_len.max(reg_clone.len()) - reg_clone.len()));
            self.pcs_mut(ctx)
                .extend(pc_cycle.take(length_len.max(pc_clone.len()) - pc_clone.len()));

            self.velocities_mut(ctx)
                .extend(vel_cycle.take(length_len.max(vel_clone.len()) - vel_clone.len()));
            self.tempos_mut(ctx)
                .extend(temp_cycle.take(length_len.max(temp_clone.len()) - temp_clone.len()));
            let len = self.controllers.borrow().len();
            self.controllers_mut(ctx).extend(
                controllers_cycle
                    .take(length_len.max(controllers_clone.len()) - controllers_clone.len()),
            );
            // self.add::<HashMap<u8, u8>>(
            //     ctx,
            //     controllers_cycle.by_ref().next().unwrap_or_default(),
            // );

            // dbg!();
        }

        c! {{
            print_state(self, ctx);
        }}
    }

    pub fn get_last_mut<'a, T>(&'a mut self, ctx: Ctx) -> Option<&'a mut T>
    where
        T: Interpolant,
    {
        let mut ts = T::get_vec_mut(self, ctx);
        ts.last_mut().and_then(|last| last.last_mut())
    }

    pub fn get<T: Clone + Default + Debug + Into<Data> + From<Data> + ToString>(
        &mut self,
        ctx: Ctx,
    ) -> Vec<T>
    where
        Data: From<T>,
    {
        // eprintln!("{}GET {} {ctx:?}", color!(), T::default().to_string());
        // graph(self, ctx, 3);

        let t = T::default();

        let data = Data::from(t);
        // dbg!(&data);

        let res = match data {
            Data::Pc(_) => self.get_::<Pc>(ctx),
            Data::Length(_) => self.get_::<Length>(ctx),
            Data::Velocity(_) => self.get_::<Velocity>(ctx),
            Data::Tempo(_) => self.get_::<Tempo>(ctx),
            Data::Register(_) => self.get_::<Register>(ctx),
            Data::Program(prog) => self.get_::<Prog>(ctx),
            Data::Interpolation(_) => todo!(),
            Data::None => todo!(),
            Data::Controller(_) => {
                // dbg!();
                self.get_::<Controller>(ctx)
            }
        };

        if matches!(&res.as_slice(), &[Data::None]) {
            vec![]
        } else {
            let res = convert_vec::<Data, T>(res);
            // eprintln!("{}{res:?}", color!());
            res
        }
    }

    fn get_<T: Default + Clone + From<Data> + Into<Data>>(&mut self, ctx: Ctx) -> Vec<Data> {
        // misc(state_str(self, ctx));
        let mut ctx_ = ctx.clone();

        let default = T::default();

        let mut data: Option<Vec<Data>> = None;
        let mut parent = self.parent(ctx);

        't: while ctx_ != Ctx::None {
            // dbg!(ctx_);

            if ctx_ == ctx {
                if let Some(ts) = match Data::from(default.clone().into()) {
                    Data::Pc(_) => self
                        .pcs(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(|ts| Some(convert_vec::<Pc, Data>(ts))),
                    Data::Length(_) => self
                        .lengths(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(|ts| Some(convert_vec::<Length, Data>(ts))),
                    Data::Velocity(_) => self
                        .velocities(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(|ts| {
                            // self.velocity = Some(ts.clone());
                            Some(convert_vec::<Velocity, Data>(ts))
                        }),
                    Data::Tempo(_) => self
                        .tempos(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(|ts| Some(convert_vec::<Tempo, Data>(ts))),
                    Data::Register(_) => self
                        .registers(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(
                            |ts| {
                                // eprint!("{}", color!());
                                // dbg!(&ts);
                                // self.register = Some(ts.clone());
                                Some(convert_vec::<Register, Data>(ts))
                            }, // )
                               // .or(self.register.clone().and_then(|ts| {
                               //     self.register = Some(ts.clone());
                               //     Some(convert_vec::<Register, Data>(ts))
                               // })
                        ),
                    Data::Interpolation(_) => todo!(),
                    Data::Program(_) => self
                        .programs(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(|ts| {
                            // self.program = Some(ts.clone());
                            Some(convert_vec::<Prog, Data>(ts))
                        }),
                    Data::None => todo!(),
                    Data::Controller(_) => {
                        Some(convert_vec::<Controller, Data>(vec![
                            Controller::from_iter(vec![match self.pedal(0) {
                                true => (PEDAL, 127),
                                false => (PEDAL, 0),
                            }]),
                        ]))
                        // let len = self.pcs(ctx_).unwrap_or_default().len();
                        // dbg!();
                        // print_state(self, ctx_);
                        // // pause();
                        // self.controllers(ctx_)
                        //     .and_then(|ts| {
                        //         dbg!(&ts);
                        //         print_state(self, ctx_);
                        //         // pause();
                        //         // ts.get(len - 1).cloned().or(ts.last().cloned())
                        //         ts.last().cloned()
                        //     })
                        //     .and_then(|ts| {
                        //         dbg!(ctx_, &ts);
                        //         // pause();
                        //         Some(convert_vec::<Controller, Data>(ts))
                        //     })
                    }
                } {
                    data = Some(ts);
                }
            } else {
                data = match Data::from(default.clone().into()) {
                    Data::Pc(_) => self
                        .pcs_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Pc, Data>(ts_))),
                    Data::Length(_) => self
                        .lengths_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Length, Data>(ts_))),
                    Data::Velocity(_) => self
                        .velocities_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Velocity, Data>(ts_))),
                    Data::Tempo(_) => self
                        .tempos_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Tempo, Data>(ts_))),
                    Data::Register(_) => self
                        .registers_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Register, Data>(ts_))),
                    Data::Interpolation(_) => todo!(),
                    Data::Program(_) => self
                        .programs_mut(ctx_)
                        .last()
                        .cloned()
                        .and_then(|ts_| Some(convert_vec::<Prog, Data>(ts_))),
                    Data::Controller(_) => {
                        // print_state(self, ctx_);
                        Some(convert_vec::<Controller, Data>(vec![
                            Controller::from_iter(vec![match self.pedal(0) {
                                true => (PEDAL, 127),
                                false => (PEDAL, 0),
                            }]),
                        ]))
                        // self.controllers_mut(ctx_).last().cloned().and_then(|ts_| {
                        //     dbg!(&ts_);
                        //     pause();
                        //     Some(convert_vec::<Controller, Data>(ts_.clone()))
                        // })
                    }
                    Data::None => todo!(),
                };

                // pause();

                // dbg!();
                // eprintln!("{}{data:?}{ResetColor}", color!());
            }

            // dbg!(&data);

            if data.is_some() {
                break 't;
            } else {
                ctx_ = self.parent(ctx_);
            }
        }

        if let Some(data) = data {
            // dbg!(ctx, &data);
            match self.scope(ctx) {
                Scope::Sequence => data,
                Scope::Stack => vec![data.last().cloned().unwrap_or_default()],
                Scope::Set => todo!(),
                Scope::None => todo!(),
            }
        } else {
            match Data::from(default.clone().into()) {
                Data::Controller(_) => {
                    let mut map = BTreeMap::<u8, u8>::new();
                    vec![(PEDAL, 0).into()]
                }
                _ => vec![Data::from(default.into())],
            }
        }
    }

    fn get_last<T: Default + Clone + From<Data> + Into<Data>>(&mut self, ctx: Ctx) -> Vec<T>
    where
        Data: From<T>,
    {
        // misc(state_str(self, ctx));
        match Data::from(T::default()) {
            Data::Pc(_) => convert_vec::<Data, T>(convert_vec::<Pc, Data>(
                self.pcs(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Pc::None]),
            )),
            Data::Length(_) => convert_vec::<Data, T>(convert_vec::<Length, Data>(
                self.lengths(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Length::MicroSeconds(self.tempo().0.clone())]),
            )),
            Data::Velocity(_) => convert_vec::<Data, T>(convert_vec::<Velocity, Data>(
                self.velocities(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Velocity::default()]),
            )),
            Data::Tempo(_) => convert_vec::<Data, T>(convert_vec::<Tempo, Data>(
                self.tempos(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Tempo::default()]),
            )),
            Data::Register(_) => convert_vec::<Data, T>(convert_vec::<Register, Data>(
                self.registers(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Register::default()]),
            )),
            Data::Interpolation(_) => todo!(),
            Data::Program(_) => convert_vec::<Data, T>(convert_vec::<Prog, Data>(
                self.programs(ctx)
                    .unwrap_or_default()
                    .last()
                    .cloned()
                    .unwrap_or(vec![Prog::default()]),
            )),
            Data::None => todo!(),
            Data::Controller(_) => todo!(),
        }
    }

    pub fn get_leaves(&self, ctx: Ctx) -> Vec<Ctx> {
        let mut children = self.children(ctx);
        if children.is_empty() {
            vec![ctx]
        } else {
            children
                .into_iter()
                .flat_map(|ctx| self.get_leaves(ctx))
                .collect()
        }
    }

    // pub fn pad(&mut self, ctx: Ctx, len: usize)

    pub fn pad(&mut self, ctx: Ctx) {
        // eprintln!("{}PAD {ctx:?}", color!(),);

        self.pad_(ctx, Self::programs_mut, Self::programs);
        self.pad_(ctx, Self::velocities_mut, Self::velocities);
        self.pad_(ctx, Self::lengths_mut, Self::lengths);
        self.pad_(ctx, Self::registers_mut, Self::registers);
        self.pad_(ctx, Self::tempos_mut, Self::tempos);
        self.pad_(ctx, Self::controllers_mut, Self::controllers);
    }

    fn pad_<T: Clone + Default + Debug + Into<Data> + From<Data> + ToString, F, G>(
        &mut self,
        ctx: Ctx,
        mut f: F,
        g: G,
    ) where
        F: FnMut(&mut Self, Ctx) -> &mut Vec<Vec<T>>,
        G: Fn(&Self, Ctx) -> Option<Vec<Vec<T>>>,
        Data: From<T>,
    {
        // c! {{
        //     print_state(self, ctx);
        //     pause();
        // }}
        let mut values: Vec<T> = self.get(ctx);
        // c! {{
        //     eprintln!("VALS: {:?}", values);
        //     pause();
        // }}

        let pcs = self.pcs(ctx).unwrap_or_default();
        let pc_index = pcs.len().max(1) - 1;
        let pcs_ = self
            .pcs(ctx)
            .unwrap_or_default()
            .get(pc_index)
            .cloned()
            .unwrap_or_default();
        let pc_count = pcs_.len();

        let values_ = g(self, ctx)
            .unwrap_or_default()
            .get(pc_index)
            .cloned()
            .unwrap_or_default();

        let len = values_.len();

        let last = values.last().cloned().unwrap_or_default();
        let take = pc_count.max(len) - len;
        // dbg!(pcs.len(), len, take);
        if take > 0 {
            let values = values
                .iter()
                .cloned()
                .chain(from_fn(move || Some(last.clone())))
                .take(take)
                .collect();
            // dbg!(&values);
            // pause();
            self.add(ctx, values);
        }

        // c! {{
        //     print_state(self, ctx);
        //     pause();
        // }}
    }

    pub fn hoist(&mut self, from: Ctx, to: Ctx) {
        // let children = self.children(from);
        // let programs = std::mem::take(self.programs_mut(from));
        // let pcs = std::mem::take(self.pcs_mut(from));
        // let velocities = std::mem::take(self.velocities_mut(from));
        // let registers = std::mem::take(self.registers_mut(from));
        // let lengths = std::mem::take(self.lengths_mut(from));
        // let tempos = std::mem::take(self.tempos_mut(from));
        // let controllers = std::mem::take(self.controllers_mut(from));
        self.hoist_pcs(from, to);
        self.hoist_lengths(from, to);
        self.add_velocities(from, to);
        self.add_tempos(from, to);
        self.add_registers(from, to);
        self.add_programs(from, to);
        self.add_controllers(from, to);
        self.move_children(from, to);

        // c! {{
        //     print_state(self, to);
        // }}
        // pause();
    }

    pub fn clean_context(&mut self, ctx: Ctx) {
        self.programs_mut(ctx).clear();
        self.pcs_mut(ctx).clear();
        self.velocities_mut(ctx).clear();
        self.registers_mut(ctx).clear();
        self.lengths_mut(ctx).clear();
        self.tempos_mut(ctx).clear();
    }

    pub fn drop(&mut self, ctx: Ctx) {
        self.bindings.remove(&ctx);
        self.tempos_.remove(&ctx);
        self.velocities_.remove(&ctx);
        self.lengths_.remove(&ctx);
        self.pcs_.remove(&ctx);
        self.registers_.remove(&ctx);
        self.programs_.remove(&ctx);
        self.children.remove(&ctx);
        let parent = self.parents.get(&ctx).cloned().unwrap_or_default();
        if let Some(siblings) = self.children.get_mut(&parent) {
            // dbg!(siblings.clone());
            *siblings = siblings
                .iter()
                .cloned()
                .filter(|ctx_| *ctx_ != ctx)
                .collect();
            // dbg!(siblings);
        }
        self.thunks.remove(&ctx);
        self.parents.remove(&ctx);
        self.ctxs.remove(&ctx);

        // eprintln!("{}{ctx:?} DROPPED{}\n", color!(), color!());
    }

    pub fn sequence(&mut self, ctx: Ctx) {
        // eprintln!("{}SEQUENCE {ctx:?} {:?}\n", color!(), self.scope(ctx));

        graph(self, ctx, 5);

        // eprintln!("{}THUNKS: {:#?}\n", color!(), self.thunks,);
        // graph(self, Ctx::Root);
        // misc(state_str(self, ctx));
        let children = self.children(ctx);

        match (ctx, self.scope(ctx)) {
            (Ctx::Root | Ctx::Id(1..), Scope::Sequence) => {
                // eprint!("{}", color!());
                // dbg!();

                // if children.len() > 0 {
                //     children.iter().cloned().for_each(|ctx| self.sequence(ctx))
                // } else {
                //     self.combine_sequences(vec![ctx]);
                // }
            }
            (Ctx::Root | Ctx::Id(1..), Scope::Stack) => {
                if children.len() > 0 {
                    self.combine_sequences(children);
                } else {
                    // eprintln!("{}PLAYHEAD: {:?}", color!(), self.playhead(),);
                    self.combine_sequences(vec![ctx]);

                    // eprint!(
                    //     "{}{}{}",
                    //     color!(),
                    //     trace.to_string(),
                    //     TextStyle::ResetColor
                    // );
                }
                // misc(state_str(self, ctx));
            }
            _ => (),
        }

        // graph(self, Ctx::Root);
    }

    #[inline(always)]
    pub fn get_ctx_length(&self, ctx: Ctx) -> Length {
        // print_state(self, ctx);
        let children = self.children(ctx);
        if children.len() > 0 {
            // dbg!();
            let mut iter = children.iter().cloned();
            // dbg!(iter.len());
            let next = iter.next().unwrap();
            let init = match self.scope(next.clone()) {
                Scope::Sequence => self.length_sum(next.clone()),
                Scope::Stack => self.min_length(next.clone()),
                _ => todo!(),
            };
            let length = iter.fold(init, |total, ctx_| {
                // dbg!();
                // dbg!(total.as_u64());
                match self.scope(ctx) {
                    Scope::Sequence => total + self.get_ctx_length(ctx_),
                    Scope::Stack => total.min(self.get_ctx_length(ctx_)),
                    Scope::Set => todo!(),
                    Scope::None => Length::None,
                }
            });
            // c! {{
            //     eprintln!("{ctx:?} {}", length.clone().as_u64())
            // }}
            // if length.as_u64() == 0 {
            //     pause();
            // }
            // dbg!(length.as_u64());
            length
        } else {
            // dbg!();
            // misc(state_str(self, ctx));
            match self.scope(ctx) {
                Scope::Sequence => {
                    let sum = self.length_sum(ctx);
                    // c! {{
                    //     eprintln!("{ctx:?} {sum:?} {}", sum.clone().as_u64())
                    // }}
                    // if sum.clone().as_u64() == 0 { pause(); }
                    sum
                }
                Scope::Stack => self.min_length(ctx),
                Scope::Set => todo!(),
                Scope::None => self.get_ctx_length(Ctx::Root),
            }
        }
    }

    pub fn stack_len(&self) -> usize {
        self.stack_indexes.len()
    }

    pub fn length_sum(&self, ctx: Ctx) -> Length {
        // dbg!(ctx);
        // print_state(self, ctx);

        let children = self.children(ctx);
        // dbg!(children.len());
        if children.len() > 0 {
            // dbg!();
            let length = children
                .into_iter()
                .fold(Length::default(), |sum, ctx| sum + self.length_sum(ctx));
            // let ratio = Ratio::<BigInt>::from_integer(BigInt::from_f64(float).unwrap());
            // Length::MicroSeconds(ratio)
            length
        } else {
            // dbg!();
            // misc(state_str(self, ctx));
            self.lengths(ctx)
                .unwrap_or_default()
                .iter()
                .map(|lengths| {
                    let min = lengths.iter().cloned().min().unwrap_or_default();
                    // if matches!(ctx, Ctx::Id(2)) {c! {{
                    //     eprintln!("{:?} {}", min.as_ratio(), min.as_u64());
                    //     pause();
                    // }}}
                    min
                })
                .sum()
        }
    }

    pub fn min_length(&self, ctx: Ctx) -> Length {
        let children = self.children(ctx);
        if children.len() > 0 {
            children
                .into_iter()
                .fold(Length::default_max(), |min, ctx| {
                    min.min(self.get_ctx_length(ctx))
                })
        } else {
            self.lengths(ctx)
                .unwrap_or_default()
                .iter()
                .map(|lengths| lengths.iter().cloned().min().unwrap_or_default())
                .min()
                .unwrap_or_default()
        }
    }

    pub fn max_length(&self, ctx: Ctx) -> Length {
        self.lengths(ctx)
            .unwrap_or_default()
            .iter()
            .map(|lengths| lengths.iter().cloned().max().unwrap_or_default())
            .max()
            .unwrap_or_default()
    }

    pub fn ctx_pc_count(&self, ctx: Ctx) -> usize {
        if self.children(ctx).is_empty() {
            self.pcs(ctx).unwrap_or_default().len()
        } else {
            self.children(ctx)
                .iter()
                .cloned()
                .map(|ctx| self.ctx_pc_count(ctx))
                .sum()
        }
    }

    pub fn combine_sequences(&mut self, ctxs: Vec<Ctx>) {
        // eprintln!(
        //     "{}COMBINE SEQUENCES {}",
        //     color!(),
        //     ctxs.iter()
        //         .cloned()
        //         .map(|ctx| format!("{ctx:?}").to_uppercase())
        //         .collect::<Vec<String>>()
        //         .join(", "),
        // );

        // c! {{ctxs.iter().cloned().for_each(|ctx| print_state(self, ctx));}}
        // pause();


        let mut parent = self.parent(ctxs.first().cloned().unwrap_or(Ctx::Root));
        let first_ctx = ctxs.first().cloned().unwrap_or(Ctx::Root);
        // let len = self.get_ctx_length(ctxs.first().cloned().unwrap_or_default());
        let ctx_len = self.get_ctx_length(first_ctx);
        // c!{{
        //     eprintln!("{first_ctx:?} ctx_len: {}", ctx_len.as_u64());
        //     print_state(self, first_ctx);
        // }}
        // let len: Length = ctxs
        //     .first()
        //     .cloned()
        //     .and_then(|ctx| {
        //         // misc(state_str(self, ctx));
        //         // pause();

        //         Some(self.get_ctx_length(ctx))
        //     })
        //     .unwrap_or_default();

        // log(format!("LEN: {}", len.as_u64()));
        // pause();
        //
        c! {{
            print_state(self, first_ctx);
        }}

        self.call_thunks(parent, first_ctx, LifeCycleEvent::PreSequence);

        // c! {{
        //     print_state(self, first_ctx);
        //     pause();
        // }}

        ctxs.iter().cloned().for_each(|ctx| {
            // c! {{
            //     eprintln!("{ctx:?} {ctx_len:?}");
            //     // print_state(self, ctx);
            // }}
            self.cycle_fill(ctx, ctx_len.clone());
            // c! {
            //     {
            //         print_state(self, ctx);
            //         let ctx_len = self.get_ctx_length(ctx);
            //         eprintln!("{ctx:?} ctx_len: {}", ctx_len.as_u64());
            //         // pause();
            //     }
            // }

            // if matches!(
            //     (self.scope(parent), self.scope(ctx)),
            //     (Scope::Stack, Scope::Sequence)
            // ) {
            //     self.call_thunks(self.parent(parent), ctx, LifeCycleEvent::PreSequence);
            // }

            self.call_thunks(parent, ctx, LifeCycleEvent::PreSequence);

            // c! {
            //     {
            //         print_state(self, ctx);
            //         pause();
            //     }
            // }
        });

        let start = self.playhead().clone();
        // dbg!(&ctxs);
        // eprint!("{}", color!());
        // dbg!(start.as_u64());
        // pause();
        let mut counters = Vec::<Length>::new();
        let mut counter = start.clone();
        let mut len = Length::default();

        // eprintln!("");

        ctxs.iter().cloned().for_each(|ctx| {
            // c! {{
            //     eprintln!("{ctx:?}");
            // }}
            let lengths = self.lengths(ctx).unwrap_or_default();
            let sum = lengths
                .iter()
                .cloned()
                .fold(Length::default(), |sum, lengths_| {
                    sum + lengths_.iter().cloned().min().unwrap_or_default()
                });
            // dbg!(ctx, lengths.len(), sum.as_u64());

            // dbg!(self.get_ctx_length(ctx).as_u64());
        });
        // pause();

        ctxs.iter().cloned().for_each(|ctx| {
            // print_state(self, ctx);

            // pause();
            let lengths = self.lengths(ctx).unwrap_or_default();
            // let rows = lengths
            //     .iter()
            //     .fold(0, |max, lengths| max.max(lengths.len()));
            counter = start.clone();

            len = Length::default();

            // dbg!(
            //     lengths.len(),
            //     self.pcs(ctx).unwrap_or_default().len(),
            //     self.registers(ctx).unwrap_or_default().len(),
            //     self.velocities(ctx).unwrap_or_default().len(),
            //     self.tempos(ctx).unwrap_or_default().len(),
            //     self.programs(ctx).unwrap_or_default().len(),
            //     self.controllers(ctx).unwrap_or_default().len()
            // );
            // pause();

            zip(
                lengths.into_iter(),
                zip(
                    self.pcs(ctx).unwrap_or_default().into_iter(),
                    zip(
                        self.registers(ctx).unwrap_or_default().into_iter(),
                        zip(
                            self.velocities(ctx).unwrap_or_default().into_iter(),
                            zip(
                                self.tempos(ctx).unwrap_or_default().into_iter(),
                                zip(
                                    self.programs(ctx).unwrap_or_default().into_iter(),
                                    self.controllers(ctx).unwrap_or_default().into_iter(),
                                ),
                            ),
                        ),
                    ),
                ),
            )
            .for_each(
                |(lengths, (pcs, (registers, (velocities, (tempos, (programs, controllers))))))| {
                    // eprint!(
                    //     "{Yellow}{} ",
                    //     lengths
                    //         .first()
                    //         .cloned()
                    //         .unwrap_or_default()
                    //         .to_note(&tempos.first().cloned().unwrap_or_default())
                    // );

                    counter += len.clone();
                    // c! {{
                    //     eprint!("{ctx:?} start: {}, counter: {} pcs: {pcs:?} regs: {registers:?}", start.as_f64(), counter.as_f64());
                    // }}
                    // pause();

                    let slice = Slice::new(
                        pcs.clone(),
                        registers.clone(),
                        lengths.clone(),
                        velocities.clone(),
                        tempos.clone(),
                        programs.clone(),
                        controllers.clone(),
                    );

                    if !self.timeline_.contains_key(&counter) {
                        self.timeline_.insert(counter.clone(), vec![]);
                    }
                    let slices = self.timeline_.entry(counter.clone()).or_default();
                    slices.push(slice.clone());

                    len = lengths.iter().cloned().min().unwrap_or_default();

                    // c! {
                    //     {
                    //         eprintln!("{}: {:?}", counter.as_f64(), self.timeline_.get(&counter).cloned().unwrap_or_default().iter().map(|slice| format!("{:?}", slice.pcs)).collect::<Vec<_>>());
                    //         pause();
                    //     }
                    // }
                },
            );

            // self.timeline_.iter().for_each(|(time, slices)| {
            //     // eprint!("T: {:<13}", time.as_u64());
            //     slices.iter().for_each(|slice| {
            //         eprintln!(
            //             "{}",
            //             zip(
            //                 slice.lengths.iter(),
            //                 zip(slice.tempos.iter(), slice.pcs.iter())
            //             )
            //             .map(|(len, (tmp, pc))| format!(
            //                 "{}",
            //                 if matches!(pc, Pc::Class(_)) {
            //                     len.clone().to_note(tmp)
            //                 } else {
            //                     len.clone().to_rest(tmp)
            //                 }
            //             ))
            //             .collect::<Vec<_>>()
            //             .join(" ")
            //         );
            //     });
            // });
            // pause();
        });

        // pause();
        counter += len.clone();

        let ctx_len = self.get_ctx_length(parent);
        // *self.playhead() += ctx_len;
        *self.playhead() = counter.clone();

        // dbg!(&counter, self.playhead());
    }

    fn emplace<T: Clone + Default + Debug>(
        &mut self,
        ts: &Vec<Vec<T>>,
        ctx: Ctx,
        col: usize,
        row: usize,
        into_col: usize,
        f: fn(&mut Self, Ctx) -> Option<&mut Vec<Vec<T>>>,
    ) {
        f(self, ctx).and_then(|ts_| {
            let t = ts
                .get(col)
                .cloned()
                .and_then(|ts_| ts_.get(row).cloned())
                .unwrap_or_default();
            ts_.get_mut(into_col)
                .and_then(|ts__| {
                    ts__.insert(row, t.clone());
                    Some(())
                })
                .or_else(|| {
                    ts_.insert(into_col, vec![t]);
                    Some(())
                })
        });

        // .or_else(|| {
        //     Some(ts_.push_mut(Vec::<T>::new())).and_then(|ts__| {
        //         ts__.extend(
        //             ts.get(col)
        //                 .cloned()
        //                 .and_then(|ts_| ts_.get(row).cloned())
        //                 .unwrap_or_default(),
        //         );
        //         Some(())
        //     })
        // })
    }

    // misc(state_str(self, ctx));
    // pause();

    #[inline(always)]
    fn take_slice(&mut self, mut from: Ctx, row: usize, col: usize) -> Slice {
        // dbg!();

        Slice::new(
            self.pcs(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.registers(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.lengths(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.velocities(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.tempos(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.programs(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
            self.controllers(from)
                .and_then(|ts| ts.get(col).cloned())
                .unwrap_or_default(),
        )
    }

    pub fn pedal(&self, ch: u8) -> bool {
        self.pedal.get(&ch).cloned().unwrap_or_default()
    }

    pub fn toggle_pedal(&mut self, ch: u8) {
        let pedal = self.pedal.entry(ch).or_insert(false);
        // eprintln!("toggle_pedal({ch}) pedal: {pedal}");
        *pedal = !*pedal;
        // eprintln!("toggle_pedal({ch}) pedal: {pedal}");
        // pause();
    }
}
