use std::{
    default,
    ops::{Add, Div, Mul, Rem, Sub},
};

use crate::compiler::{codegen::MicroSeconds, functional::*};

#[derive(Debug, Clone)]
pub struct Program {
    pub exps: Vec<Exp>,
}

#[derive(Debug, Clone)]
pub enum Exp {
    Simple(Simple),
    Compound(Compound),
    None,
}

#[derive(Debug, Clone, Default)]
pub struct Bpm(pub Absolute);

#[derive(Debug, Clone)]
pub enum Compound {
    Parens(Vec<Exp>),
    Braces(Vec<Exp>),
    Brackets(Vec<Exp>),
    Ratio(Vec<Absolute>),
}

#[derive(Debug, Clone)]
pub enum Simple {
    Scalar(Scalar),
    Op(Op),
    Ident(Ident),
    Primitive(Primitive),
}

#[derive(Debug, Clone)]
pub enum Scalar {
    Duration(Duration),
    Frequency(Absolute),
    Pure(Pure),
}

#[derive(Debug, Clone)]
pub struct Frequency(pub Pure);

#[derive(Debug, Clone, Copy)]
pub enum Op {
    Colon,
    Intercalate,
    Range,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Exp,
    pub end: Exp,
}

#[derive(Debug, Clone)]
pub struct Ident(pub String);

#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Prefix(Prefix),
    Suffix(Suffix),
}

#[derive(Debug, Clone, Copy)]
pub enum Prefix {
    Dur,
    Rest,
    Pc,
    Reg,
}

#[derive(Debug, Clone, Copy)]
pub enum Suffix {
    Amp,
    Bpm,
    Freq,
}

#[derive(Debug, Clone)]
pub enum Duration {
    Fixed(Fixed),
    Fractional(Fractional),
}

#[derive(Debug, Clone)]
pub struct Fixed {
    pub minutes: Absolute,
    pub seconds: Absolute,
}

#[derive(Debug, Clone)]
pub enum Fractional {
    Absolute(Absolute),
    Tuplet(Tuplet),
}

#[derive(Debug, Clone)]
pub struct Tuplet {
    pub lhs: Absolute,
    pub rhs: Absolute,
}

#[derive(Debug, Clone)]
pub struct Minutes(pub Pure);

#[derive(Debug, Clone)]
pub struct Seconds(pub Pure);

#[derive(Debug, Clone)]
pub enum Pure {
    Relative(Relative),
    Absolute(Absolute),
}

#[derive(Debug, Clone)]
pub struct Relative {
    pub sign: Sign,
    pub val: Absolute,
}

#[derive(Debug, Clone, Copy)]
pub enum Absolute {
    UInt(u64),
    Float(f64),
}

impl Default for Absolute {
    fn default() -> Self {
        Absolute::Float(0.0)
    }
}

impl Absolute {
    pub fn as_u64(&self) -> u64 {
        match self {
            Self::UInt(int) => *int,
            Self::Float(float) => f64::round(*float) as u64,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            Self::UInt(int) => *int as f64,
            Self::Float(float) => *float,
        }
    }
}

impl Div for Absolute {
    type Output = Absolute;
    fn div(self, rhs: Self) -> Self::Output {
        Absolute::Float(f64::round(self.as_f64() / rhs.as_f64()))
    }
}

#[derive(Debug, Clone)]
pub enum Sign {
    Plus,
    Minus,
}

pub mod utils {
    use crate::compiler::ast::Absolute;

    pub fn abs_to_f64(abs: Absolute) -> f64 {
        match abs {
            Absolute::UInt(int) => int as f64,
            Absolute::Float(float) => float,
        }
    }
}
