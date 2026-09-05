#![allow(unused, const_item_mutation)]
use std::{
    collections::VecDeque, default, fmt::Display, hash::{BuildHasher, DefaultHasher}, ops::{Add, Div, Mul, Rem, Sub}, str::FromStr
};


use crate::compiler::{
    codegen::{self, *},
    functional::*,
};

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub exps: Vec<Exp>,
}

#[derive(Debug, Clone, Default)]
pub enum Exp {
    Compound(Compound),
    Simple(Simple),
    Decl(Decl),
    #[default]
    Noop,
    EOI,
}

impl Exp {
    pub fn to_string(&self) -> String {
        let s = format!("{self}").rsplit_once("::").unwrap().1.replace("}", "").to_string();
        s.clone()
            .chars()
            .map(|c| {
                if !s.contains('(') && matches!(c, ')') {
                    '\0'
                } else {
                    c
                }
            })
            .collect::<String>()
    }
}

impl Display for Exp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exp::Compound(compound) => write!(
                f,
                "{}",
                match compound {
                    Compound::Parens(_) => "Compound::Parens",
                    Compound::Braces(_) => "Compound::Braces",
                    Compound::Brackets(_) => "Compound::Brackets",
                    Compound::Ratio(_) => "Compound::Ratio",
                }
            ),
            Exp::Simple(simple) => write!(
                f,
                "{}",
                match simple {
                    Simple::Prefix(prefix) => {
                        "Simple::Prefix(".to_owned()
                            + match prefix {
                                Prefix::Dur => "Prefix::Dur)",
                                Prefix::Reg => "Prefix::Reg)",
                                Prefix::Prog => "Prefix::Prog)",
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
                                Infix::Plus => "Infix::Plus)",
                                Infix::Minus => "Infix::Minus)",
                                Infix::Mul => "Infix::Mul)",
                                Infix::Div => "Infix::Div)",
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
                        Scalar::Rest => format!("Scalar::Rest"),
                        Scalar::Prog(prog) => format!("Scalar::Prog({}))", prog.0),

                        Scalar::Tuplet(tuplet) => format!("Scalar::Tuplet({tuplet:?}))"),
                        Scalar::Register(register) => format!("Scalar::Register({register:?}))"),
                        _ => todo!(),
                    }
                    .to_owned(),
                    Simple::Decl(Decl::ExpDecl(ExpDecl { ident, binding })) => format!(
                        "Simple::Decl(Decl {{ ident: {} binding: {binding} }})",
                        ident.0
                    ),
                    Simple::Ident(ident) => format!("Simple::Ident({ident:?}))").parse().unwrap(),
                    _ => todo!(),
                }
            ),
            Exp::Noop => write!(f, "Exp::Noop"),
            Exp::EOI => write!(f, "Exp::EOI"),
            Exp::Decl(decl) => write!(f, "Exp::Decl"),
        }
    }
}

// impl Display for Vec<Exp> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             self.iter()
//                 .map(|exp| format!("{exp}"))
//                 .collect::<Vec<String>>()
//                 .join("\n")
//         )
//     }
// }

pub const NOOP: (Exp, codegen::Ctx) = (Exp::Noop, codegen::Ctx::None);

#[derive(Debug, Clone, Default, Copy)]
pub struct Bpm(pub Absolute);

#[derive(Debug, Clone)]
pub enum Decl {
    ImportDecl(ImportDecl),
    ExpDecl(ExpDecl),
    FuncDecl(FuncDecl)
}

#[derive(Debug, Clone)]
pub struct ImportDecl(Ident);

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub ident: Ident,
    pub params: Vec<Ident>,
    pub funcdef: Vec<Exp>
}

#[derive(Debug, Clone)]
pub struct ExpDecl {
    pub ident: Ident,
    pub binding: Box<Exp>,
}

#[derive(Debug, Clone)]
pub enum Simple {
    Prefix(Prefix),
    Scalar(Scalar),
    Infix(Infix),
    Suffix(Suffix),
    Decl(Decl),
    Ident(Ident),
}

#[derive(Debug, Clone)]
pub enum Compound {
    Parens(Vec<Exp>),
    Braces(Vec<Exp>),
    Brackets(Vec<Exp>),
    Ratio(Vec<Absolute>)
}

impl From<Compound> for VecDeque<Exp> {
    fn from(value: Compound) -> Self {
        value.to_vecdeque()
    }
}

impl Compound {
    pub fn to_vecdeque(&self) -> VecDeque<Exp> {
        match self {
            Compound::Parens(exps) | Compound::Braces(exps) | Compound::Brackets(exps) => {
                VecDeque::from_iter(exps.iter().cloned())
            }
            _ => VecDeque::new(),
        }
    }

