#![allow(unused)]
pub mod state;
#[macro_use]
pub mod utils;

use std::{
    any::{Any, TypeId}, cell::{LazyCell, OnceCell, RefCell}, collections::{BTreeSet, HashSet}, fmt::{Debug, Display}, fs::OpenOptions, hash::{BuildHasher, DefaultHasher}, i8, iter::Sum, marker::PhantomData, ops::{Add, AddAssign, Deref, Div, Mul, Rem, Shl, Sub, SubAssign}, rc::Rc, u8, u64, usize
};

use derive_more::{From, Into};

use crate::compiler::{
    ast::*,
    codegen::{
        state::State,
        utils::{ColorIter, TextStyle},
    },
    functional::*,
};
use midly::SmpteTime;
pub use midly::{MidiMessage, num::*};

use num_bigint::BigInt;
use num_rational::{Ratio, Rational64};
use num_traits::{ToPrimitive, cast::FromPrimitive};

pub const PPQ: u15 = u15::new(25200);

pub const PC_ID: TypeId = TypeId::of::<Pc>();
pub const LENGTH_ID: TypeId = TypeId::of::<Length>();
pub const VELOCITY_ID: TypeId = TypeId::of::<Velocity>();
pub const PROGRAM_ID: TypeId = TypeId::of::<Prog>();
pub const REGISTER_ID: TypeId = TypeId::of::<Register>();
pub const TEMPO_ID: TypeId = TypeId::of::<Mpb>();

pub const SB: char = '\u{1d173}';
pub const EB: char = '\u{1d174}';

pub trait Normalize: Clone + From<f64> + From<Self::Output> {
    type Output: Add<f64>
        + Add<Self::Output>
        + Sub<f64>
        + Sub<Self::Output>
        + Mul<f64>
        + Mul<Self::Output>
        + Div<f64>
        + Div<Self::Output>;

    fn norm(&self) -> f64;
}

pub trait Interpolant:
    // std::fmt::Debug
    Default
    + Debug
    + Clone
    + From<Data>
    + Into<Data>
    + Into<<Self as Interpolant>::T>
    + From<<Self as Interpolant>::T>
    + From<f64>
    + Into<f64>

    // + From<usize>
    // + Into<usize>
    // + Into<f64>
    // + From<f64>
    // + PartialOrd
    // + Ord
    // + Add<Output = Self>
    // + Sub<Output = Self>
    // + AddAssign
    // + SubAssign
    // + Div<Output = Self>
    // + Mul<Self, Output = Self>
    // + Copy
{
    type T: Interpolant;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self>>>;

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self>>>;


}

pub trait ToString {
    fn to_string(&self) -> String;
}

#[derive(Debug, Clone, Ord, Eq, From)]
pub enum Length {
    MicroSeconds(Ratio<u64>),
    None,
}

impl Normalize for Length {
    type Output = f64;
    fn norm(&self) -> Self::Output {
        1.0 / self.as_f64()
    }
}

impl ToString for Length {
    fn to_string(&self) -> String {
        "LENGTH".to_string()
    }
}

impl Interpolant for Length {
    type T = Length;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        Some(state.lengths(ctx).unwrap_or_default())
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.lengths_mut(ctx)
    }
}

impl From<Length> for f64 {
    fn from(value: <Length as Interpolant>::T) -> Self {
        value.as_f64()
    }
}

impl From<Data> for <Length as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Length(length) = value {
            length
        } else {
            Length::default()
        }
    }
}

impl From<<Length as Interpolant>::T> for usize {
    fn from(value: <Length as Interpolant>::T) -> Self {
        value.as_usize()
    }
}

impl From<u64> for Length {
    fn from(value: u64) -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_u64(value).unwrap())
    }
}

impl Length {
    pub fn as_u64(&self) -> u64 {
        if let Length::MicroSeconds(uint) = self {
            uint.to_u64().unwrap()
        } else {
            0
        }
    }

    pub fn as_u32(&self) -> u32 {
        if let Length::MicroSeconds(uint) = self {
            uint.to_u32().unwrap()
        } else {
            0
        }
    }

    pub fn as_f64(&self) -> f64 {
        if let Length::MicroSeconds(n) = self {
            n.to_f64().unwrap()
        } else {
            0.
        }
    }

    pub fn as_usize(&self) -> usize {
        if let Length::MicroSeconds(n) = self {
            n.to_usize().unwrap()
        } else {
            0
        }
    }

