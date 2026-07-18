use crate::compiler::codegen::{
    utils::{TextStyle::*, *},
    *,
};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    iter::{Cloned, Enumerate, repeat_n, zip},
    slice::Iter,
    vec::IntoIter,
};

#[derive(Debug)]
pub struct State {
    index: usize,
    exps: Vec<Exp>,
    ctx_count: usize,
    ctxs: HashSet<Ctx>,
    parents: HashMap<Ctx, Ctx>,
    children: HashMap<Ctx, Vec<Ctx>>,
    scopes: HashMap<Ctx, Scope>,
    programs: HashMap<Ctx, Vec<Prog>>,
    registers: HashMap<Ctx, Vec<Register>>,
    pcs: HashMap<Ctx, Vec<Pc>>,
    lengths: HashMap<Ctx, Vec<Length>>,
    velocities: HashMap<Ctx, Vec<Velocity>>,
    tempos: HashMap<Ctx, Vec<Mpb>>,
    bindings: HashMap<Ctx, HashMap<Ident, Exp>>,
    timeline: Vec<(u64, Ctx)>,
    playhead: Length,
    stack: Vec<Ctx>,
}

impl Default for State {
    fn default() -> Self {
        let exps = Vec::new();

        Self {
            index: 0,
            exps,
            ctx_count: 0,
            ctxs: HashSet::from_iter(vec![]),
            parents: HashMap::new(),
            children: HashMap::from_iter(vec![]),
            scopes: HashMap::from_iter(vec![]),
            programs: HashMap::from_iter(vec![]),
            registers: HashMap::from_iter(vec![]),
            pcs: HashMap::from_iter(vec![]),
            lengths: HashMap::from_iter(vec![]),
            velocities: HashMap::from_iter(vec![]),
            tempos: HashMap::from_iter(vec![]),
            bindings: HashMap::from_iter(vec![]),
            timeline: Vec::new(),
            playhead: Length::MicroSeconds(0),
            stack: Vec::new(),
        }
    }
}

impl Iterator for State {
    type Item = Exp;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.exps.len() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            self.exps.get(index).cloned()
        }
    }
}

impl ExactSizeIterator for State {
    fn len(&self) -> usize {
        self.exps.len() - self.index
    }
}

impl State {
    pub fn append_child(&mut self, parent: Ctx) -> Ctx {
        let ctx = if self.ctxs.is_empty() {
            self.ctxs.insert(parent);
            self.ctx_count += 1;
            self.set_parent(parent, Ctx::None);
            parent
        } else {
            let ctx = self.add_ctx();
            self.set_parent(ctx, parent);
            ctx
        };



        eprintln!(
            "{}APPEND CHILD {parent:?} -> {ctx:?}{}",
            TextStyle::IntenseBoldGreen,
            TextStyle::ResetColor
        );

        self.set_scope(ctx, Scope::None);
        self.programs.insert(ctx, Vec::new());
        self.registers.insert(ctx, Vec::new());
        self.pcs.insert(ctx, Vec::new());
        self.lengths.insert(ctx, Vec::new());
        self.velocities.insert(ctx, Vec::new());
        self.tempos.insert(ctx, Vec::new());
        self.bindings.insert(ctx, HashMap::new());

        // print_state(self, ctx);
        ctx
    }

    fn add_ctx(&mut self) -> Ctx {
        let id = self.ctx_count;
        self.ctx_count += 1;
        let ctx = Ctx::Id(id);
        self.ctxs.insert(ctx);
        ctx
    }

    pub fn exps(&self) -> Vec<Exp> {
        self.exps.clone()
    }

    pub fn set_exps(&mut self, exps: Vec<Exp>, index: usize) -> (Vec<Exp>, usize) {
        let exps = std::mem::replace(&mut self.exps, exps);
        let index_ = self.index;
        self.index = index;
        (exps, index_)
    }