    pub fn exps_mut(&mut self) -> &mut Vec<Exp> {
        match self {
            Compound::Parens(exps) | Compound::Braces(exps) | Compound::Brackets(exps) => exps,
            Compound::Ratio(absolutes) => todo!()
        }
    }

    pub fn to_exp(self) -> Exp {
        match self {
            Compound::Parens(exps) => Exp::Compound(Compound::Parens(exps)),
            Compound::Braces(exps) => Exp::Compound(Compound::Braces(exps)),
            Compound::Brackets(exps) => Exp::Compound(Compound::Brackets(exps)),
            Compound::Ratio(abss) => Exp::Compound(Compound::Ratio(abss)),
        }
    }

    pub fn scope(&self) -> Scope {
        match *self {
            Compound::Parens(_) => Scope::Sequence,
            Compound::Braces(_) => Scope::Stack,
            Compound::Brackets(_) => Scope::Set,
            _ => Scope::None,
        }
    }
}

impl IntoIterator for Compound {
    type Item = Exp;
    type IntoIter = std::collections::vec_deque::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.to_vecdeque().into_iter()
    }
}

#[derive(Debug, Clone)]
pub enum Scalar {
    Duration(Duration),
    Frequency(Absolute),
    Pure(Pure),
    Dynamic(Dynamic),
    Tuplet(Tuplet),
    Prog(Prog),
    Register(Register),
    Rest,
}

#[derive(Clone, Debug)]
pub struct Dynamic(pub String);



#[derive(Debug, Clone)]
pub struct Frequency(pub Pure);

#[derive(Debug, Clone, Copy)]
pub enum Infix {
    Colon,
    Intercalate,
    Range,
    Interpolation(Interpolation),
    Plus,
    Minus,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Exp,
    pub end: Exp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(pub String);

impl BuildHasher for Ident {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Interpolation {
    #[default]
    Increase,
    Decrease,
}

impl Display for Interpolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interpolation::Increase => f.write_str("<"),
            Interpolation::Decrease => f.write_str(">"),
        }
    }
}

impl From<Data> for Interpolation {
    fn from(value: Data) -> Self {
        if let Data::Interpolation(interpolation) = value {
            interpolation
        } else {
            Interpolation::Increase
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Prefix {
    Dur,
    Reg,
    Prog
}

impl Prefix {
    pub fn unwrap(exp: Exp) -> Option<Self> {
        match exp {
            Exp::Simple(Simple::Prefix(prefix)) => Some(prefix),
            _ => None
        }
    }
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

impl From<Duration> for Data {
    fn from(value: Duration) -> Self {
        match value {
            Duration::Fixed(fixed) => todo!(),
            Duration::Fractional(fractional) => match fractional {
                Fractional::Absolute(absolute) => Length::from(absolute).into(),
                Fractional::Tuplet(tuplet) => todo!(),
                Fractional::Rational(rational) => todo!(),
            },
        }
    }
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
    Rational(Rational),
}

#[derive(Debug, Clone)]
pub struct Rational {
    pub num: Absolute,
    pub den: Absolute,
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
    Rational(Rational)
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

impl Mul<u64> for Absolute {
    type Output = u64;
    fn mul(self, rhs: u64) -> Self::Output {
        match self {
            Absolute::UInt(uint) => uint * rhs,
            Absolute::Float(float) => f64::round(float * rhs as f64) as u64,
        }
    }
}

impl Mul<Length> for Absolute {
    type Output = Absolute;
    fn mul(self, rhs: Length) -> Self::Output {
        match self {
            Absolute::UInt(uint) => Absolute::UInt(uint * rhs.as_u64()),
            Absolute::Float(float) => Absolute::Float(float * rhs.as_f64()),
        }
    }
}

impl Div<Absolute> for f64 {
    type Output = f64;
    fn div(self, rhs: Absolute) -> Self::Output {
        self / rhs.as_f64()
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

    pub fn as_usize(&self) -> usize {
        match self {
            Self::UInt(int) => *int as usize,
            Self::Float(float) => f64::round(*float) as usize,
        }
    }
}

impl Div for Absolute {
    type Output = Absolute;
    fn div(self, rhs: Self) -> Self::Output {
        // Absolute::Float(f64::round(self.as_f64() / rhs.as_f64()))
        Absolute::UInt((self.as_u64() * 100) / (rhs.as_u64() * 100))
    }
}

#[derive(Debug, Clone, Copy)]
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
