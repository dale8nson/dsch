#![allow(unused)]
use std::{
    collections::{BTreeSet, HashSet},
    fmt::Debug,
    hash::{BuildHasher, DefaultHasher},
    iter::Sum,
    ops::{Add, AddAssign, Div, Mul, Rem},
    u64, usize,
};

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
            0
        }
    }

    pub fn as_u32(&self) -> u32 {
        if let Length::MicroSeconds(uint) = *self {
            uint as u32
        } else {
            0
        }
    }

    pub fn as_f64(&self) -> f64 {
        if let Length::MicroSeconds(n) = self {
            *n as f64
        } else {
            0.
        }
    }

    pub fn as_usize(&self) -> usize {
        if let Length::MicroSeconds(n) = self {
            *n as usize
        } else {
            0
        }
    }

    pub fn default_max() -> Self {
        Length::MicroSeconds(u64::MAX)
    }
}

impl Default for Length {
    fn default() -> Self {
        Length::MicroSeconds(u64::MIN)
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

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_u64() + rhs.as_u64())
    }
}

impl Sum for Length {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Length::MicroSeconds(0), |sum, l| sum + l)
    }
}

impl Div for Length {
    type Output = Length;
    fn div(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(f64::round(self.as_f64() / rhs.as_f64()) as u64)
    }
}

impl Rem for Length {
    type Output = Length;
    fn rem(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_u64() % rhs.as_u64())
    }
}

impl Mul for Length {
    type Output = Length;
    fn mul(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_u64() * rhs.as_u64())
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Velocity(pub u8);
// pub type F<'a, Args, Ret> = Box<dyn FnMut(Args) -> Ret + 'a>;
// pub const ID: fn() -> M<'static, Exp, Exp> = || Monad::ret(Box::new(move |exp| exp));
pub const ID: fn() -> F<'static, Exp, Exp> = || Box::new(move |exp| exp);

impl BuildHasher for Velocity {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Clone, Debug, Copy, Default, Hash, PartialEq, Eq)]
pub enum ScopeType {
    Sequence,
    Stack,
    Set,
    #[default]
    None,
}

impl BuildHasher for ScopeType {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, Hash, PartialOrd, Ord)]
pub enum Ctx {
    Id(usize),
    #[default]
    Root,
    None,
}

impl Ctx {
    pub fn to_usize(&self) -> usize {
        if let Ctx::Id(id) = self { *id } else { 0 }
    }

    pub fn to_u32(&self) -> u32 {
        if let Ctx::Id(id) = self {
            *id as u32
        } else {
            0
        }
    }
}

impl PartialEq for Ctx {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) {
            (Ctx::Id(n1), Ctx::Id(n2)) => n1 == n2,
            (Ctx::None, Ctx::None) => true,
            (Ctx::Root, Ctx::None | Ctx::Id(_)) => false,
            (Ctx::None | Ctx::Id(_), Ctx::Root) => false,
            (Ctx::Root, Ctx::Root) => true,
            (Ctx::Id(_), Ctx::Root | Ctx::None) => false,
            _ => false,
        }
    }
}

impl BuildHasher for Ctx {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Microseconds per beat
pub struct Mpb(pub u64);

impl Default for Mpb {
    fn default() -> Self {
        Mpb(f64::round(60_000_000 as f64 / 120 as f64) as u64)
    }
}

impl BuildHasher for Mpb {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Prog(pub u8);

impl Default for Prog {
    fn default() -> Self {
        Prog(0)
    }
}

impl BuildHasher for Prog {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    Reg(i8),
    #[default]
    None,
}

impl BuildHasher for Register {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tps(pub u64);

#[derive(Debug, Clone, Copy)]
pub enum Instruction<'a> {
    Midi(MidiMessage),
    Meta(MetaMessage<'a>),
}

pub mod utils {
    use std::{
        fmt::Display,
        io::stderr,
        iter::repeat_n,
        ops::{Div, Mul, Rem},
    };

    use crate::compiler::{
        ast::{utils::abs_to_f64, *},
        codegen::*,
    };

    use crossterm::{
        cursor::{self, MoveToNextLine, RestorePosition, position},
        execute,
        terminal::{self, ClearType, size},
    };

    #[derive(Clone, Copy)]
    pub enum Color {
        Black(Intensity),
        Red(Intensity),
        Green(Intensity),
        Yellow(Intensity),
        Blue(Intensity),
        Purple(Intensity),
        Cyan(Intensity),
        White(Intensity),
        Reset,
    }

    impl Display for Color {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Color::Black(intensity) => write!(f, "\x1b{intensity}0m"),
                Color::Red(intensity) => write!(f, "\x1b{intensity}1m"),
                Color::Green(intensity) => write!(f, "\x1b{intensity}2m"),
                Color::Yellow(intensity) => write!(f, "\x1b{intensity}3m"),
                Color::Blue(intensity) => write!(f, "\x1b{intensity}4m"),
                Color::Purple(intensity) => write!(f, "\x1b{intensity}5m"),
                Color::Cyan(intensity) => write!(f, "\x1b{intensity}6m"),
                Color::White(intensity) => write!(f, "\x1b{intensity}7m"),
                Color::Reset => write!(f, "\x1b[0m"),
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum Intensity {
        Normal,
        Intense,
        Bold,
        IntenseBold,
    }

