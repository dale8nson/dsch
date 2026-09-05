use crossterm::{cursor::*, execute, style::*, terminal::*};
use num_rational::BigRational;
use pest::state;

use crate::{
    color,
    compiler::{
        codegen::{
            Data,
            utils::{TextStyle::*, *},
            *,
        },
        error,
    },
};
use std::{
    any::{Any, TypeId},
    backtrace,
    collections::{BTreeMap, HashMap, HashSet},
    convert,
    io::stderr,
    iter::{Cloned, Enumerate, Map, repeat_n, zip},
    ops::{DerefMut, Index, IndexMut},
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

        info(format!("{}APPEND CHILD {parent:?} -> {ctx:?}", color!(),));

        self.set_scope(ctx, Scope::None);
        self.programs_.insert(ctx, Vec::new());
        self.layers.insert(ctx, Layer::Heterogenous);
        self.registers_.insert(ctx, Vec::new());
        self.pcs_.insert(ctx, Vec::new());
        self.lengths_.insert(ctx, Vec::new());
        self.velocities_.insert(ctx, Vec::new());
        self.tempos_.insert(ctx, Vec::new());
        self.bindings.insert(ctx, HashMap::new());

        while let Some((event, thunk)) = self.thunk_stack.pop() {
            self.add_thunk(ctx, event, thunk);
        }

        // graph(self, Ctx::Root, 2);

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
        print_stacks(self);
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

    pub fn programs_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Prog>>> {
        self.programs_.get_mut(&ctx)
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
    pub fn add_programs(&mut self, ctx: Ctx, programs: Vec<Prog>) {
        if let Some(programs_) = self.programs_.get_mut(&ctx) {
            programs_.push(programs);
        } else {
            self.programs_.insert(ctx, vec![programs]);
        }
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

    pub fn registers_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Register>>> {
        self.registers_.get_mut(&ctx)
    }

    pub fn add_register(&mut self, ctx: Ctx, register: Register) {
        // let pc_len = self.pcs(ctx).unwrap_or_default().len();
        // let register_len = self.registers(ctx).len();

        if let Some(registers) = self.registers_.get_mut(&ctx) {
            registers.push(vec![register]);
        }
    }

    #[inline(always)]
    pub fn add_registers(&mut self, ctx: Ctx, registers: Vec<Register>) {
        if let Some(registers_) = self.registers_.get_mut(&ctx) {
            registers_.push(registers);
        } else {
            self.registers_.insert(ctx, vec![registers]);
        }
    }

    #[inline(always)]
    pub fn pcs(&self, ctx: Ctx) -> Option<Vec<Vec<Pc>>> {
        self.pcs_.get(&ctx).cloned()
    }

    pub fn pcs_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Pc>>> {
        self.pcs_.get_mut(&ctx)
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
    pub fn add_pcs(&mut self, ctx: Ctx, pcs: Vec<Pc>) {
        if let Some(pcs_) = self.pcs_.get_mut(&ctx) {
            pcs_.push(pcs);
        } else {
            self.pcs_.insert(ctx, vec![pcs]);
        }
    }

    #[inline(always)]
    pub fn velocities(&self, ctx: Ctx) -> Option<Vec<Vec<Velocity>>> {
        self.velocities_.get(&ctx).cloned()
    }

    pub fn velocities_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Velocity>>> {
        self.velocities_.get_mut(&ctx)
    }

    pub fn velocity(&self) -> Option<Vec<Velocity>> {
        self.velocity.clone()
    }

    pub fn set_velocity(&mut self, velocity: Velocity) {
        self.velocity = Some(vec![velocity]);
    }

    #[inline(always)]
    pub fn add_velocities(&mut self, ctx: Ctx, velocities: Vec<Velocity>) {
        // dbg!(&velocities);
        // trace(TextStyle::IntenseBoldCyan);
        if let Some(velocities_) = self.velocities_.get_mut(&ctx) {
            velocities_.push(velocities.clone());
        } else {
            self.velocities_.insert(ctx, vec![velocities.clone()]);
        }

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
        log(format!(
            "{}ADD {ts:?} TO {ctx:?} {:?}\n",
            color!(),
            self.scope(ctx)
        ));
        // trace(TextStyle::BoldPurple);
        // pause();

        match self.scope(ctx) {
            Scope::Sequence => match <T as Into<Data>>::into(T::default()) {
                Data::Pc(_) => {
                    self.add_pcs(ctx, convert_vec::<Data, Pc>(convert_vec::<T, Data>(ts)))
                }
                Data::Length(_) => {
                    self.add_lengths(ctx, convert_vec::<Data, Length>(convert_vec::<T, Data>(ts)));
                }
                Data::Velocity(_) => self.add_velocities(
                    ctx,
                    convert_vec::<Data, Velocity>(convert_vec::<T, Data>(ts)),
                ),
                Data::Tempo(_) => {
                    self.add_tempos(ctx, convert_vec::<Data, Tempo>(convert_vec::<T, Data>(ts)))
                }
                Data::Register(_) => self.add_registers(
                    ctx,
                    convert_vec::<Data, Register>(convert_vec::<T, Data>(ts)),
                ),
                Data::Interpolation(_) => todo!(),
                Data::Program(_) => {
                    self.add_programs(ctx, convert_vec::<Data, Prog>(convert_vec::<T, Data>(ts)))
                }
                Data::None => todo!(),
            },
            Scope::Stack => self.extend(ctx, vec![ts]),
            Scope::Set => todo!(),
            Scope::None => {
                trace(color!());

                // pause();
                todo!()
            }
        }
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
                    .map(|ts| {
                        ts.into_iter()
                            .map(|t| <T as Into<Data>>::into(t))
                            .map(|d| <Length as From<Data>>::from(d))
                            .collect()
                    })
                    .collect();
                if let Some(ts_) = self.lengths_.get_mut(&ctx) {
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

    pub fn lengths_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Length>>> {
        self.lengths_.get_mut(&ctx)
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

    pub fn add_lengths(&mut self, ctx: Ctx, lengths: Vec<Length>) {
        // eprintln!("{}ADD LENGTHS: {lengths:?}", color!());
        if let Some(lengths_) = self.lengths_.get_mut(&ctx) {
            lengths_.push(lengths);
        } else {
            self.lengths_.insert(ctx, vec![lengths]);
        }
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

    pub fn tempos_mut(&mut self, ctx: Ctx) -> Option<&mut Vec<Vec<Tempo>>> {
        self.tempos_.get_mut(&ctx)
    }

    pub fn set_bpm(&mut self, bpm: Absolute) {
        let mpb = match bpm {
            Absolute::Float(float) => Ratio::<u64>::from_f64(60_000_000. / float).unwrap(),
            Absolute::UInt(uint) => Ratio::<u64>::from_f64(60_000_000. / uint as f64).unwrap(),
        };
        self.tempo = Mpb(mpb);
    }

    pub fn tempo(&self) -> Tempo {
        self.tempo.clone()
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

    pub fn add_tempos(&mut self, ctx: Ctx, tempos: Vec<Tempo>) {
        if let Some(tempos_) = self.tempos_.get_mut(&ctx) {
            tempos_.push(tempos);
        } else {
            self.tempos_.insert(
                ctx,
                vec![
                    tempos
                        .into_iter()
                        .map(|tempo| {
                            Mpb(tempo.0.clone() - tempo.0 % Ratio::<u64>::from_u64(128).unwrap())
                        })
                        .collect(),
                ],
            );
        }
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

    pub fn thunks_mut(&mut self, ctx: Ctx) -> Option<&mut HashMap<LifeCycleEvent, Vec<Thunk>>> {
        self.thunks.get_mut(&ctx)
    }

    pub fn push_thunk(&mut self, event: LifeCycleEvent, thunk: Thunk) {
        self.thunk_stack.push((event, thunk));
    }

    pub fn add_thunk(&mut self, ctx: Ctx, life_cycle_event: LifeCycleEvent, thunk: Thunk) {
        misc(format!("{:?}", self.thunks(ctx)));
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
        misc(format!("{:?}", self.thunks(ctx)));
        // pause();
    }

    pub fn call_thunks(&mut self, ctx: Ctx, life_cycle_event: LifeCycleEvent) {
        let thunk_map = self.thunks.get(&ctx).cloned().unwrap_or_default();
        if thunk_map.is_empty() {
            return;
        }
        eprintln!("{}CALL THUNKS: {ctx:?} {:?}", color!(), self.scope(ctx));
        // pause();
        let thunks = thunk_map
            .get(&life_cycle_event)
            .cloned()
            .unwrap_or_default();
        // thunks.iter().for_each(|thunk| eprintln!("{thunk:?}"));
        thunks.iter().cloned().for_each(|mut thunk| {
            thunk.call(ctx, self);
            // match thunk
            //     .0
            //     .first()
            //     .cloned()
            //     .unwrap_or_default()
            //     .first()
            //     .cloned()
            //     .unwrap_or_default()
            // {
            //     Data::Pc(_) => thunk.call::<Pc>(ctx, self),
            //     Data::Length(_) => thunk.call::<Length>(ctx, self),
            //     Data::Velocity(_) => thunk.call::<Velocity>(ctx, self),
            //     Data::Tempo(_) => thunk.call::<Mpb>(ctx, self),
            //     Data::Register(_) => thunk.call::<Register>(ctx, self),
            //     Data::Interpolation(_) => todo!(),
            //     Data::None => todo!(),
            //     Data::Program(prog) => todo!(),
            // }
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
        color!();
        info(format!(
            "{}CYCLE FILL {ctx:?} LEN: {}",
            color!(),
            len.as_u64()
        ));
        print_states(self, ctx);
        // misc(state_str(self, ctx));
        // pause();

        let children_len = self.children(ctx).len();
        if children_len > 0 {
            let clone = self.children(ctx);
            let mut cycle = clone.iter().cloned().cycle();
            while self.get_ctx_length(ctx) <= len {
                self.children_mut(ctx)
                    .unwrap()
                    .extend(cycle.by_ref().take(1));
            }
        } else {
            let len_clone = self.lengths(ctx).unwrap_or_default();
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

            while self.get_ctx_length(ctx) < len {
                eprintln!(
                    "{}T:{}:{}",
                    color!(),
                    self.get_ctx_length(ctx).as_u64(),
                    len.as_u64()
                );
                print_state(self, ctx);
                // pause();

                self.add::<Length>(ctx, len_cycle.by_ref().next().unwrap_or_default());
                self.add::<Prog>(ctx, prog_cycle.by_ref().next().unwrap_or_default());
                self.add::<Register>(ctx, reg_cycle.by_ref().next().unwrap_or_default());
                self.add::<Pc>(ctx, pc_cycle.by_ref().next().unwrap_or_default());
                self.add::<Velocity>(ctx, vel_cycle.by_ref().next().unwrap_or_default());
                self.add::<Tempo>(ctx, temp_cycle.by_ref().next().unwrap_or_default());
            }
        }
        print_states(self, ctx);

        // pause();
        // misc(format!("{}{}", color!(), state_str(self, ctx)));
        // pause();
        // eprint!("{}", color!());
        // dbg!();
        // misc(state_str(self, ctx));
    }

    pub fn get_last_mut<'a, T>(&'a mut self, ctx: Ctx) -> Option<&'a mut T>
    where
        T: Interpolant,
    {
        let mut ts = T::get_vec_mut(self, ctx);
        ts?.last_mut()?.last_mut()
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
                        .and_then(|ts| {
                            // misc(state_str(self, ctx_));
                            // self.length = Some(ts.clone());
                            Some(convert_vec::<Length, Data>(ts))
                        }),
                    // .or(self.length.clone().and_then(|ts| {
                    //     // self.length = Some(ts.clone());
                    //     Some(convert_vec::<Length, Data>(ts))
                    // })
                    Data::Velocity(_) => self
                        .velocities(ctx_)
                        .unwrap_or_default()
                        .last()
                        .cloned()
                        .and_then(
                            |ts| {
                                // self.velocity = Some(ts.clone());
                                Some(convert_vec::<Velocity, Data>(ts))
                            }, // )
                               // .or(self.velocity.clone().and_then(|ts| {
                               //     self.velocity = Some(ts.clone());
                               //     Some(convert_vec::<Velocity, Data>(ts))
                               // })
                        ),
                    Data::Tempo(_) =>
                    // Some(vec![Data::from(self.tempo)]),
                    {
                        self.tempos(ctx_)
                            .unwrap_or_default()
                            .last()
                            .cloned()
                            .and_then(|ts| Some(convert_vec::<Tempo, Data>(ts)))
                    }
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
                        .and_then(
                            |ts| {
                                // self.program = Some(ts.clone());
                                Some(convert_vec::<Prog, Data>(ts))
                            }, // )
                               // .or(self.program.clone().and_then(|ts| {
                               //     self.program = Some(ts.clone());
                               //     Some(convert_vec::<Prog, Data>(ts))
                               // })
                        ),
                    Data::None => todo!(),
                } {
                    // misc(state_str(self, ctx_));
                    data = Some(ts);
                }
            } else {
                data = match Data::from(default.clone().into()) {
                    Data::Pc(_) => self
                        .pcs_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Pc, Data>(ts_))
                        }),
                    Data::Length(_) => self
                        .lengths_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Length, Data>(ts_))
                        }),
                    Data::Velocity(_) => self
                        .velocities_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Velocity, Data>(ts_))
                        }),
                    Data::Tempo(_) => self
                        .tempos_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Tempo, Data>(ts_))
                        }),
                    Data::Register(_) => self
                        .registers_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Register, Data>(ts_))
                        }),
                    Data::Interpolation(_) => todo!(),
                    Data::Program(_) => self
                        .programs_mut(ctx_)
                        .and_then(|ts| ts.last().cloned())
                        .and_then(|ts_| {
                            // self.add(ctx, ts_.clone());
                            Some(convert_vec::<Prog, Data>(ts_))
                        }),
                    Data::None => todo!(),
                };

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
            vec![Data::from(default.into())]
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
                    .unwrap_or(vec![Length::default()]),
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

    pub fn pad(&mut self, ctx: Ctx, len: usize) {
        // eprintln!("{}PAD {ctx:?} LEN: {len}", color!(),);
        // eprint!("{}", color!());
        // misc(state_str(self, ctx));
        //

        self.pad_(ctx, Self::programs_mut, Self::programs, len);
        self.pad_(ctx, Self::velocities_mut, Self::velocities, len);
        self.pad_(ctx, Self::lengths_mut, Self::lengths, len);
        self.pad_(ctx, Self::registers_mut, Self::registers, len);
        self.pad_(ctx, Self::tempos_mut, Self::tempos, len);
    }

    fn pad_<T: Clone + Default + Debug + Into<Data> + From<Data> + ToString, F, G>(
        &mut self,
        ctx: Ctx,
        mut f: F,
        g: G,
        len: usize,
    ) where
        F: FnMut(&mut Self, Ctx) -> Option<&mut Vec<Vec<T>>>,
        G: Fn(&Self, Ctx) -> Option<Vec<Vec<T>>>,
        Data: From<T>,
    {
        // eprint!("{}", color!());
        // dbg!();
        // misc(state_str(self, ctx));
        let values: Vec<T> = self.get(ctx);
        // dbg!(ctx, &values);

        let (pc_index, ts_len) = (
            self.pcs(ctx).unwrap_or_default().len().max(1) - 1,
            g(self, ctx).unwrap_or_default().len(),
        );

        let pcs = self
            .pcs(ctx)
            .unwrap_or_default()
            .get(pc_index)
            .cloned()
            .unwrap_or_default();
        let pcs_len = pcs.len();
        let ts = g(self, ctx).unwrap_or_default();

        // dbg!(&pcs, &ts);
        let ts = ts.get(pc_index).cloned().unwrap_or_default();
        let t_len = ts.len();
        if t_len < pcs_len {
            // dbg!(pc_index, len, t_len);
            // misc(state_str(self, ctx));
            let take = pcs_len - t_len;

            self.add(ctx, values.iter().cloned().cycle().take(take).collect());

            // misc(state_str(self, ctx));
        }
        // }

        // eprint!("{}", color!());
        // dbg!();
        misc(state_str(self, ctx));
        // pause();
        // trace(TextStyle::IntenseBoldGreen);
    }

    pub fn hoist(&mut self, from: Ctx, to: Ctx) {
        // eprintln!("{}HOIST {from:?} {:?}", color!(), self.scope(from));
        // eprint!("{}", color!());
        // misc(state_str(self, from));
        // misc(state_str(self, to));
        let children = self.children(from);
        // if children.len() > 0 {
        // self.fun_name(ctx);

        // children.into_iter().for_each(|ctx| self.hoist(ctx));
        // }

        // dbg!(parent);
        let programs = std::mem::take(self.programs_mut(from).unwrap_or(&mut [].to_vec().as_mut()));
        let pcs = std::mem::take(self.pcs_mut(from).unwrap_or(&mut [].to_vec().as_mut()));
        let velocities = std::mem::take(
            self.velocities_mut(from)
                .unwrap_or(&mut [].to_vec().as_mut()),
        );
        // dbg!(&velocities);
        let registers = std::mem::take(
            self.registers_mut(from)
                .unwrap_or(&mut [].to_vec().as_mut()),
        );
        let lengths = std::mem::take(self.lengths_mut(from).unwrap_or(&mut [].to_vec().as_mut()));
        // dbg!(&lengths);

        let tempos = std::mem::take(self.tempos_mut(from).unwrap_or(&mut [].to_vec().as_mut()));
        // dbg!(&tempos);

        zip(
            pcs.into_iter(),
            zip(
                programs.into_iter(),
                zip(
                    registers.into_iter(),
                    zip(
                        velocities.into_iter(),
                        zip(lengths.into_iter(), tempos.into_iter()),
                    ),
                ),
            ),
        )
        .for_each(
            |(pcs, (programs, (registers, (velocities, (lengths, tempos)))))| {
                self.add_pcs(to, pcs);
                let index = self.pcs(to).unwrap_or_default().len() - 1;
                self.registers_mut(to).and_then(|regs| {
                    if regs.is_empty() {
                        regs.push(registers);
                    } else {
                        regs.get_mut(index)
                            .and_then(|regs| {
                                *regs = registers.clone();
                                Some(())
                            })
                            .or_else(|| {
                                regs.push(registers);
                                Some(())
                            });
                    }
                    Some(())
                });
                self.lengths_mut(to).and_then(|lens| {
                    // dbg!(&lens);
                    if lens.is_empty() {
                        lens.push(lengths);
                    } else {
                        lens.get_mut(index)
                            .and_then(|lens| {
                                // dbg!(&lens);
                                *lens = lengths.clone();
                                Some(())
                            })
                            .or_else(|| {
                                lens.push(lengths);
                                Some(())
                            });
                    }
                    Some(())
                });

                self.velocities_mut(to).and_then(|vels| {
                    if vels.is_empty() {
                        vels.push(velocities);
                    } else {
                        vels.get_mut(index)
                            .and_then(|vels| {
                                *vels = velocities.clone();
                                Some(())
                            })
                            .or_else(|| {
                                vels.push(velocities);
                                Some(())
                            });
                    }
                    Some(())
                });
                self.tempos_mut(to).and_then(|temps| {
                    if temps.is_empty() {
                        temps.push(tempos);
                    } else {
                        temps
                            .get_mut(index)
                            .and_then(|temps| {
                                *temps = tempos.clone();
                                Some(())
                            })
                            .or_else(|| {
                                temps.push(tempos);
                                Some(())
                            });
                    }
                    Some(())
                });

                self.programs_mut(to).and_then(|progs| {
                    if progs.is_empty() {
                        progs.push(programs);
                    } else {
                        progs
                            .get_mut(index)
                            .and_then(|progs| {
                                *progs = programs.clone();
                                Some(())
                            })
                            .or_else(|| {
                                progs.push(programs);
                                Some(())
                            });
                    }
                    Some(())
                });
            },
        );

        // self.children_mut(parent)
        //     .unwrap_or(Vec::new().as_mut())
        //     .extend(children);
        self.move_children(from, to);
        // self.thunks(parent).get_mut(&LifeCycleEvent::Sequencing).unwrap_or(&mut[].to_vec().as_mut()).extend(thunks);

        // self.drop(from);
    }

    pub fn clean_context(&mut self, ctx: Ctx) {
        self.programs_mut(ctx).and_then(|programs| {
            programs.clear();
            Some(())
        });
        self.pcs_mut(ctx).and_then(|pcs| {
            pcs.clear();
            Some(())
        });
        self.velocities_mut(ctx).and_then(|velocities| {
            velocities.clear();
            Some(())
        });
        self.registers_mut(ctx).and_then(|registers| {
            registers.clear();
            Some(())
        });
        self.lengths_mut(ctx).and_then(|lengths| {
            lengths.clear();
            Some(())
        });
        self.tempos_mut(ctx).and_then(|tempos| {
            tempos.clear();
            Some(())
        });
    }

    pub fn flatten_stack(&mut self, ctx: Ctx) {
        // misc(state_str(self, ctx));
        if let Some(programs) = self.programs_mut(ctx) {
            flatten(programs);
        }
        if let Some(pcs) = self.pcs_mut(ctx) {
            flatten(pcs);
            // misc(state_str(self, ctx));
            // if ctx.to_usize() == 1 {
            //     pause()
            // }
        }
        if let Some(velocities) = self.velocities_mut(ctx) {
            flatten(velocities)
        }
        if let Some(registers) = self.registers_mut(ctx) {
            flatten(registers);
        }
        if let Some(tempos) = self.tempos_mut(ctx) {
            flatten(tempos);
        }
        if let Some(lengths) = self.lengths_mut(ctx) {
            flatten(lengths)
        }

        // misc(state_str(self, ctx));
        // if ctx.to_usize() == 1 {
        //     pause();
        // }
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
        let children = self.children(ctx);
        if children.len() > 0 {
            let mut iter = children.iter().cloned();
            let init = match self.scope(iter.next().unwrap()) {
                Scope::Sequence => self.length_sum(iter.next().unwrap_or_default()),
                Scope::Stack => self.min_length(iter.next().unwrap_or_default()),
                _ => todo!(),
            };
            iter.fold(init, |total, ctx_| {
                // dbg!(&total);
                match self.scope(ctx) {
                    Scope::Sequence => total + self.get_ctx_length(ctx_),
                    Scope::Stack => total.min(self.get_ctx_length(ctx_)),
                    Scope::Set => todo!(),
                    Scope::None => Length::None,
                }
            })
        } else {
            // dbg!();
            // misc(state_str(self, ctx));
            match self.scope(ctx) {
                Scope::Sequence => {
                    let sum = self.length_sum(ctx);
                    // dbg!(&sum);
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
        let children = self.children(ctx);
        if children.len() > 0 {
            children
                .into_iter()
                .fold(Length::default(), |sum, ctx| sum + self.length_sum(ctx))
        } else {
            // misc(state_str(self, ctx));
            self.lengths(ctx)
                .unwrap_or_default()
                .iter()
                .map(|lengths| lengths.iter().cloned().max().unwrap())
                .sum()
        }
    }

    pub fn min_length(&self, ctx: Ctx) -> Length {
        let children = self.children(ctx);
        if children.len() > 0 {
            children
                .into_iter()
                .fold(Length::default(), |sum, ctx| self.get_ctx_length(ctx))
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
        status(format!(
            "{}COMBINE SEQUENCES {}",
            color!(),
            ctxs.iter()
                .cloned()
                .map(|ctx| format!("{ctx:?}").to_uppercase())
                .collect::<Vec<String>>()
                .join(", "),
        ));

        let len: Length = ctxs
            .first()
            .cloned()
            .and_then(|ctx| {
                // compose_thunks(ctx, ctx, self);
                // self.call_thunks(ctx, LifeCycleEvent::Sequencing);
                misc(state_str(self, ctx));
                // pause();
                Some(self.get_ctx_length(ctx))
            })
            .unwrap_or_default();

        log(format!("LEN: {}", len.as_u64()));
        // pause();

        ctxs.iter().cloned().for_each(|ctx| {
            dbg!();
            print_state(self, ctx);
            // pause();
            // self.call_thunks(ctx, LifeCycleEvent::Sequencing);
            self.cycle_fill(ctx, len.clone());
            // self.call_thunks(ctx, LifeCycleEvent::Sequencing);
            dbg!();
            print_state(self, ctx);
            // pause();
        });

        let start = self.playhead().clone();
        dbg!(start.as_u64());
        // pause();
        let mut counters = Vec::<Length>::new();
        let mut len = Length::default();

        ctxs.iter().cloned().for_each(|ctx| {
            dbg!();
            print_state(self, ctx);
            // pause();
            let lengths = self.lengths(ctx).unwrap_or_default();
            let rows = lengths
                .iter()
                .fold(0, |max, lengths| max.max(lengths.len()));
            let mut counter = start.clone();
            len = Length::default();
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
                                self.programs(ctx).unwrap_or_default().into_iter(),
                            ),
                        ),
                    ),
                ),
            )
            .for_each(
                |(lengths, (pcs, (registers, (velocities, (tempos, programs)))))| {
                    counter += len.clone();

                    // dbg!(
                    //     lengths.len(),
                    //     pcs.len(),
                    //     registers.len(),
                    //     velocities.len(),
                    //     tempos.len(),
                    //     programs.len()
                    // );
                    // dbg!(&counter);
                    // pause();
                    let slice = Slice::new(
                        pcs.clone(),
                        registers.clone(),
                        lengths.clone(),
                        velocities.clone(),
                        tempos.clone(),
                        programs.clone(),
                    );

                    if !self.timeline_.contains_key(&counter) {
                        self.timeline_.insert(counter.clone(), vec![]);
                    }
                    self.timeline_.get_mut(&counter).and_then(|slices| {
                        slices.push(slice);
                        Some(())
                    });

                    len = lengths.iter().cloned().min().unwrap_or_default();
                },
            );

            counter += len.clone();

            counters.push(counter);
            // dbg!(&counters);
            // pause();
        });

        let len = counters.iter().cloned().min().unwrap_or_default();
        dbg!(&len);
        // pause();
        // let len = self.get_ctx_length(self.parent(ctxs.first().cloned().unwrap_or_default()));
        *self.playhead() = len;
        ctxs.into_iter().for_each(|ctx| self.drop(ctx));
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
        )
    }
}