    pub fn default_max() -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_u64(u64::MAX).unwrap())
    }

    pub fn to_note(self, tempo: &Tempo) -> String {
        // dbg!(&self, &tempo);
        let note_value = 4. / (self.as_f64() / tempo.0.to_f64().unwrap());
        // dbg!(note_value);
        // eprintln!("{note_value}");
        let s = match note_value {
            128.0.. => "\u{1d164}\u{0020}\u{0020}",
            96.0 => "\u{1d164}\u{1D16D}\u{0020}",
            80.0 => "\u{1d164}\u{1D16D}\u{1D16D}",
            64.0.. => "\u{1d163}\u{0020}\u{0020}",
            48.0.. => "\u{1d163}\u{1D16D}\u{0020}",
            40.0.. => "\u{1d163}\u{1D16D}\u{1D16D}",
            32.0.. => "\u{1d162}\u{0020}\u{0020}",
            24.0.. => "\u{1d162}\u{1D16D}\u{0020}",
            20.0.. => "\u{1d162}\u{1D16D}\u{1D16D}",
            16.0.. => "\u{1d161}\u{0020}\u{0020}",
            12.0.. => "\u{1d161}\u{1D16D}\u{0020}",
            10.0.. => "\u{1d161}\u{1D16D}\u{1D16D}",
            8.0.. => "\u{1d160}\u{0020}\u{0020}",
            6.0.. => "\u{1d160}\u{1D16D}\u{0020}",
            5.0.. => "\u{1d160}\u{1D16D}\u{1D16D}",
            // 128.0.. => "\u{1d158}\u{1d165}\u{1d172}\u{0020}\u{0020}",
            // 96.0 => "\u{1d158}\u{1d165}\u{1d172}\u{1D16D}\u{0020}",
            // 80.0 => "\u{1d158}\u{1d165}\u{1d172}\u{1D16D}\u{1D16D}",
            // 64.0.. => "\u{1d158}\u{1d165}\u{1d171}\u{0020}\u{0020}",
            // 48.0.. => "\u{1d158}\u{1d165}\u{1d171}\u{1D16D}\u{0020}",
            // 40.0.. => "\u{1d158}\u{1d165}\u{1d171}\u{1D16D}\u{1D16D}",
            // 32.0.. => "\u{1d158}\u{1d165}\u{1d170}\u{0020}\u{0020}",
            // 24.0.. => "\u{1d158}\u{1d165}\u{1d170}\u{1D16D}\u{0020}",
            // 20.0.. => "\u{1d158}\u{1d165}\u{1d170}\u{1D16D}\u{1D16D}",
            // 16.0.. => "\u{1d158}\u{1d165}\u{1d16f}\u{0020}\u{0020}",
            // 12.0.. => "\u{1d158}\u{1d165}\u{1d16f}\u{1D16D}\u{0020}",
            // 10.0.. => "\u{1d158}\u{1d165}\u{1d16f}\u{1D16D}\u{1D16D}",
            // 8.0.. => "\u{1d158}\u{1d165}\u{1d16e}\u{0020}\u{0020}",
            // 6.0.. => "\u{1d158}\u{1d165}\u{1d16e}\u{1D16D}\u{0020}",
            // 5.0.. => "\u{1d158}\u{1d165}\u{1d16e}\u{1D16D}\u{1D16D}",
            4.0.. => "\u{1d15f}\u{0020}\u{0020}",
            3.0.. => "\u{1d15f}\u{1D16D}\u{0020}",
            2.5.. => "\u{1d15f}\u{1D16D}\u{0020}",
            2.0.. => "\u{1d15e}\u{0020}\u{0020}",
            1.5.. => "\u{1d15e}\u{1D16D}\u{0020}",
            1.25.. => "\u{1d15e}\u{1D16D}\u{0020}",
            1.0.. => "\u{1d15d}\u{0020}\u{0020}",
            0.75.. => "\u{1d15d}\u{1D16D}\u{0020}",
            0.625.. => "\u{1d15d}\u{1D16D}\u{0020}",
            0.5.. => "\u{1d15c}\u{0020}\u{0020}",
            0.375.. => "\u{1d15c}\u{1D16D}\u{0020}",
            0.3125.. => "\u{1d15c}\u{1D16D}\u{1D16D}",
            _ => "\u{1d158}\u{0020}\u{0020}",
        };
        format!(
            "{}{}{}{}",
            TextStyle::ResetColor,
            TextStyle::IntenseBoldGreen,
            s,
            TextStyle::ResetColor
        )
    }

    pub fn to_rest(self, tempo: &Tempo) -> String {
        let note_value = 4. / (self.as_f64() / tempo.0.to_f64().unwrap());
        // dbg!(note_value);
        // eprintln!("{note_value}");
        let s = match note_value {
            128.0.. => "\u{1d142}\u{0020}\u{0020}",
            96.0 => "\u{1d142}\u{1D16D}\u{0020}",
            80.0 => "\u{1d142}\u{1D16D}\u{1D16D}",
            64.0.. => "\u{1d141}\u{0020}\u{0020}",
            48.0.. => "\u{1d141}\u{1D16D}\u{0020}",
            40.0.. => "\u{1d141}\u{1D16D}\u{1D16D}",
            32.0.. => "\u{1d140}\u{0020}\u{0020}",
            24.0.. => "\u{1d140}\u{1D16D}\u{0020}",
            20.0.. => "\u{1d140}\u{1D16D}\u{1D16D}",
            16.0.. => "\u{1d13f}\u{0020}\u{0020}",
            12.0.. => "\u{1d13f}\u{1D16D}\u{0020}",
            10.0.. => "\u{1d13f}\u{1D16D}\u{1D16D}",
            8.0.. => "\u{1d13e}\u{0020}\u{0020}",
            6.0.. => "\u{1d13e}\u{1D16D}\u{0020}",
            5.0.. => "\u{1d13e}\u{1D16D}\u{1D16D}",
            4.0.. => "\u{1d13d}\u{0020}\u{0020}",
            3.0.. => "\u{1d13d}\u{1D16D}\u{0020}",
            2.5.. => "\u{1d13d}\u{1D16D}\u{0020}",
            2.0.. => "\u{1d13c}\u{0020}\u{0020}",
            1.5.. => "\u{1d13c}\u{1D16D}\u{0020}",
            1.25.. => "\u{1d13c}\u{1D16D}\u{0020}",
            1.0.. => "\u{1d13b}\u{0020}\u{0020}",
            0.75.. => "\u{1d13b}\u{1D16D}\u{0020}",
            0.625.. => "\u{1d13b}\u{1D16D}\u{0020}",
            0.5.. => "\u{1d13a}\u{0020}\u{0020}",
            0.375.. => "\u{1d13a}\u{1D16D}\u{0020}",
            0.3125.. => "\u{1d13a}\u{1D16D}\u{1D16D}",
            _ => "\u{1d129}\u{0020}\u{0020}",
        };
        format!(
            "{}{}{}{}",
            TextStyle::ResetColor,
            TextStyle::IntenseBoldRed,
            s,
            TextStyle::ResetColor
        )
    }

    pub fn as_ratio(&self) -> Ratio<u64> {
        match self {
            Length::MicroSeconds(ratio) => *ratio,
            Length::None => Ratio::<u64>::default(),
        }
    }
}

