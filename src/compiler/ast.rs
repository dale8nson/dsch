#![allow(unused, const_item_mutation)]
use std::{
    collections::VecDeque,
    default,
    fmt::Display,
    hash::{BuildHasher, DefaultHasher},
    iter::Sum,
    ops::{Add, Div, Mul, Rem, Sub},
    str::FromStr,
};

use num_bigint::BigInt;
use num_rational::Ratio;
use num_traits::{FromPrimitive, ToPrimitive};
// use derive_more::{Add, Mul, Sum};
use pest::error::Error;

use crate::compiler::{
    codegen::{
        self,
        state::State,
        utils::{Coord, QUARTER_NOTE_RATIO},
        *,
    },
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
        let s = format!("{self}")
            .rsplit_once("::")
            .unwrap()
            .1
            .replace("}", "")
            .to_string();
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
                                Infix::Lerp => "Infix::Lerp",
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
                    Simple::Controller(ControllerType::Pedal) =>
                        format!("Simple::Controller::Pedal)").parse().unwrap(),
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
    FuncDecl(FuncDecl),
}

#[derive(Debug, Clone)]
pub enum ControllerType {
    Pedal,
}

#[derive(Debug, Clone)]
pub struct ImportDecl(Ident);

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub ident: Ident,
    pub params: Vec<Ident>,
    pub funcdef: Vec<Exp>,
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
    Controller(ControllerType),
    Ident(Ident),
}

#[derive(Debug, Clone)]
pub enum Compound {
    Parens(Vec<Exp>),
    Braces(Vec<Exp>),
    Brackets(Vec<Exp>),
    Ratio(Vec<Absolute>),
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
            Compound::Ratio(absolutes) => todo!(),
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
    Tempo(Absolute),
    Tuplet(Tuplet),
    Prog(Prog),
    Register(Register),
    Rest,
}

// impl Coord for Scalar {
//     fn id(&self) -> Self {
//         match self {
//             Scalar::Duration(_) => Scalar::Duration(Duration::Fractional(Fractional::Absolute(
//                 Absolute::Float(1.0),
//             ))),
//             Scalar::Frequency(_) => todo!(),
//             Scalar::Pure(_) => Scalar::Pure(Pure::Absolute(Absolute::Float(1.0))),
//             Scalar::Dynamic(_) => Scalar::Dynamic(Dynamic::Id),
//             Scalar::Tuplet(_) => todo!(),
//             Scalar::Prog(_) => todo!(),
//             Scalar::Register(_) => todo!(),
//             Scalar::Rest => todo!(),
//         }
//     }
// }

impl Add for Scalar {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Scalar::Duration(lhs), Scalar::Duration(rhs)) => {
                Scalar::Duration(Duration::Fractional(Fractional::Absolute(Absolute::Float(
                    lhs.as_f64() + rhs.as_f64(),
                ))))
            }
            (Scalar::Frequency(lhs), Scalar::Frequency(rhs)) => todo!(),
            (Scalar::Pure(lhs), Scalar::Pure(rhs)) => Scalar::Pure(lhs + rhs),
            (Scalar::Dynamic(lhs), Scalar::Dynamic(rhs)) => {
                Scalar::Pure(Pure::Absolute(Absolute::Float(lhs.as_f64() + rhs.as_f64())))
            }
            (Scalar::Tuplet(lhs), Scalar::Tuplet(rhs)) => todo!(),
            (Scalar::Prog(lhs), Scalar::Prog(rhs)) => todo!(),
            (Scalar::Register(lhs), Scalar::Register(rhs)) => todo!(),
            _ => todo!(),
        }
    }
}

impl Sum for Scalar {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, s| acc + s).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, Hash)]
pub enum Dynamic {
    Id,
    PPP,
    PP,
    P,
    MP,
    Default,
    MF,
    F,
    FF,
    FFF,
}

impl Mul for Dynamic {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::from(self.as_f64() * rhs.as_f64())
    }
}

impl Coord for Dynamic {
    type MultiplicativeIdentity = f64;
    fn id(&self) -> Self::MultiplicativeIdentity {
        Dynamic::Id.as_f64()
    }
}

impl PartialOrd for Dynamic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_f64().partial_cmp(&other.as_f64())
    }
}

impl FromStr for Dynamic {
    type Err = Error<Dynamic>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ppp" => Self::PPP,
            "pp" => Self::PP,
            "p" => Self::P,
            "mp" => Self::MP,
            "mf" => Self::MF,
            "f" => Self::F,
            "ff" => Self::FF,
            "fff" => Self::FFF,
            "id" => Self::Id,
            _ => Self::Default,
        })
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        self.as_f64() == other.as_f64()
    }
}

impl Add for Dynamic {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.as_f64() + rhs.as_f64())
    }
}

impl Sub for Dynamic {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from(self.as_f64() - rhs.as_f64())
    }
}

impl Sum for Dynamic {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, d| Dynamic::from(acc.as_f64() + d.as_f64()))
            .unwrap()
    }
}

