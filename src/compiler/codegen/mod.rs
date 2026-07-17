#![allow(unused)]
pub mod state;
pub mod utils;

use std::{
    any::{Any, TypeId},
    collections::{BTreeSet, HashSet},
    fmt::Debug,
    hash::{BuildHasher, DefaultHasher},
    iter::Sum,
    ops::{Add, AddAssign, BitAnd, BitOr, Div, Mul, Rem, Sub, SubAssign},
    u64, usize,
};

use crate::compiler::{ast::*, functional::*};
pub use midly::{MetaMessage, MidiMessage, num::*};

pub const PPQ: u15 = u15::new(25200);

pub const PC_ID: TypeId = TypeId::of::<Pc>();
pub const LENGTH_ID: TypeId = TypeId::of::<Length>();
pub const VELOCITY_ID: TypeId = TypeId::of::<Velocity>();
pub const PROGRAM_ID: TypeId = TypeId::of::<Prog>();
pub const REGISTER_ID: TypeId = TypeId::of::<Register>();
pub const TEMPO_ID: TypeId = TypeId::of::<Mpb>();

#[derive(Debug, Clone, Copy)]
pub struct MicroSeconds(pub u64);

#[derive(Debug, Clone, Copy, Ord, Eq)]
pub enum Length {
    MicroSeconds(u64),
    None,
}

impl From<Data> for Length {
    fn from(value: Data) -> Self {
        if let Data::Length(length) = value {
            length
        } else {
            Length::default()
        }
    }
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
        Length::MicroSeconds(0)
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
        // dbg!(&self, &rhs);
        Length::MicroSeconds(self.as_u64() % rhs.as_u64())
    }
}

impl Mul for Length {
    type Output = Length;
    fn mul(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_u64() * rhs.as_u64())
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_u64() - rhs.as_u64())
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs
    }
}

impl SubAssign for Length {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Velocity(pub u8);

impl Default for Velocity {
    fn default() -> Self {
        Velocity(80)
    }
}

impl From<Data> for Velocity {
    fn from(value: Data) -> Self {
        if let Data::Velocity(velocity) = value {
            velocity
        } else {
            Velocity::default()
        }
    }
}

// pub type F<'a, Args, Ret> = Box<dyn FnMut(Args) -> Ret + 'a>;
// pub const ID: fn() -> M<'static, Exp, Exp> = || Monad::ret(Box::new(move |exp| exp));
// pub const ID: fn() -> F<'static, Exp, Exp> = || Box::new(move |exp| exp);

impl BuildHasher for Velocity {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Clone, Debug, Copy, Default, Hash, PartialEq, Eq)]
pub enum Scope {
    Sequence,
    Stack,
    Set,
    #[default]
    None,
}

impl BuildHasher for Scope {
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
            (Ctx::Root, Ctx::Id(0)) | (Ctx::Id(0), Ctx::Root) => true,
            (Ctx::Id(n1), Ctx::Id(n2)) => n1 == n2,
            (Ctx::None, Ctx::None) => true,
            // (Ctx::Root, Ctx::None) => false,
            // (Ctx::None, Ctx::Root) => false,
            (Ctx::Root, Ctx::Root) => true,
            // (Ctx::Root, Ctx::Id(_)) => false,
            // (Ctx::Id(_), Ctx::Root) => false,
            // (Ctx::Id(_), Ctx::None) => false,
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
    Class(u8),
    #[default]
    None,
}

impl From<Data> for Pc {
    fn from(value: Data) -> Self {
        if let Data::Pc(pc) = value {
            pc
        } else {
            Pc::default()
        }
    }
}

impl Pc {
    pub fn to_u8(self) -> u8 {
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

impl From<Data> for Mpb {
    fn from(value: Data) -> Self {
        if let Data::Tempo(tempo) = value {
            tempo
        } else {
            Mpb::default()
        }
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

impl From<Data> for Prog {
    fn from(value: Data) -> Self {
        if let Data::Program(prog) = value {
            prog
        } else {
            Prog::default()
        }
    }
}

impl BuildHasher for Prog {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    Reg(i8),
    None,
}

impl Default for Register {
    fn default() -> Self {
        Self::Reg(4)
    }
}

impl From<Data> for Register {
    fn from(value: Data) -> Self {
        if let Data::Register(register) = value {
            register
        } else {
            Register::default()
        }
    }
}

impl Add<i8> for Register {
    type Output = i8;
    fn add(self, rhs: i8) -> Self::Output {
        match self {
            Register::Reg(lhs) => lhs + rhs,
            Register::None => rhs,
        }
    }
}

impl BuildHasher for Register {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction<'a> {
    Midi(MidiMessage),
    Meta(MetaMessage<'a>),
}

pub enum Type {
    Pc,
    Reg,
    Length,
    Velocity,
    Tempo,
    Program,
    // PR,
    // PRL,
    // PRLV,
    // PRLVT,
    // PRLVTP,
    // PvR,
    // PvRvL,
    // PvRvLvV,
    // PvRvLvVvT,
    // PvRvLvVvTvP
}

pub enum Data {
    Pc(Pc),
    Length(Length),
    Velocity(Velocity),
    Program(Prog),
    Tempo(Mpb),
    Register(Register),
    None,
}

// impl BitOr for Type {
//     type Output = Type;
//     fn bitor(self, rhs: Self) -> Self::Output {

//     }
// }

// impl BitAnd for Type {
//     type Output = u8;
//     fn bitand(self, rhs: Self) -> Self::Output {
//         self as u8 & rhs as u8
//     }
// }