/*
 * 1D173 ž MUSICAL SYMBOL BEGIN BEAM
 * 1D174 ſ MUSICAL SYMBOL END BEAM
 * 1D175 ƀ MUSICAL SYMBOL BEGIN TIE
 * 1D176 Ɓ MUSICAL SYMBOL END TIE
 * 1D177 Ƃ MUSICAL SYMBOL BEGIN SLUR
 * 1D178 ƃ MUSICAL SYMBOL END SLUR
 * 1D179 Ƅ MUSICAL SYMBOL BEGIN PHRASE
 * 1D17A ƅ MUSICAL SYMBOL END PHRASE */

impl Default for Length {
    fn default() -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_u64(0).unwrap())
    }
}

impl From<Absolute> for Length {
    fn from(value: Absolute) -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_u64(value.as_u64()).unwrap())
    }
}

impl From<Fixed> for Length {
    fn from(value: Fixed) -> Self {
        let Fixed { minutes, seconds } = value;
        Length::MicroSeconds(Ratio::<u64>::from_u64(minutes * 60_000_000).unwrap())
            + Length::MicroSeconds(Ratio::<u64>::from_u64(seconds * 1_000_000).unwrap())
    }
}

impl From<usize> for Length {
    fn from(value: usize) -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_u64(value as u64).unwrap())
    }
}

impl From<f64> for Length {
    fn from(value: f64) -> Self {
        Length::MicroSeconds(Ratio::<u64>::from_f64(f64::round(value)).unwrap())
    }
}

impl Mul<Length> for f64 {
    type Output = f64;
    fn mul(self, rhs: Length) -> Self::Output {
        self * rhs.as_f64()
    }
}

impl Div<usize> for Length {
    type Output = Length;
    fn div(self, rhs: usize) -> Self::Output {
        self / Length::MicroSeconds(Ratio::<u64>::from_usize(rhs).unwrap())
    }
}

impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        Ratio::<u64>::from_f64(self.as_f64()).unwrap_or_default()
            == Ratio::<u64>::from_f64(other.as_f64()).unwrap_or_default()
    }
}

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.as_ratio().cmp(&other.as_ratio()))
    }
}

impl Add for Length {
    type Output = Length;
    fn add(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(self.as_ratio() + rhs.as_ratio())
    }
}

impl Sum for Length {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Length::default(), |sum, l| sum + l)
    }
}

impl Mul for Length {
    type Output = Length;
    fn mul(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(Ratio::<u64>::from_f64(self.as_f64() * rhs.as_f64()).unwrap())
    }
}

impl Div for Length {
    type Output = Length;
    fn div(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(Ratio::<u64>::from_f64(self.as_f64() / rhs.as_f64()).unwrap())
        // Length::MicroSeconds(self.as_u64() / rhs.as_u64())
    }
}

