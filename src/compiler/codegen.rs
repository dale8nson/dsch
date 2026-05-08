use crate::compiler::{ast::*, functional::*};
pub use midly::{MetaMessage, MidiMessage, num::*};

pub const PPQ: u15 = u15::new(25200);

#[derive(Debug, Clone, Copy)]
pub struct MicroSeconds(pub u64);

#[derive(Debug, Clone, Copy)]
pub enum Length {
    MicroSeconds(u64),
    None,
}

impl Length {
    pub fn as_u64(&self) -> u64 {
        if let Length::MicroSeconds(uint) = *self {
            uint
        } else {
            u15::max_value().as_int() as u64
        }
    }
}

impl Default for Length {
    fn default() -> Self {
        Length::MicroSeconds(u15::max_value().as_int() as u64)
    }
}

impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        self.as_u64() == other.as_u64()
    }
}

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.as_u64().cmp(&other.as_u64()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Velocity(pub u8);
// pub type F<'a, Args, Ret> = Box<dyn FnMut(Args) -> Ret + 'a>;
// pub const ID: fn() -> M<'static, Exp, Exp> = || Monad::ret(Box::new(move |exp| exp));
pub const ID: fn() -> F<'static, Exp, Exp> = || Box::new(move |exp| exp);

#[derive(Clone, Debug)]
pub enum ScopeType {
    Sequence,
    Stack,
    Set,
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Ctx {
    Id(usize),
    Root,
    #[default]
    None,
}

impl Ctx {
    pub fn to_usize(&self) -> usize {
        if let Ctx::Id(id) = self { *id } else { 0 }
    }
}

impl PartialEq for Ctx {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Ctx::Id(n1), Ctx::Id(n2)) => n1 == n2,
            (Ctx::None, Ctx::None) => true,
            _ => false,
        }
    }
}

impl Into<usize> for Ctx {
    fn into(self) -> usize {
        if let Ctx::Id(int) = self { int } else { 0 }
    }
}

impl From<usize> for Ctx {
    fn from(value: usize) -> Self {
        Ctx::Id(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pc(pub i8);

#[derive(Debug, Clone, Copy)]
/// Microseconds per beat
pub struct Mpb(pub u64);

#[derive(Debug, Clone)]
pub struct Prog(pub u8);

impl Default for Prog {
    fn default() -> Self {
        Prog(0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tps(pub u64);

#[derive(Debug, Clone)]
pub enum Instruction<'a> {
    MidiMessage(MidiMessage),
    MetaMessage(MetaMessage<'a>),
}

pub mod utils {
    use std::{
        cell::{RefCell, RefMut},
        ops::{Div, Rem},
        rc::Rc,
    };

    use crate::compiler::{
        ast::{utils::abs_to_f64, *},
        codegen::*,
    };

    pub fn to_length(frac: Absolute, tempo: Mpb) -> MicroSeconds {
        let fr = abs_to_f64(frac);
        MicroSeconds(f64::round(fr / 4 as f64 * tempo.0 as f64) as u64)
    }

    pub fn duration_to_micros(minutes: Absolute, seconds: Absolute) -> MicroSeconds {
        MicroSeconds(f64::round(
            match minutes {
                Absolute::UInt(int) => (int * 60 * 1_000_000) as f64,
                Absolute::Float(float) => float * 1_000_000 as f64,
            } + match seconds {
                Absolute::UInt(int) => (int * 1_000_000) as f64,
                Absolute::Float(float) => float * 1_000_000 as f64,
            },
        ) as u64)
    }

    pub fn gcf<T: Div + Rem<Output = T> + From<u64> + PartialOrd + Ord + PartialEq + Eq + Copy>(
        mut n1: T,
        mut n2: T,
    ) -> T {
        let mut rem = n1 % n2;

        while rem != Into::<T>::into(0) {
            n1 = n2;
            n2 = rem;
            rem = n1 % n2;
        }
        n2
    }
}