impl Div for Dynamic {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Dynamic::from(self.as_f64() / rhs.as_f64())
    }
}

impl From<f64> for Dynamic {
    fn from(value: f64) -> Self {
        match value {
            20.0..39.0 => Dynamic::PPP,
            39.0..57.0 => Dynamic::PP,
            57.0..71.0 => Dynamic::P,
            71.0..84.0 => Dynamic::MP,
            84.0..98.0 => Dynamic::MF,
            98.0..113.0 => Dynamic::F,
            113.0..127.0 => Dynamic::FF,
            127.0 => Dynamic::FFF,
            _ => Dynamic::Default,
        }
    }
}

impl Dynamic {
    pub fn as_f64(&self) -> f64 {
        match self {
            Dynamic::Id => 1.,
            Dynamic::PPP => 20.,
            Dynamic::PP => 39.,
            Dynamic::P => 57.,
            Dynamic::MP => 71.,
            Dynamic::Default => 80.,
            Dynamic::MF => 84.,
            Dynamic::F => 98.,
            Dynamic::FF => 113.,
            Dynamic::FFF => 127.,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frequency(pub Pure);

#[derive(Debug, Clone, Copy)]
pub enum Infix {
    Colon,
    Intercalate,
    Range,
    Lerp,
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
    Prog,
}

impl Prefix {
    pub fn unwrap(exp: Exp) -> Option<Self> {
        match exp {
            Exp::Simple(Simple::Prefix(prefix)) => Some(prefix),
            _ => None,
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

impl Mul for Duration {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Duration::Fixed(lhs), Duration::Fixed(rhs)) => Duration::Fixed(lhs * rhs),
            (Duration::Fixed(fixed), Duration::Fractional(fractional)) => todo!(),
            (Duration::Fractional(fractional), Duration::Fixed(fixed)) => todo!(),
            (Duration::Fractional(lhs), Duration::Fractional(rhs)) => {
                Duration::Fractional(lhs * rhs)
            }
        }
    }
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

impl Duration {
    pub fn as_f64(&self) -> f64 {
        match self {
            Duration::Fixed(fixed) => todo!(),
            Duration::Fractional(fractional) => match fractional {
                Fractional::Absolute(absolute) => absolute.as_f64(),
                Fractional::Tuplet(tuplet) => todo!(),
                Fractional::Rational(rational) => rational.num.as_f64() / rational.den.as_f64(),
            },
        }
    }

    pub fn to_micros(&self, state: &State) -> Ratio<BigInt> {
        // dbg!();
        if let Duration::Fractional(Fractional::Absolute(absolute)) = self {
            let tempo = state.tempo().0.clone();
            let float = absolute.as_f64() / QUARTER_NOTE_RATIO;
            let ratio = Ratio::<BigInt>::from_float(float).unwrap();
            // dbg!(tempo, ratio.clone(), absolute.as_f64(), absolute.as_f64() / QUARTER_NOTE_RATIO);
            Ratio::<BigInt>::from(state.tempo().0.clone() / ratio.max(Ratio::<BigInt>::from_float(f64::EPSILON).unwrap_or_default()))
        } else {
            state.tempo().0.clone() / Ratio::<BigInt>::from_float(1.0 / QUARTER_NOTE_RATIO).unwrap()
        }
    }
}

#[derive(Debug, Clone, derive_more::Add, derive_more::Div)]
#[div(forward)]
pub struct Fixed {
    pub minutes: Absolute,
    pub seconds: Absolute,
}

impl Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let seconds = self.seconds.as_u64() * rhs.seconds.as_u64() % 60;
        let minutes = self.minutes.as_u64() * rhs.minutes.as_u64()
            + self.seconds.as_u64() * rhs.seconds.as_u64() / 60;
        Self {
            minutes: Absolute::UInt(minutes),
            seconds: Absolute::UInt(seconds),
        }
    }
}

#[derive(Debug, Clone, derive_more::Add)]
pub enum Fractional {
    Absolute(Absolute),
    Tuplet(Tuplet),
    Rational(Rational),
}

impl Mul for Fractional {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Fractional::Absolute(lhs), Fractional::Absolute(rhs)) => {
                Fractional::Absolute(lhs * rhs)
            }
            (Fractional::Absolute(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Absolute(lhs), Fractional::Rational(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Absolute(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Rational(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Absolute(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Rational(rhs)) => todo!(),
        }
    }
}

impl Div for Fractional {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Fractional::Absolute(lhs), Fractional::Absolute(rhs)) => {
                Fractional::Absolute(lhs / rhs)
            }
            (Fractional::Absolute(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Absolute(lhs), Fractional::Rational(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Absolute(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Tuplet(lhs), Fractional::Rational(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Absolute(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Tuplet(rhs)) => todo!(),
            (Fractional::Rational(lhs), Fractional::Rational(rhs)) => todo!(),
        }
    }
}

#[derive(Debug, Clone, derive_more::Add, derive_more::Mul, derive_more::Div)]
#[mul(forward)]
#[div(forward)]
pub struct Rational {
    pub num: Absolute,
    pub den: Absolute,
}

// impl Mul for Rational {
//     type Output = Self;
//     fn mul(self, rhs: Self) -> Self::Output {
//         Self { num: self.num * rhs.num, den: self.den * rhs.den }
//     }
// }

#[derive(Debug, Clone, derive_more::Div)]
#[div(forward)]
pub struct Tuplet {
    pub lhs: Absolute,
    pub rhs: Absolute,
}

impl Mul for Tuplet {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            lhs: self.lhs * rhs.lhs,
            rhs: self.rhs * rhs.rhs,
        }
    }
}

impl Add for Tuplet {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            lhs: self.lhs * rhs.rhs + rhs.lhs * self.rhs,
            rhs: self.rhs * rhs.rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Minutes(pub Pure);

#[derive(Debug, Clone)]
pub struct Seconds(pub Pure);

#[derive(Debug, Clone, derive_more::Div)]
#[div(forward)]
pub enum Pure {
    Relative(Relative),
    Absolute(Absolute),
    Rational(Rational),
}

impl Add for Pure {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Pure::Relative(lhs), Pure::Relative(rhs)) => todo!(),
            (Pure::Relative(relative), Pure::Absolute(absolute)) => todo!(),
            (Pure::Relative(relative), Pure::Rational(rational)) => todo!(),
            (Pure::Absolute(absolute), Pure::Relative(relative)) => todo!(),
            (Pure::Absolute(lhs), Pure::Absolute(rhs)) => {
                Pure::Absolute(Absolute::Float(lhs.as_f64() + rhs.as_f64()))
            }
            (Pure::Absolute(absolute), Pure::Rational(rational)) => todo!(),
            (Pure::Rational(rational), Pure::Relative(relative)) => todo!(),
            (Pure::Rational(rational), Pure::Absolute(absolute)) => todo!(),
            (Pure::Rational(lhs), Pure::Rational(rhs)) => todo!(),
        }
    }
}

impl Mul for Pure {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Pure::Relative(lhs), Pure::Relative(rhs)) => Pure::Relative(lhs * rhs),
            (Pure::Relative(relative), Pure::Absolute(absolute)) => todo!(),
            (Pure::Relative(relative), Pure::Rational(rational)) => todo!(),
            (Pure::Absolute(absolute), Pure::Relative(relative)) => todo!(),
            (Pure::Absolute(lhs), Pure::Absolute(rhs)) => Pure::Absolute(lhs * rhs),
            (Pure::Absolute(absolute), Pure::Rational(rational)) => todo!(),
            (Pure::Rational(rational), Pure::Relative(relative)) => todo!(),
            (Pure::Rational(rational), Pure::Absolute(absolute)) => todo!(),
            (Pure::Rational(lhs), Pure::Rational(rhs)) => Pure::Rational(lhs * rhs),
        }
    }
}

