#![allow(unused, const_item_mutation)]
use std::{
    default,
    fmt::Display,
    ops::{Add, Div, Mul, Rem, Sub},
    str::FromStr,
};

use crate::compiler::functional::*;

#[derive(Debug, Clone)]
pub struct Program {
    pub exps: Vec<Exp>,
}

#[derive(Debug, Clone, Default)]
pub enum Exp {
    Compound(Box<Compound>),
    Simple(Simple),
    #[default]
    Noop,
    EOS,
}

impl Display for Exp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exp::Compound(compound) => write!(
                f,
                "{}",
                match **compound {
                    Compound::Parens(_) => "Compound::Parens",
                    Compound::Braces(_) => "Compound::Braces",
                    Compound::Brackets(_) => "Compound::Brackets",
                    Compound::Ratio(_) => "Compound::Ratio",
                    Compound::Decl(_) => "Compound::Decl",
                }
            ),
            Exp::Simple(simple) => write!(
                f,
                "{}",
                match simple {
                    Simple::Prefix(prefix) => {
                        "Simple::Prefix(".to_owned()
                            + match prefix {
                                Prefix::Pc => "Prefix::Pc)",
                                Prefix::Dur => "Prefix::Dur)",
                                Prefix::Reg => "Prefix::Reg)",
                                Prefix::Rest => "Prefix::Rest)",
                            }
                    }
                    Simple::Suffix(suffix) => {
                        "Simple::Suffix(".to_owned()
                            + match suffix {
                                Suffix::Bpm => "Suffix::Bpm)",
                                Suffix::Amp => "Suffix::Amp)",
                                Suffix::Freq => "Suffix::Freq)",
                            }
                    }
                    Simple::Infix(infix) =>
                        "Simple::Infix(".to_owned()
                            + match infix {
                                Infix::Colon => "Infix::Colon",
                                Infix::Intercalate => "Infix::Intercalate",
                                Infix::Range => "Infix::Range(",
                                Infix::Interpolation(interpolation) => match interpolation {
                                    Interpolation::Increase => "Interpolation::Increase)",
                                    Interpolation::Decrease => "Interpolation::Decrease)",
                                },
                            },
                    Simple::Scalar(scalar) => match scalar.clone() {
                        Scalar::Duration(duration) => match duration {
                            Duration::Fixed(Fixed { minutes, seconds }) => format!(
                                "Simple::Scalar(Scalar::Duration(Duration::Fixed({:?}'{:?}))\"",
                                minutes.as_u64().clone(),
                                seconds.as_u64().clone()
                            ),
                            Duration::Fractional(fractional) =>
                                format!("Duration::Fractional({fractional:?}))")
                                    .parse()
                                    .unwrap(),
                        }
                        .to_owned(),
                        Scalar::Dynamic(dynamic) =>
                            format!("Scalar::Dynamic({dynamic:?}))").parse().unwrap(),
                        Scalar::Pure(pure) => format!("Scalar::Pure({pure:?}))").parse().unwrap(),
                        Scalar::Frequency(frequency) =>
                            format!("Scalar::Frequency({frequency:?}))")
                                .parse()
                                .unwrap(),
                        Scalar::Tempo(abs) => format!("Scalar::Tempo({abs:?}))").parse().unwrap(),
                        _ => todo!(),
                    }
                    .to_owned(),
                    Simple::Ident(ident) => format!("Simple::Ident({ident:?}))").parse().unwrap(),
                    _ => todo!(),
                }
            ),
            Exp::Noop => write!(f, "Exp::Noop"),
            Exp::EOS => write!(f, "Exp::EOS"),
        }
    }
}

pub const NOOP: Exp = Exp::Noop;

#[derive(Debug, Clone, Default, Copy)]
pub struct Bpm(pub Absolute);

#[derive(Debug, Clone)]
pub struct Decl {
    pub ident: Ident,
    pub binding: Box<Exp>,
}

#[derive(Debug, Clone)]
pub enum Simple {
    Prefix(Prefix),
    Scalar(Scalar),
    Infix(Infix),
    Suffix(Suffix),
    Ident(Ident),
}

#[derive(Debug, Clone)]
pub enum Compound {
    Parens(Vec<Exp>),
    Braces(Vec<Exp>),
    Brackets(Vec<Exp>),
    Ratio(Vec<Absolute>),
    Decl(Box<Decl>),
}

#[derive(Debug, Clone)]
pub enum Scalar {
    Duration(Duration),
    Frequency(Absolute),
    Pure(Pure),
    Dynamic(String),
    Tempo(Absolute),
}

#[derive(Debug, Clone)]
pub struct Frequency(pub Pure);

#[derive(Debug, Clone, Copy)]
pub enum Infix {
    Colon,
    Intercalate,
    Range,
    Interpolation(Interpolation),
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Exp,
    pub end: Exp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(pub String);

#[derive(Debug, Clone, Copy)]
pub enum Interpolation {
    Increase,
    Decrease,
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
    use crate::compiler::{
        ast::Absolute,
        codegen::{Mpb, PPQ},
    };

    pub fn abs_to_f64(abs: Absolute) -> f64 {
        match abs {
            Absolute::UInt(int) => int as f64,
            Absolute::Float(float) => float,
        }
    }
}