impl Rem for Length {
    type Output = Length;
    fn rem(self, rhs: Self) -> Self::Output {
        // dbg!(&self, &rhs);
        Length::MicroSeconds(Ratio::<u64>::from_u64(self.as_u64() % rhs.as_u64()).unwrap())
    }
}

impl Sub for Length {
    type Output = Length;
    fn sub(self, rhs: Self) -> Self::Output {
        Length::MicroSeconds(Ratio::<u64>::from_f64(self.as_f64() - rhs.as_f64()).unwrap())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
pub struct Velocity(pub u8);

impl Normalize for Velocity {
    type Output = f64;
    fn norm(&self) -> Self::Output {
        1.0 / self.0.min(i8::MAX as u8).max(0) as f64
    }
}

impl ToString for Velocity {
    fn to_string(&self) -> String {
        "VELOCITY".to_string()
    }
}

impl Interpolant for Velocity {
    type T = Velocity;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        Some(state.velocities(ctx).unwrap_or_default())
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.velocities_mut(ctx)
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Velocity(80)
    }
}

impl From<Velocity> for usize {
    fn from(value: Velocity) -> Self {
        value.0 as usize
    }
}

impl From<Velocity> for f64 {
    fn from(value: Velocity) -> Self {
        value.0 as f64
    }
}

impl From<f64> for Velocity {
    fn from(value: f64) -> Self {
        Velocity(value as u8)
    }
}

impl From<Data> for <Velocity as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Velocity(velocity) = value {
            velocity
        } else {
            Velocity::default()
        }
    }
}

impl From<Absolute> for Velocity {
    fn from(value: Absolute) -> Self {
        Velocity(value.as_usize().max(u8::MAX as usize) as u8)
    }
}

impl From<usize> for Velocity {
    fn from(value: usize) -> Self {
        Velocity(value.min(u8::MAX as usize) as u8)
    }
}

impl From<Dynamic> for Velocity {
    fn from(value: Dynamic) -> Self {
        let Dynamic(string) = value;
        let vel = match string.as_str() {
            "ppp" => 20,
            "pp" => 39,
            "mp" => 71,
            "p" => 57,
            "mf" => 84,
            "f" => 98,
            "ff" => 113,
            "fff" => 127,
            _ => 80,
        };
        Velocity(vel)
    }
}

impl Add for Velocity {
    type Output = Velocity;

    fn add(self, rhs: Self) -> Self::Output {
        Velocity(self.0 + rhs.0)
    }
}

impl AddAssign for Velocity {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for Velocity {
    type Output = Velocity;

    fn sub(self, rhs: Self) -> Self::Output {
        Velocity(self.0 - rhs.0)
    }
}

impl SubAssign for Velocity {
    fn sub_assign(&mut self, rhs: Self) {
        *self = Velocity(self.0 - rhs.0);
    }
}

impl Div for Velocity {
    type Output = Velocity;

    fn div(self, rhs: Self) -> Self::Output {
        Velocity(f64::round(self.0 as f64 / rhs.0.max(1).min(u8::MAX) as f64) as u8)
    }
}

impl Div<usize> for Velocity {
    type Output = Velocity;

    fn div(self, rhs: usize) -> Self::Output {
        Velocity(f64::round(self.0 as f64 / (rhs.max(1).min(u8::MAX as usize) as f64)) as u8)
    }
}

impl Mul<Velocity> for Velocity {
    type Output = Velocity;

    fn mul(self, rhs: Velocity) -> Self::Output {
        Velocity(self.0 * rhs.0)
    }
}

impl PartialOrd for Velocity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Velocity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

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
    Root,
    #[default]
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
            // (Ctx::Root, Ctx::Id(0)) | (Ctx::Id(0), Ctx::Root) => true,
            (Ctx::Id(n1), Ctx::Id(n2)) => n1 == n2,
            (Ctx::None, Ctx::None) => true,
            (Ctx::Root, Ctx::Root) => true,
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

#[derive(Debug, Clone, Copy, Default, From)]
pub enum Pc {
    Class(f64),
    #[default]
    None,
}

impl Normalize for Pc {
    type Output = f64;
    fn norm(&self) -> Self::Output {
        1.0 / self.as_f64() as f64
    }
}

impl ToString for Pc {
    fn to_string(&self) -> String {
        "PC".to_string()
    }
}

impl Interpolant for Pc {
    type T = Pc;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        state.pcs(ctx)
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.pcs_mut(ctx)
    }
}

impl Eq for Pc {}

impl PartialEq for Pc {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Class(l0), Self::Class(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Ord for Pc {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_f64().total_cmp(&other.as_f64())
    }
}

impl PartialOrd for Pc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_f64().partial_cmp(&other.as_f64())
    }
}

impl From<Pc> for f64 {
    fn from(value: Pc) -> Self {
        value.as_f64() as f64
    }
}

