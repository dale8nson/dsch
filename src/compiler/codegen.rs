#![allow(unused)]
use std::u64;

use crate::compiler::{ast::*, functional::*};
pub use midly::{MetaMessage, MidiMessage, num::*};

pub const PPQ: u15 = u15::new(25200);
// pub const PPQ: u15 = u15::new(480);

#[derive(Debug, Clone)]
pub struct Context {
    pub ctx: Ctx,
    pub parent: Ctx,
    pub children: Vec<Ctx>,
    pub scope: ScopeType,
    pub register: Register,
    pub pcs: Vec<Pc>,
    pub velocities: Vec<Velocity>,
    pub bpm: Bpm,
    pub tempo: Mpb,
    pub lengths: Vec<Length>,
    pub program: Prog,
}

#[derive(Debug, Clone, Copy)]
pub struct MicroSeconds(pub u64);

#[derive(Debug, Clone, Copy, Ord, Eq)]
pub enum Length {
    MicroSeconds(u64),
    None,
}

impl Length {
    pub fn as_u64(&self) -> u64 {
        if let Length::MicroSeconds(uint) = *self {
            uint
        } else {
            u64::MAX
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Velocity(pub u8);
// pub type F<'a, Args, Ret> = Box<dyn FnMut(Args) -> Ret + 'a>;
// pub const ID: fn() -> M<'static, Exp, Exp> = || Monad::ret(Box::new(move |exp| exp));
pub const ID: fn() -> F<'static, Exp, Exp> = || Box::new(move |exp| exp);

#[derive(Clone, Debug, Copy, Default)]
pub enum ScopeType {
    Sequence,
    Stack,
    Set,
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, Default)]
pub enum Pc {
    Class(i8),
    #[default]
    None,
}

impl Pc {
    pub fn to_i8(self) -> i8 {
        match self {
            Pc::Class(int) => int,
            Pc::None => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Microseconds per beat
pub struct Mpb(pub u64);

impl Default for Mpb {
    fn default() -> Self {
        Mpb(f64::round(60_000_000 as f64 / 120 as f64) as u64)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Prog(pub u8);

impl Default for Prog {
    fn default() -> Self {
        Prog(0)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum Register {
    Reg(i8),
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tps(pub u64);

#[derive(Debug, Clone, Copy)]
pub enum Instruction<'a> {
    Midi(MidiMessage),
    Meta(MetaMessage<'a>),
}

pub mod utils {
    use std::ops::{Div, Rem};

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

    pub fn length_to_ticks(length: Length, tempo: Mpb) -> u64 {
        match length {
            Length::None => 0,
            Length::MicroSeconds(micros) => {
                f64::round(micros as f64 / tempo.0 as f64 * PPQ.as_int() as f64) as u64
            }
        }
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

    pub fn align(expr: impl std::fmt::Debug, indents: usize, width: usize) -> String {
        let expr = format!("{:?}", expr).replace('\n', "");
        let step: usize = isize::min(
            (i32::min(
                width as i32 - (indents) as i32,
                i32::max(expr.len() as i32 - (indents) as i32, expr.len() as i32),
            )) as isize,
            expr.len() as isize,
        ) as usize;

        // eprintln!("step: {step}\nindents: {indents}");

        let mut prev: usize = 0;
        let mut string = format!("{}", " ".repeat(indents));
        string.push_str(
            format!(
                "{}",
                expr.chars()
                    .enumerate()
                    .step_by(step)
                    .scan(format!("{:?}", expr), |string, (idx, _)| {
                        let slice = &string[prev..idx];
                        prev = idx;
                        Some(format!("{}{}", " ".repeat(indents), slice.to_string()))
                    })
                    .collect::<Vec<String>>()
                    .join(format!("\n{0:<1$}", ' ', indents).as_str())
            )
            .as_str(),
        );
        let slice = &expr[prev..expr.len()];
        string.push_str(format!("\n{0:<1$}{2}", ' ', indents * 2, slice).as_str());
        // println!("{}", string);
        string.replace('"', "")
    }

    // #[macro_export]
    // macro_rules! align {
    //     ($expr: expr, $indents: ident, $width: literal) => {
    //         format!("{:?}", $expr)
    //             .replace("\n", "")
    //             .chars()
    //             .enumerate()
    //             .step_by($width - $indents * 4)
    //             .scan((format!("{:?}", $expr), 0), |(string, prev), (idx, _)| {
    //                 let slice = &string.to_string()[*prev..idx];
    //                 *prev = idx;
    //                 Some(slice)
    //             })
    //             .collect()
    //             .join(format!("\n{:<0$}", $indents * 4))
    //     };
    // }
}