#[derive(Debug, Clone, derive_more::Mul, derive_more::Div)]
#[mul(forward)]
#[div(forward)]
pub struct Relative {
    pub sign: Sign,
    pub val: Absolute,
}

impl Add for Relative {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            sign: self.sign + rhs.sign,
            val: self.val + rhs.val,
        }
    }
}

#[derive(Debug, Clone, Copy, derive_more::From)]
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

impl Mul for Absolute {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Absolute::UInt(_) => Absolute::UInt(self.as_u64() * rhs.as_u64()),
            Absolute::Float(_) => Absolute::Float(self.as_f64() * self.as_f64()),
        }
    }
}

impl Div for Absolute {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Absolute::Float(self.as_f64() / rhs.as_f64())
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

impl Add for Absolute {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Float(self.as_f64() + rhs.as_f64())
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

#[derive(Debug, Clone, Copy)]
pub enum Sign {
    Plus,
    Minus,
}

impl Add for Sign {
    type Output = Sign;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Sign::Plus, Sign::Plus) => Sign::Plus,
            (Sign::Plus, Sign::Minus) => Sign::Minus,
            (Sign::Minus, Sign::Plus) => Sign::Minus,
            (Sign::Minus, Sign::Minus) => Sign::Plus,
        }
    }
}

impl Mul for Sign {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Sign::Plus, Sign::Plus) => Self::Plus,
            (Sign::Plus, Sign::Minus) => Self::Minus,
            (Sign::Minus, Sign::Plus) => Self::Minus,
            (Sign::Minus, Sign::Minus) => Self::Plus,
        }
    }
}

impl Div for Sign {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Sign::Plus, Sign::Plus) => Self::Plus,
            (Sign::Plus, Sign::Minus) => Self::Minus,
            (Sign::Minus, Sign::Plus) => Self::Minus,
            (Sign::Minus, Sign::Minus) => Self::Plus,
        }
    }
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