    impl Display for Intensity {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Intensity::Normal => write!(f, "[0;3"),
                Intensity::Intense => write!(f, "[0;9"),
                Intensity::Bold => write!(f, "[1;3"),
                Intensity::IntenseBold => write!(f, "[1;9"),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum TextStyle {
        Black,
        IntenseBlack,
        BoldBlack,
        IntenseBoldBlack,
        Red,
        IntenseRed,
        BoldRed,
        IntenseBoldRed,
        Green,
        IntenseGreen,
        BoldGreen,
        IntenseBoldGreen,
        Yellow,
        IntenseYellow,
        BoldYellow,
        IntenseBoldYellow,
        Blue,
        IntenseBlue,
        BoldBlue,
        IntenseBoldBlue,
        Purple,
        IntensePurple,
        BoldPurple,
        IntenseBoldPurple,
        Cyan,
        IntenseCyan,
        BoldCyan,
        IntenseBoldCyan,
        White,
        IntenseWhite,
        BoldWhite,
        IntenseBoldWhite,
        ResetColor,
    }

    impl Display for TextStyle {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            use Intensity::*;
            use TextStyle::*;
            match self {
                Black => write!(f, "{}", Color::Black(Normal)),
                IntenseBlack => write!(f, "{}", Color::Black(Intense)),
                BoldBlack => write!(f, "{}", Color::Black(Bold)),
                IntenseBoldBlack => write!(f, "{}", Color::Black(IntenseBold)),
                Red => write!(f, "{}", Color::Red(Normal)),
                IntenseRed => write!(f, "{}", Color::Red(Intense)),
                BoldRed => write!(f, "{}", Color::Red(Bold)),
                IntenseBoldRed => write!(f, "{}", Color::Red(IntenseBold)),
                Green => write!(f, "{}", Color::Green(Normal)),
                IntenseGreen => write!(f, "{}", Color::Green(Intense)),
                BoldGreen => write!(f, "{}", Color::Green(Bold)),
                IntenseBoldGreen => write!(f, "{}", Color::Green(IntenseBold)),
                Yellow => write!(f, "{}", Color::Yellow(Normal)),
                IntenseYellow => write!(f, "{}", Color::Yellow(Intense)),
                BoldYellow => write!(f, "{}", Color::Yellow(Bold)),
                IntenseBoldYellow => write!(f, "{}", Color::Yellow(IntenseBold)),
                Blue => write!(f, "{}", Color::Blue(Normal)),
                IntenseBlue => write!(f, "{}", Color::Blue(Intense)),
                BoldBlue => write!(f, "{}", Color::Blue(Bold)),
                IntenseBoldBlue => write!(f, "{}", Color::Blue(IntenseBold)),
                Purple => write!(f, "{}", Color::Purple(Normal)),
                IntensePurple => write!(f, "{}", Color::Purple(Intense)),
                BoldPurple => write!(f, "{}", Color::Purple(Bold)),
                IntenseBoldPurple => write!(f, "{}", Color::Purple(IntenseBold)),
                Cyan => write!(f, "{}", Color::Cyan(Normal)),
                IntenseCyan => write!(f, "{}", Color::Cyan(Intense)),
                BoldCyan => write!(f, "{}", Color::Cyan(Bold)),
                IntenseBoldCyan => write!(f, "{}", Color::Cyan(IntenseBold)),
                White => write!(f, "{}", Color::White(Normal)),
                IntenseWhite => write!(f, "{}", Color::White(Intense)),
                BoldWhite => write!(f, "{}", Color::White(Bold)),
                IntenseBoldWhite => write!(f, "{}", Color::White(IntenseBold)),
                ResetColor => write!(f, "{}", Color::Reset),
            }
        }
    }

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

    pub fn gcd<
        T: Div<Output = T> + Rem<Output = T> + Ord + PartialEq + Default + Clone + Copy + Debug,
    >(
        a: T,
        b: T,
    ) -> T {
        // eprintln!(
        //     "{}{:?}, {:?}{}",
        //     TextStyle::IntensePurple,
        //     a,
        //     b,
        //     TextStyle::ResetColor
        // );
        let max = a.max(b);
        let min = a.min(b);

        if min == T::default() {
            max
        } else {
            gcd(min, max % min)
        }
    }

    pub fn lcd<
        T: Mul<Output = T>
            + Div<Output = T>
            + Rem<Output = T>
            + Ord
            + PartialEq
            + Default
            + Debug
            + Copy,
    >(
        a: T,
        b: T,
    ) -> T {
        let g = gcd(a, b);
        eprintln!(
            "{}gcd: {g:?}{}",
            TextStyle::IntensePurple,
            TextStyle::ResetColor
        );
        a * b / g
    }

    pub fn progress<T: Div + Display + Copy + Into<f64> + Into<u32>>(
        dividend: T,
        divisor: T,
        row: u16,
    ) {
        let quotient = Into::<f64>::into(dividend) / Into::<f64>::into(divisor);
        let rem: u32 = Into::<u32>::into(dividend) % Into::<u32>::into(divisor);
        let subtrahend = f64::floor(rem as f64 / Into::<f64>::into(divisor) * 8.) as u32;

        let rem_block = format!("{}", char::from_u32(0x258F - subtrahend).unwrap());
        let pc: f64 = quotient * 100.0;
        let text = format!("{:.2}%", pc);
        let (c, r) = size().unwrap();
        let width = c as usize - 7;
        // execute!(stderr(), cursor::Hide, cursor::MoveTo(0, r.min(row)));
        eprint!(
            "{}{}{}{}{text:>7}",
            TextStyle::Cyan,
            repeat_n(
                "\u{2588}",
                (f64::floor(pc as f64 / width as f64 * width as f64)) as usize,
            )
            .collect::<String>(),
            rem_block,
            TextStyle::ResetColor,
        );
    }

    pub fn out(col: u16, row: u16, s: String) {
        let (c, r) = size().unwrap();

        // execute!(
        //     stderr(),
        //     cursor::SavePosition,
        //     cursor::MoveTo(col.min(c), row.min(r))
        // );
        eprintln!("{}", s);
        // execute!(stderr(), RestorePosition);
    }
}
