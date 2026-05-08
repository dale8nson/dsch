use std::option::IntoIter;

use crate::compiler::{ast::*, codegen::*, functional::*};

#[derive(Debug, Default)]
pub struct Composer<'a> {
    contexts: Vec<Ctx>,
    current_context: Ctx,
    tps: Tps,
    parents: Vec<Ctx>,
    scope_types: Vec<ScopeType>,
    lengths: Vec<MicroSeconds>,
    pcs: Vec<Vec<Pc>>,
    tempos: Vec<Mpb>, // in micro seconds
    bpms: Vec<Bpm>,   // quarter note beats
    registers: Vec<Integer>,
    velocities: Vec<Velocity>,
    instruments: Vec<Instrument>,
    children: Vec<Vec<Ctx>>,
    instructions: Vec<Vec<Instruction<'a>>>,
    events: Vec<Vec<TrackEvent<'a>>>,
}

impl<'a> Composer<'a> {
    pub fn compose_program(&'a mut self, ast: Program) {
        let mut exps = ast.exps.into_iter();
        let ctx = self.append_child(Ctx::None);
        self.compose_exps(exps, ctx);
        todo!()
    }

    fn compose_exps(&mut self, exps: impl IntoIterator<Item = Exp>, ctx: Ctx) {
        exps.into_iter().for_each(|exp| self.compose_exp(exp, ctx));
    }

    fn compose_exp(&mut self, exp: Exp, ctx: Ctx) {
        match exp {
            Exp::Simple(simple) => self.compose_simple(simple, ctx),
            Exp::Compound(compound) => self.compose_compound(compound, ctx),
            Exp::None => todo!(),
        }
    }

    fn compose_simple(&mut self, simple: Simple, ctx: Ctx) {
        match simple {
            Simple::Scalar(scalar) => self.compose_scalar(scalar, ctx),
            Simple::Primitive(primitive) => self.compose_primitive(primitive, ctx),
            Simple::Op(op) => self.compose_op(op, ctx),
            Simple::Ident(ident) => self.compose_ident(ident, ctx),
        }
    }

    fn compose_compound(&mut self, compound: Compound, ctx: Ctx) {
        match compound {
            Compound::Parens(exps) => {
                self.compose_exps(exps, ctx);
            }
            Compound::Braces(exps) => {
                self.compose_exps(exps, ctx);
            }
            Compound::Brackets(exps) => {
                self.compose_exps(exps, ctx);
            }
            Compound::Ratio(absolutes) => {
                self.compose_ratio(absolutes, ctx);
            }
            Compound::Range(range) => self.compose_range(range, ctx),
        }
    }

    fn compose_scalar(&mut self, scalar: Scalar, ctx: Ctx) {
        match scalar {
            Scalar::Duration(duration) => self.compose_duration(duration, ctx),
            Scalar::Frequency(frequency) => self.compose_frequency(frequency, ctx),
            Scalar::Pure(pure) => self.compose_pure(pure, ctx),
        }
    }

    fn compose_primitive(&mut self, primitive: Primitive, ctx: Ctx) {
        match primitive {
            Primitive::Prefix(prefix) => self.compose_prefix(prefix, ctx),
            Primitive::Suffix(suffix) => self.compose_suffix(suffix, ctx),
        }
    }

    fn compose_op(&mut self, op: Op, ctx: Ctx) {
        match op {
            Op::Colon => todo!(),
            Op::Intercalate => todo!(),
        }
    }

    fn compose_ident(&mut self, ident: Ident, ctx: Ctx) {
        todo!()
    }

    fn compose_duration(&mut self, duration: Duration, ctx: Ctx) {
        todo!()
    }

    fn compose_frequency(&mut self, frequency: Absolute, ctx: Ctx) {
        todo!()
    }

    fn compose_pure(&mut self, pure: Pure, ctx: Ctx) {
        match pure {
            Pure::Absolute(abs) => self.compose_absolute(abs, ctx),
            Pure::Relative(relative) => self.compose_relative(relative, ctx),
        }
    }

    fn compose_prefix(&mut self, prefix: Prefix, ctx: Ctx) {
        match prefix {
            Prefix::Pc => todo!(),
            Prefix::Dur => todo!(),
            Prefix::Rest => todo!(),
            Prefix::Reg => todo!(),
        }
    }

    fn compose_suffix(&mut self, suffix: Suffix, ctx: Ctx) {
        match suffix {
            Suffix::Bpm => todo!(),
            Suffix::Amp => todo!(),
            Suffix::Freq => todo!(),
        }
    }

    fn compose_absolute(&mut self, abs: Absolute, ctx: Ctx) {
        todo!()
    }

    fn compose_relative(&mut self, relative: Relative, ctx: Ctx) {
        todo!()
    }

    fn compose_ratio(&mut self, absolutes: Vec<Absolute>, ctx: Ctx) {
        todo!()
    }

    fn compose_range(&mut self, range: Range, ctx: Ctx) {
        todo!()
    }

    fn append_child(&mut self, parent: Ctx) -> Ctx {
        let id = self.contexts_mut().len();
        let ctx = Ctx::Id(id);
        let length = if let Ctx::Id(parent_id) = parent {
            self.lengths_mut()[parent_id]
        } else {
            MicroSeconds(0)
        };
        self.contexts_mut().push(ctx);
        self.parents_mut().push(parent);
        self.children_mut().push(Vec::<Ctx>::new());
        if parent != Ctx::None {
            self.children_mut()[parent.to_usize()].push(ctx);
            let parent_tempo = self.get_tempo(parent);
            self.tempos_mut().push(parent_tempo);
        }

        self.scope_types_mut().push(ScopeType::None);
        self.lengths_mut().push(length);

        self.pcs_mut().push(vec![Pc(-1)]);
        self.bpms_mut().push(Bpm(Absolute::Integer(0)));
        self.registers_mut().push(Integer(4));
        self.velocities_mut().push(Velocity(0));
        self.instruments_mut()
            .push(Instrument("Piano".as_bytes().iter().cloned().collect()));
        self.instructions_mut().push(Vec::<Instruction>::new());
        self.events_mut().push(Vec::<TrackEvent>::new());
        self.current_context = ctx;
        ctx
    }

    fn current_context(&self) -> Ctx {
        self.current_context
    }

    fn tps_mut(&mut self) -> &mut Tps {
        &mut self.tps
    }

    fn lengths_mut(&mut self) -> &mut Vec<MicroSeconds> {
        &mut self.lengths
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
        self.tempos()[ctx.to_usize()]
    }

    fn tempos_mut(&mut self) -> &mut Vec<Mpb> {
        &mut self.tempos
    }

    fn bpms_mut(&mut self) -> &mut Vec<Bpm> {
        &mut self.bpms
    }

    fn registers_mut(&mut self) -> &mut Vec<Integer> {
        &mut self.registers
    }

    fn velocities_mut(&mut self) -> &mut Vec<Velocity> {
        &mut self.velocities
    }

    fn instruments_mut(&mut self) -> &mut Vec<Instrument> {
        &mut self.instruments
    }

    fn children_mut(&mut self) -> &mut Vec<Vec<Ctx>> {
        &mut self.children
    }

    fn instructions_mut(&mut self) -> &mut Vec<Vec<Instruction<'a>>> {
        &mut self.instructions
    }

    fn events_mut(&mut self) -> &mut Vec<Vec<TrackEvent<'a>>> {
        &mut self.events
    }

    fn tempos(&self) -> &[Mpb] {
        &self.tempos
    }

    fn contexts_mut(&mut self) -> &mut Vec<Ctx> {
        &mut self.contexts
    }
}