impl From<Pc> for usize {
    fn from(value: Pc) -> Self {
        value.as_f64() as usize
    }
}

// impl From<f64> for Pc {
//     fn from(value: f64) -> Self {
//         Pc::Class(value)
//     }
// }

impl Add for Pc {
    type Output = Pc;

    fn add(self, rhs: Self) -> Self::Output {
        Pc::Class(self.as_f64() + rhs.as_f64())
    }
}

impl Add<Absolute> for Pc {
    type Output = Pc;

    fn add(self, rhs: Absolute) -> Self::Output {
        Pc::Class(self.as_f64() + rhs.as_f64())
    }
}

impl Sub<Absolute> for Pc {
    type Output = Pc;

    fn sub(self, rhs: Absolute) -> Self::Output {
        let lhs = self.as_f64();
        let rhs = rhs.as_f64();
        Pc::Class(lhs - rhs)
    }
}

impl AddAssign for Pc {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Sub for Pc {
    type Output = Pc;

    fn sub(self, rhs: Self) -> Self::Output {
        Pc::Class(self.as_f64() - rhs.as_f64())
    }
}

impl SubAssign for Pc {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl From<usize> for Pc {
    fn from(value: usize) -> Self {
        Pc::Class(value as f64)
    }
}

impl Div<Pc> for Pc {
    type Output = Pc;

    fn div(self, rhs: Pc) -> Self::Output {
        Pc::Class(self.as_f64() / rhs.as_f64())
    }
}

impl Div<usize> for Pc {
    type Output = Pc;

    fn div(self, rhs: usize) -> Self::Output {
        Pc::Class(self.as_f64() as f64 / rhs as f64)
    }
}

impl Mul<Pc> for Pc {
    type Output = Pc;

    fn mul(self, rhs: Pc) -> Self::Output {
        Pc::Class(self.as_f64() * rhs.as_f64())
    }
}

impl From<Data> for <Pc as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Pc(pc) = value {
            pc
        } else {
            Pc::default()
        }
    }
}

impl From<Absolute> for Pc {
    fn from(value: Absolute) -> Self {
        Pc::Class(value.as_usize().max(u8::MAX as usize) as f64)
    }
}

impl Pc {
    pub fn as_f64(&self) -> f64 {
        match *self {
            Pc::Class(float) => float,
            Pc::None => 0.0,
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialOrd, Ord, From)]
/// Microseconds per beat
pub struct Mpb(pub Ratio<u64>);

pub type Tempo = Mpb;

impl ToString for Mpb {
    fn to_string(&self) -> String {
        "TEMPO".to_string()
    }
}

impl Normalize for Mpb {
    type Output = f64;
    fn norm(&self) -> Self::Output {
        (self.0.clone() * Ratio::<u64>::from_f64(1.0).unwrap())
            .to_f64()
            .unwrap()
    }
}

impl Interpolant for Mpb {
    type T = Mpb;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        state.tempos(ctx)
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.tempos_mut(ctx)
    }
}

impl Default for Mpb {
    fn default() -> Self {
        Mpb(Ratio::<u64>::from_f64(60_000_000 as f64 / 120 as f64).unwrap())
    }
}

impl Add for Mpb {
    type Output = Mpb;

    fn add(self, rhs: Self) -> Self::Output {
        Mpb(self.0 + rhs.0)
    }
}

impl AddAssign for Mpb {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone() + rhs;
    }
}

impl Sub for Mpb {
    type Output = Mpb;

    fn sub(self, rhs: Self) -> Self::Output {
        Mpb(self.0 - rhs.0)
    }
}

impl SubAssign for Mpb {
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.clone() - rhs;
    }
}

impl Mul<Mpb> for Mpb {
    type Output = Mpb;

    fn mul(self, rhs: Mpb) -> Self::Output {
        Mpb(self.0 * rhs.0)
    }
}

impl Div for Mpb {
    type Output = Mpb;

    fn div(self, rhs: Self) -> Self::Output {
        Mpb(self.0 / rhs.0)
    }
}

impl From<Absolute> for Mpb {
    fn from(value: Absolute) -> Self {
        let microseconds_per_quarter_note = Ratio::<u64>::from_f64(60_000_000. / value).unwrap();
        Mpb(microseconds_per_quarter_note)
    }
}

impl From<usize> for Mpb {
    fn from(value: usize) -> Self {
        Mpb(Ratio::<u64>::from_usize(value).unwrap())
    }
}

impl From<Mpb> for usize {
    fn from(value: Mpb) -> Self {
        value.0.to_usize().unwrap()
    }
}

impl From<Length> for Mpb {
    fn from(value: Length) -> Self {
        Mpb(Ratio::<u64>::from_f64(value.as_f64()).unwrap())
    }
}

impl From<Mpb> for f64 {
    fn from(value: Mpb) -> Self {
        value.0.to_f64().unwrap()
    }
}