    pub fn scope(&self, ctx: Ctx) -> Scope {
        self.scopes.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn set_scope(&mut self, ctx: Ctx, scope: Scope) {
        self.scopes.insert(ctx, scope);
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

    pub fn children(&self, ctx: Ctx) -> Vec<Ctx> {
        self.children.get(&ctx).cloned().unwrap_or_default()
    }

    fn add_child(&mut self, parent: Ctx, child: Ctx) {
        if let Some(children) = self.children.get_mut(&parent) {
            children.push(child);
        }
    }

    pub fn programs(&self, ctx: Ctx) -> Vec<Prog> {
        self.programs.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_program(&mut self, ctx: Ctx, program: Prog) {
        let pc_len = self.pcs(ctx).len();
        let program_len = self.programs(ctx).len();
        if let Some(programs) = self.programs.get_mut(&ctx) {
            if program_len > pc_len && (program_len - pc_len) >= 1 {
                programs.remove(0);
            }
            programs.push(program);
        }
    }

    pub fn registers(&self, ctx: Ctx) -> Vec<Register> {
        self.registers.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_register(&mut self, ctx: Ctx, register: Register) {
        let pc_len = self.pcs(ctx).len();
        let register_len = self.registers(ctx).len();

        if let Some(registers) = self.registers.get_mut(&ctx) {
            if (register_len - pc_len) >= 1 {
                registers.remove(0);
            }

            registers.push(register);
        }
    }

    pub fn pcs(&self, ctx: Ctx) -> Vec<Pc> {
        self.pcs.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_pc(&mut self, ctx: Ctx, pc: Pc) {
        if let Some(pcs) = self.pcs.get_mut(&ctx) {
            pcs.push(pc);
        }
    }

    pub fn add_pcs(&mut self, ctx: Ctx, pcs: Vec<Pc>) {
        if let Some(pcs_) = self.pcs.get_mut(&ctx) {
            pcs_.extend(pcs);
        }
    }

    pub fn velocities(&self, ctx: Ctx) -> Vec<Velocity> {
        self.velocities.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_velocity(&mut self, ctx: Ctx, velocity: Velocity) {
        if let Some(velocities) = self.velocities.get_mut(&ctx) {
            velocities.push(velocity);
        }
    }

    pub fn lengths(&self, ctx: Ctx) -> Vec<Length> {
        self.lengths.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_length(&mut self, ctx: Ctx, length: Length) {
        let pc_len = self.pcs(ctx).len();
        let length_len = self.lengths(ctx).len();

        if let Some(lengths) = self.lengths.get_mut(&ctx) {
            if length_len > pc_len && (length_len - pc_len) >= 1 {
                lengths.remove(0);
            }
            lengths.push(length)
        }
    }

    pub fn add_lengths(&mut self, ctx: Ctx, lengths: Vec<Length>) {
        if let Some(lengths_) = self.lengths.get_mut(&ctx) {
            lengths_.extend(lengths);
        }
    }

    pub fn tempos(&self, ctx: Ctx) -> Vec<Mpb> {
        self.tempos.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn add_tempo(&mut self, ctx: Ctx, tempo: Mpb) {
        let pc_len = self.pcs(ctx).len();
        let tempo_len = self.tempos(ctx).len();

        if let Some(tempos) = self.tempos.get_mut(&ctx) {
            if tempo_len > pc_len && (tempo_len - pc_len) >= 1 {
                tempos.remove(0);
            }
            tempos.push(tempo);
        }
    }

    pub fn bindings(&self, ctx: Ctx) -> HashMap<Ident, Exp> {
        self.bindings.get(&ctx).cloned().unwrap_or_default()
    }

    pub fn binding(&self, ctx: Ctx, ident: Ident) -> Exp {
        if let Some(bindings) = self.bindings.get(&ctx) {
            bindings.get(&ident).cloned().unwrap_or_default()
        } else {
            Exp::Noop
        }
    }

    pub fn add_binding(&mut self, ctx: Ctx, ident: Ident, exp: Exp) {
        if let Some(bindings) = self.bindings.get_mut(&ctx) {
            bindings.insert(ident, exp);
        }
    }

    pub fn add_bindings(&mut self, ctx: Ctx, bindings: HashMap<Ident, Exp>) {
        if let Some(bindings_) = self.bindings.get_mut(&ctx) {
            bindings_.extend(bindings);
        }
    }

    pub fn push(&mut self, ctx: Ctx) {
        self.stack.push(ctx);
    }

    pub fn pop(&mut self) -> Ctx {
        self.stack.pop().unwrap_or_default()
    }

    pub fn timeline(&mut self) -> &mut Vec<(u64, Ctx)> {
        &mut self.timeline
    }

    pub fn add_timeline_event(&mut self, ctx: Ctx) {
        let position = self.playhead.as_u64();
        self.timeline().push((position, ctx));
    }

    pub fn playhead(&mut self) -> &mut Length {
        &mut self.playhead
    }

    pub fn max_len(&self, ctx: Ctx) -> usize {
        vec![
            self.pcs(ctx).len(),
            self.lengths(ctx).len(),
            self.velocities(ctx).len(),
            self.registers(ctx).len(),
        ]
        .iter()
        .cloned()
        .max()
        .unwrap_or_default()
    }

    pub fn cycle_fill(&mut self, ctx: Ctx, len: usize) {
        if let Some(programs) = self.programs.get_mut(&ctx) {
            let program_len = programs.len();
            if len > program_len {
                let take: Vec<Prog> = programs
                    .iter()
                    .cloned()
                    .cycle()
                    .take(len - program_len)
                    .collect();
                programs.extend(take);
            }
        }
        if let Some(registers) = self.registers.get_mut(&ctx) {
            let register_len = registers.len();
            if len > register_len {
                let take: Vec<Register> = registers
                    .iter()
                    .cloned()
                    .cycle()
                    .take(len - register_len)
                    .collect();
                registers.extend(take);
            }
        }
        if let Some(pcs) = self.pcs.get_mut(&ctx) {
            let pc_len = pcs.len();
            if len > pc_len {
                let take: Vec<Pc> = pcs.iter().cloned().cycle().take(len - pc_len).collect();
                pcs.extend(take);
            }
        }
        if let Some(lengths) = self.lengths.get_mut(&ctx) {
            let length_len = lengths.len();
            if len > length_len {
                let take: Vec<Length> = lengths
                    .iter()
                    .cloned()
                    .cycle()
                    .take(len - length_len)
                    .collect();
                lengths.extend(take);
            }
        }
        if let Some(velocities) = self.velocities.get_mut(&ctx) {
            let velocity_len = velocities.len();
            if len > velocity_len {
                let take: Vec<Velocity> = velocities
                    .iter()
                    .cloned()
                    .cycle()
                    .take(len - velocity_len)
                    .collect();
                velocities.extend(take);
            }
        }
        if let Some(tempos) = self.tempos.get_mut(&ctx) {
            let tempo_len = tempos.len();
            if len > tempo_len {
                let take: Vec<Mpb> = tempos
                    .iter()
                    .cloned()
                    .cycle()
                    .take(len - tempo_len)
                    .collect();
                tempos.extend(take);
            }
        }
    }

    pub fn resize(&mut self, ctx: Ctx, len: usize) {
        if let Some(programs) = self.programs.get_mut(&ctx) {
            let program_len = programs.len();
            if len > program_len {
                programs.resize(len - program_len, Prog(0));
            }
        }
        if let Some(registers) = self.registers.get_mut(&ctx) {
            let register_len = registers.len();
            if len > register_len {
                registers.resize(len - register_len, Register::None);
            }
        }
        if let Some(pcs) = self.pcs.get_mut(&ctx) {
            let pc_len = pcs.len();
            if len > pc_len {
                pcs.resize(len - pc_len, Pc::None);
            }
        }
        if let Some(lengths) = self.lengths.get_mut(&ctx) {
            let length_len = lengths.len();
            if len > length_len {
                lengths.resize(len, Length::None);
            }
        }
        if let Some(velocities) = self.velocities.get_mut(&ctx) {
            let velocity_len = velocities.len();
            if len > velocity_len {
                velocities.resize(len - velocity_len, Velocity(0));
            }
        }
        if let Some(tempos) = self.tempos.get_mut(&ctx) {
            let tempo_len = tempos.len();
            if len > tempo_len {
                tempos.resize(len, Mpb(0));
            }
        }
    }

    pub fn pad(&mut self, ctx: Ctx, len: usize) {
        if let Some(programs) = self.programs.get_mut(&ctx) {
            let program_len = programs.len();
            if len > program_len {
                let last_program = programs.last().cloned().unwrap_or_default();
                programs.extend(repeat_n(last_program, len - program_len));
            }
        }

        if let Some(pcs) = self.pcs.get_mut(&ctx) {
            let pc_len = pcs.len();
            if len > pc_len {
                let last_velocity = pcs.last().cloned().unwrap_or_default();
                pcs.extend(repeat_n(last_velocity, len - pc_len));
            }
        }

        if let Some(lengths) = self.lengths.get_mut(&ctx) {
            let length_len = lengths.len();
            if len > length_len {
                let last_length = lengths.last().cloned().unwrap_or_default();
                lengths.extend(repeat_n(last_length, len - length_len));
            }
        }

        if let Some(registers) = self.registers.get_mut(&ctx) {
            let register_len = registers.len();
            if len > register_len {
                let last_register = registers.last().cloned().unwrap_or_default();
                registers.extend(repeat_n(last_register, len - register_len));
            }
        }

        if let Some(velocities) = self.velocities.get_mut(&ctx) {
            let velocity_len = velocities.len();
            if len > velocity_len {
                let last_velocity = velocities.last().cloned().unwrap_or_default();
                velocities.extend(repeat_n(last_velocity, len - velocity_len));
            }
        }

        if let Some(tempos) = self.tempos.get_mut(&ctx) {
            let tempo_len = tempos.len();
            if len > tempo_len {
                let last_tempo = tempos.last().cloned().unwrap_or_default();
                tempos.extend(repeat_n(last_tempo, len - tempo_len));
            }
        }
    }

    fn is_only(&self, ctx: Ctx, type_: Type) -> bool {
        match type_ {
            Type::Length => todo!(),
            Type::Length | Type::Pc => todo!(),
            _ => todo!(),
        }
    }

    pub fn take_last<T: Any + Default>(&mut self, ctx: Ctx) -> Data {
        let id = TypeId::of::<T>();
        if id == PC_ID {
            Data::Pc(std::mem::take(
                self.pcs
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Pc::None),
            ))
        } else if id == LENGTH_ID {
            Data::Length(std::mem::take(
                self.lengths
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Length::None),
            ))
        } else if id == VELOCITY_ID {
            Data::Velocity(std::mem::take(
                self.velocities
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Velocity::default()),
            ))
        } else if id == REGISTER_ID {
            Data::Register(std::mem::take(
                self.registers
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Register::None),
            ))
        } else if id == PROGRAM_ID {
            Data::Program(std::mem::take(
                self.programs
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Prog::default()),
            ))
        } else if id == TEMPO_ID {
            Data::Tempo(std::mem::take(
                self.tempos
                    .get_mut(&ctx)
                    .unwrap_or(&mut Vec::new())
                    .last_mut()
                    .unwrap_or(&mut Mpb::default()),
            ))
        } else {
            Data::None
        }
    }

    pub fn sequence(&mut self, ctx: Ctx) {
        eprintln!(
            "{IntenseYellow}SEQUENCE {ctx:?} {:?}{ResetColor}",
            self.scope(ctx)
        );
        graph(self, Ctx::Root);
        // print_state(self, ctx);
        let children = self.children(ctx);
        // if children.len() > 0 {
            match self.scope(ctx) {
                Scope::Sequence => {
                    if children.len() > 0 {
                        children.iter().cloned().for_each(|ctx| self.sequence(ctx))
                    } else {
                        self.combine_sequences(vec![ctx]);
                    }
                }
                Scope::Stack => {
                    let mut iter = children.iter().cloned();
                    let mut ctx_ = iter.next();
                    while let Some(mut ctx__) = ctx_ {
                        let mut scope = self.scope(ctx__);
                        match scope {
                            Scope::Sequence => {
                                self.set_scope(ctx, Scope::Sequence);
                                let mut ctxs: Vec<Ctx> = vec![ctx__];
                                ctxs.extend(
                                    iter.by_ref()
                                        .take_while(|ctx| self.scope(*ctx) == Scope::Sequence)
                                        .collect::<Vec<Ctx>>()
                                        .iter(),
                                );
                                dbg!(&ctxs);
                                // ctxs.iter().cloned().for_each(|ctx| self.sequence(ctx));
                                self.combine_sequences(ctxs);
                            }
                            Scope::Stack => {
                                self.sequence(ctx__);
                            }
                            Scope::Set => todo!(),
                            Scope::None => todo!(),
                        }
                        ctx_ = iter.next();
                    }
                }
                Scope::Set => todo!(),
                Scope::None => todo!(),
            }

        self.bindings.remove(&ctx);
        self.tempos.remove(&ctx);
        self.velocities.remove(&ctx);
        self.lengths.remove(&ctx);
        self.pcs.remove(&ctx);
        self.registers.remove(&ctx);
        self.programs.remove(&ctx);
        self.children.remove(&ctx);
        let parent = self.parents.get(&ctx).cloned().unwrap_or_default();
        if let Some(siblings) = self.children.get_mut(&parent) {
            *siblings = siblings.extract_if(.., |ctx_| *ctx_ == ctx).collect();
        }
        self.ctxs.remove(&ctx);
        graph(self, Ctx::Root);
    }

    pub fn min_length(&self, ctx: Ctx) -> Length {
        self.lengths(ctx).iter().cloned().min().unwrap_or_default()
    }

    pub fn combine_sequences(&mut self, ctxs: Vec<Ctx>) {
        // dbg!();
        // print_state(self, ctx);

        let mut sequence_length = Length::None;
        let mut parent = Ctx::None;
        ctxs.iter().for_each(|ctx| {
            if sequence_length == Length::None {
                sequence_length = self.lengths(*ctx).iter().cloned().sum::<Length>();
                parent = self.parent(*ctx);
            }
            dbg!(&sequence_length);
            let mut len: usize = 0;
            let lengths = self.lengths(*ctx);
            let length_sum = lengths.iter().cloned().sum::<Length>();
            let len =
                f64::floor(sequence_length.as_f64() / length_sum.as_f64()) as usize * lengths.len();

            // dbg!(ctx, length_sum, len);
            self.cycle_fill(*ctx, len);
            // dbg!();
            print_state(self, *ctx);
        });
        let mut iters: Vec<(Ctx, Length, Enumerate<IntoIter<Length>>)> = ctxs
            .iter()
            .cloned()
            .map(|ctx| {
                (
                    ctx,
                    *self.playhead(),
                    self.lengths(ctx).into_iter().enumerate(),
                )
            })
            .collect();

        let init = self
            .lengths(parent)
            .iter()
            .cloned()
            .reduce(|a, b| gcd(a, b))
            .unwrap_or_default();

        let step = ctxs.iter().cloned().fold(init, |step, ctx| {
            gcd(
                step,
                self.lengths(ctx)
                    .iter()
                    .cloned()
                    .reduce(|a, b| gcd(a, b))
                    .unwrap_or_default(),
            )
        });

        eprintln!(
            "{}STEP: {step:?}{}",
            TextStyle::IntenseBoldYellow,
            TextStyle::ResetColor
        );

        'seq: loop {
            let mut ctx = Ctx::None;
            // dbg!(&iters);
            for (ctx_, counter, iter) in &mut iters {
                let playhead = self.playhead().clone();
                if *counter <= playhead {
                    let iter_len = iter.len();
                    if let Some((index, length)) = iter.next() {
                        if ctx == Ctx::None {
                            ctx = self.append_child(parent);
                            self.set_scope(ctx, Scope::Stack);
                        }
                        // eprintln!(
                        //     "{}index: {index}, length: {length:?} iter.len(): {}{}",
                        //     TextStyle::IntenseBoldRed,
                        //     iter_len,
                        //     TextStyle::ResetColor
                        // );

                        if let Some(register) = self.registers(*ctx_).get(index) {
                            if let Some(pc) = self.pcs(*ctx_).get(index) {
                                // print_state(self, *ctx_);
                                if let Some(velocity) = self.velocities(*ctx_).get(index) {
                                    if let Some(program) = self.programs(*ctx_).get(index) {
                                        if let Some(tempo) = self.tempos(*ctx_).get(index) {
                                            eprintln!(
                                                "{}{}s: PC: {} REG: {} VEL: {} LENGTH: {}{}",
                                                TextStyle::Cyan,
                                                counter.as_f64() / 1_000_000 as f64,
                                                pc.as_u8(),
                                                register.as_i8(),
                                                velocity.0,
                                                length.as_usize(),
                                                TextStyle::ResetColor
                                            );
                                            self.add_length(ctx, length);
                                            self.add_register(ctx, *register);
                                            self.add_pc(ctx, *pc);
                                            self.add_velocity(ctx, *velocity);
                                            self.add_program(ctx, *program);
                                            self.add_tempo(ctx, *tempo);
                                            // dbg!();
                                            // print_state(self, ctx);
                                            *counter += length;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // else {
                    //     break 'seq;
                    // }
                }
            }

            if ctx != Ctx::None {
                self.add_timeline_event(ctx);
            }
            if iters.iter().cloned().all(|(_, _, iter)| iter.len() == 0) {
                *self.playhead() += step;
                break 'seq;
            }
            *self.playhead() += step;
        }
        eprintln!();
    }
}