impl From<f64> for Mpb {
    fn from(value: f64) -> Self {
        Mpb(Ratio::<u64>::from_f64(value).unwrap())
    }
}

impl PartialEq for Mpb {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<Data> for <Mpb as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Tempo(tempo) = value {
            tempo
        } else {
            Mpb::default()
        }
    }
}

impl Mul<Mpb> for f64 {
    type Output = f64;
    fn mul(self, rhs: Mpb) -> Self::Output {
        self * rhs.0.to_f64().unwrap()
    }
}

impl BuildHasher for Mpb {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From)]
pub struct Prog(pub u8);

impl Normalize for Prog {
    type Output = f64;

    fn norm(&self) -> Self::Output {
        1.0 / self.0 as f64
    }
}

impl ToString for Prog {
    fn to_string(&self) -> String {
        "PROGRAM".to_string()
    }
}

impl Interpolant for Prog {
    type T = Prog;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        Some(state.programs(ctx).unwrap_or_default())
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.programs_mut(ctx)
    }
}

impl Default for Prog {
    fn default() -> Self {
        Prog(0)
    }
}

impl From<Prog> for f64 {
    fn from(value: <Prog as Interpolant>::T) -> Self {
        value.0 as f64
    }
}

impl From<Data> for <Prog as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Program(prog) = value {
            prog
        } else {
            Prog::default()
        }
    }
}

impl From<Absolute> for Prog {
    fn from(value: Absolute) -> Self {
        Prog(value.as_usize().max(u8::MAX as usize) as u8)
    }
}

impl From<usize> for Prog {
    fn from(value: usize) -> Self {
        Prog(value.min(u8::MAX as usize) as u8)
    }
}

impl From<f64> for Prog {
    fn from(value: f64) -> Self {
        Prog(value.min(127 as f64).max(0 as f64) as u8)
    }
}

impl From<Prog> for usize {
    fn from(value: Prog) -> Self {
        value.0 as usize
    }
}

impl Add for Prog {
    type Output = Prog;

    fn add(self, rhs: Self) -> Self::Output {
        Prog(self.0 + rhs.0)
    }
}

impl AddAssign for Prog {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Prog {
    type Output = Prog;

    fn sub(self, rhs: Self) -> Self::Output {
        Prog(self.0 - rhs.0)
    }
}

impl SubAssign for Prog {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Div for Prog {
    type Output = Prog;

    fn div(self, rhs: Self) -> Self::Output {
        Prog(self.0 / rhs.0)
    }
}

impl Mul for Prog {
    type Output = Prog;

    fn mul(self, rhs: Self) -> Self::Output {
        Prog(self.0 * rhs.0)
    }
}

impl BuildHasher for Prog {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, From)]
pub enum Register {
    Reg(i8),
    None,
}

impl Normalize for Register {
    type Output = f64;

    fn norm(&self) -> Self::Output {
        1.0 / self.as_i8() as f64
    }
}

impl ToString for Register {
    fn to_string(&self) -> String {
        "REGISTER".to_string()
    }
}

impl Interpolant for Register {
    type T = Register;

    fn get_vec(state: &State, ctx: Ctx) -> Option<Vec<Vec<Self::T>>> {
        Some(state.registers(ctx).unwrap_or_default())
    }

    fn get_vec_mut(state: &mut State, ctx: Ctx) -> Option<&mut Vec<Vec<Self::T>>> {
        state.registers_mut(ctx)
    }
}

impl Default for Register {
    fn default() -> Self {
        Self::Reg(4)
    }
}

impl From<Register> for f64 {
    fn from(value: Register) -> Self {
        value.as_i8() as f64
    }
}

impl From<f64> for Register {
    fn from(value: f64) -> Self {
        Register::Reg(value.min(i8::MAX as f64).max(i8::MIN as f64) as i8)
    }
}

impl From<Data> for <Register as Interpolant>::T {
    fn from(value: Data) -> Self {
        if let Data::Register(register) = value {
            register
        } else {
            Register::default()
        }
    }
}

impl Add for Register {
    type Output = Register;

    fn add(self, rhs: Self) -> Self::Output {
        Register::Reg(self.as_i8() + rhs.as_i8())
    }
}

impl AddAssign for Register {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
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

impl Sub for Register {
    type Output = Register;

    fn sub(self, rhs: Self) -> Self::Output {
        let lhs = self.as_i8();
        let rhs = rhs.as_i8();
        // dbg!(lhs, rhs, i8::MIN + rhs, (i8::MIN + rhs) - rhs, lhs.max((i8::MIN + rhs) - rhs), lhs.max((i8::MIN + rhs) - rhs).max(-1));
        Register::Reg((lhs.max(i8::MIN + rhs) - rhs).max(-1))
        // Register::Reg(if lhs < i8::MIN + rhs.max(rhs * -1) {
        //     i8::MAX - (lhs + i8::MAX - rhs)
        // } else if i8::MAX - lhs < rhs.max(rhs * -1) {
        //     i8::MIN + (lhs - i8::MAX + rhs.max(rhs * -1))
        // } else {
        //     lhs - rhs
        // })
    }
}

impl SubAssign for Register {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Div for Register {
    type Output = Register;

    fn div(self, rhs: Self) -> Self::Output {
        Register::Reg(f64::round(self.as_i8() as f64 / rhs.as_i8() as f64) as i8)
    }
}

impl Mul for Register {
    type Output = Register;

    fn mul(self, rhs: Register) -> Self::Output {
        Register::Reg(self.as_i8() * rhs.as_i8())
    }
}

impl From<usize> for Register {
    fn from(value: usize) -> Self {
        Register::Reg(value.min(i8::MAX as usize) as i8)
    }
}

impl From<Register> for usize {
    fn from(value: Register) -> Self {
        value.as_i8() as usize
    }
}

impl From<Absolute> for Register {
    fn from(value: Absolute) -> Self {
        Register::Reg(value.as_usize().min(u8::MAX as usize) as i8)
    }
}

impl BuildHasher for Register {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> Self::Hasher {
        let s = std::hash::RandomState::new();
        s.build_hasher()
    }
}

impl Register {
    pub fn as_i8(&self) -> i8 {
        match *self {
            Register::Reg(n) => n,
            Register::None => 4,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum MetaMessage {
    /// For `Format::Sequential` MIDI file types, `TrackNumber` can be empty, and defaults to
    /// the track index.
    TrackNumber(Option<u16>),
    /// Arbitrary text associated to an instant.
    Text(Vec<u8>),
    /// A copyright notice.
    Copyright(Vec<u8>),
    /// Information about the name of the track.
    TrackName(Vec<u8>),
    /// Information about the name of the current instrument.
    InstrumentName(Vec<u8>),
    /// Arbitrary lyric information associated to an instant.
    Lyric(Vec<u8>),
    /// Arbitrary marker text associated to an instant.
    Marker(Vec<u8>),
    /// Arbitrary cue point text associated to an instant.
    CuePoint(Vec<u8>),
    /// Information about the name of the current program.
    ProgramName(Vec<u8>),
    /// Name of the device that this file was intended to be played with.
    DeviceName(Vec<u8>),
    /// Number of the MIDI channel that this file was intended to be played with.
    MidiChannel(u4),
    /// Number of the MIDI port that this file was intended to be played with.
    MidiPort(u7),
    /// Obligatory at track end.
    EndOfTrack,
    /// Amount of microseconds per beat (quarter note).
    ///
    /// Usually appears at the beggining of a track, before any midi events are sent, but there
    /// are no guarantees.
    Tempo(u24),
    /// The MIDI SMPTE offset meta message specifies an offset for the starting point of a MIDI
    /// track from the start of a sequence in terms of SMPTE time (hours:minutes:seconds:frames:subframes).
    ///
    /// [Reference](https://www.recordingblogs.com/wiki/midi-smpte-offset-meta-message)
    SmpteOffset(SmpteTime),
    /// In order of the MIDI specification, numerator, denominator, MIDI clocks per click, 32nd
    /// notes per quarter
    TimeSignature(u8, u8, u8, u8),
    /// As in the MIDI specification, negative numbers indicate number of flats and positive
    /// numbers indicate number of sharps.
    /// `false` indicates a major scale, `true` indicates a minor scale.
    KeySignature(i8, bool),
    /// Arbitrary data intended for the sequencer.
    /// This data is never sent to a device.
    SequencerSpecific(Vec<u8>),
    /// An unknown or malformed meta-message.
    ///
    /// The first `u8` is the raw meta-message identifier byte.
    /// The slice is the actual payload of the meta-message.
    Unknown(u8, Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Midi(MidiMessage),
    Meta(MetaMessage),
}

// #[derive(Clone)]
// pub struct Thunk(
//     pub fn(ctx: Ctx, data: Vec<Vec<Data>>, state: &mut State),
//     pub Vec<Vec<Data>>,
// );


#[derive(Clone)]
pub struct Thunk(
    pub Rc<RefCell<Box<dyn FnMut(Ctx, &mut State)>>>
);

// impl<'a, 'b> Clone for Thunk<'a, 'b> {
//     fn clone(&self) -> Self<'a, 'b> {
//         self.
//     }
// }


// impl Default for Thunk {
//     fn default() -> Self {
//         let cell = RefCell::<F: FnMut(Ctx, &mut State) + Sized>::new();
//         cell.set(|_, _| ());
//         Self(Rc::new(cell))
//     }
// }

impl Debug for Thunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "{}Thunk({:?}){}",
                color!(),
                (self.0).as_ptr(),
                TextStyle::ResetColor
            )
            .as_str(),
        )?;
        Ok(())
    }
}

impl Thunk {
    pub fn new(thunk: Box<dyn FnMut(Ctx, &mut State)>) -> Self {

        Self(Rc::new(RefCell::new(thunk)))
    }

    pub fn call(&mut self, ctx: Ctx, state: &mut State)

    {
        self.0.borrow_mut()(ctx, state);
    }
}

// #[macro_export]
// macro_rules! thunk {
//     ($expr: expr) => {
//         Thunk(Box::new($expr), data)

//     };
// }

// #[macro_export]
// macro_rules! thunk {
//     ($block: block) => {
//         Thunk(Box::new(|| $block))
//     };
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LifeCycleEvent {
    Sequencing,
    Composing,
    #[default]
    None
}

#[derive(Debug, Clone, Default, From)]
pub enum Data {
    Pc(Pc),
    Length(Length),
    Velocity(Velocity),
    Tempo(Mpb),
    Register(Register),
    /// TODO: Move to separate enum
    Interpolation(Interpolation),
    Program(Prog),
    #[default]
    None,
}

// impl From<<Velocity as Interpolate>::T> for Data {
//     fn from(value: Velocity) -> Self {
//         Data::Velocity(value)
//     }
// }

// impl From<Interpolation> for Data {
//     fn from(value: Interpolation) -> Self {
//         Data::Interpolation(value)
//     }
// }

// impl From<<Pc as Interpolate>::T> for Data {
//     fn from(value: Pc) -> Self {
//         Data::Pc(value)
//     }
// }

// impl From<<Length as Interpolate>::T> for Data {
//     fn from(value: Length) -> Self {
//         Data::Length(value)
//     }
// }

// impl From<<Mpb as Interpolate>::T> for Data {
//     fn from(value: Mpb) -> Self {
//         Data::Tempo(value)
//     }
// }

// impl From<<Register as Interpolate>::T> for Data {
//     fn from(value: Register) -> Self {
//         Data::Register(value)
//     }
// }

// impl From<<Prog as Interpolate>::T> for Data {
//     fn from(value: Prog) -> Self {
//         Data::Program(value)
//     }
// }

// pub struct Interpolator<U, T: FnMut<(U)>>(T, T);

// impl Interpolator {
//     pub fn interpolate(&mut self, data: Vec<T>) -> Vec<T> {
//         let len = f64::round(data.len() as f64 / self.functors.len() as f64) as usize;
//         let mut result = Vec::<T>::new();
//         let mut iter = data.iter();
//         self.functors.iter_mut().for_each(|f| {
//             let mut data_: Vec<T> = iter
//                 .by_ref()
//                 .take(len)
//                 .cloned()
//                 .map(|t| f.call(t))
//                 .collect();
//             result.extend(data_);
//         });
//         result
//     }
// }

// impl<T: Interpolate> Shl<Functor<T>> for Interpolator<T> {
//     type Output = Interpolator<T>;

//     fn shl(self, rhs: Functor<T>) -> Self::Output {
//         let mut functors_ = self.functors.clone();
//         let functors: Vec<Functor<T>> = if let Some(mut last) = functors_.last().cloned() {
//             functors_.push(last << rhs);
//             functors_
//         } else {
//             vec![rhs]
//         };

//         Self { functors }
//     }
// }

#[derive(Clone, Copy, Debug, Default)]
pub enum Layer {
    #[default]
    Heterogenous,
    Homogenous(HomogenousLayer),
}

#[derive(Debug, Clone, Copy)]
pub enum HomogenousLayer {
    Length,
    Pc,
    Prog,
    Register,
    Velocity,
    Tempo,
}

#[derive(Default, Debug)]
pub struct Slice {
    pcs: Vec<Pc>,
    registers: Vec<Register>,
    lengths: Vec<Length>,
    velocities: Vec<Velocity>,
    tempos: Vec<Tempo>,
    programs: Vec<Prog>,
}

impl Slice {
    pub fn new(
        pcs: Vec<Pc>,
        registers: Vec<Register>,
        lengths: Vec<Length>,
        velocities: Vec<Velocity>,
        tempos: Vec<Tempo>,
        programs: Vec<Prog>,
    ) -> Self {
        Self {
            pcs,
            registers,
            lengths,
            velocities,
            tempos,
            programs,
        }
    }

    pub fn data(
        &self,
    ) ->
    (
        &[Pc],
        &[Register],
        &[Length],
        &[Velocity],
        &[Tempo],
        &[Prog]
    )

    // (
    //     Vec<Pc>,
    //     Vec<Register>,
    //     Vec<Length>,
    //     Vec<Velocity>,
    //     Vec<Tempo>,
    //     Vec<Prog>,
    // )
    {
        (
            self.pcs.as_slice(),
            self.registers.as_slice(),
            self.lengths.as_slice(),
            self.velocities.as_slice(),
            self.tempos.as_slice(),
            self.programs.as_slice(),
        )
    }
}
