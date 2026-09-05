#![allow(unused)]

use std::ops::{Add, AddAssign, Shl, Shr};

use crate::compiler::codegen::Interpolant;
pub struct Category<T> {
    objs: Vec<T>,
    morphs: Vec<Box<dyn Fn(T) -> T>>,
}

// pub enum Class {
//     Monad(Monad),
//     Functor(Functor),
//     Combinator(Combinator),
// }

// pub const fn id<A>(a: A) -> A {
//     a
// }

#[derive(Debug, Clone, Default)]
pub struct Monad<A: Clone>(A);

impl<A: Clone> Monad<A> {
    pub fn ret(a: A) -> Monad<A> {
        Monad::<A>(a)
    }
    pub fn bind<B: Clone, F: FnOnce(A) -> Monad<B>>(self, mut f: F) -> Monad<B> {
        f(self.0)
    }
}

impl<A: Clone, B: Clone> Add<Box<dyn FnOnce(A) -> Monad<B>>> for Monad<A> {
    type Output = Monad<B>;

    fn add(self, rhs: Box<dyn FnOnce(A) -> Monad<B>>) -> Self::Output {
        self.bind(|a| rhs(a))
    }
}

impl<A: Clone> AddAssign<Box<dyn FnOnce(A) -> Monad<A>>> for Monad<A> {
    fn add_assign(&mut self, rhs: Box<dyn FnOnce(A) -> Monad<A>>) {
        *self = self.clone().bind(rhs);
    }
}

fn id<T>(t: T) -> T {
    t
}

// fn combine<T>(f: fn(T) -> T, g: fn(T) -> T, t: fn(T) -> T) -> fn(T) -> T {
//     g(f(t))
// }

#[derive(Debug, Clone)]
pub struct Functor<T>(fn(T) -> T, fn(T) -> T);

impl<T> Default for Functor<T> {
    fn default() -> Self {
        Self(id::<T>, id::<T>)
    }
}

impl<T> Shl for Functor<T> {
    type Output = Self;
    fn shl(self, mut rhs: Self) -> Self::Output {
        Self(self.1, rhs.1)
    }
}

impl<T> Shr for Functor<T> {
    type Output = Functor<T>;

    fn shr(self, rhs: Self) -> Self::Output {
        Self(rhs.1, self.1)
    }
}

impl<T> Functor<T> {
    pub fn new(f: fn (T) -> T) -> Self {
        Self(id::<T>, f)
    }

    pub fn call(&mut self, input: T) -> T {
        self.1(self.0(input))
    }
}

pub struct Combinator<A, B, C, D>(
    Box<dyn Fn(Box<dyn Fn(A) -> B>, Box<dyn Fn(C) -> D>) -> Box<dyn Fn(A) -> D>>,
);

impl<A: 'static, B: 'static, C: 'static, D: 'static> Combinator<A, B, C, D> {
    pub fn compose(self, f: Box<dyn Fn(A) -> B>, g: Box<dyn Fn(C) -> D>) -> impl Fn(A) -> D {
        Box::new(self.0(Box::new(f), Box::new(g)))
    }
}

pub fn tail<T>(ts: Vec<T>) -> Vec<T> {
    if ts.is_empty() {
        vec![]
    } else {
        ts.into_iter().skip(1).collect()
    }
}

pub fn head<T: Default>(ts: Vec<T>) -> T {
    ts.into_iter().take(1).next().unwrap_or_default()
}

// Y f = f(Y f)
// F: Fn(A) -> B, G: Fn(C) -> D, H: Fn(A) -> D -> fn(F, G) -> H
// f . g
// type Y<F: Fn(Y<F>) -> Y<F>> = Combinator<F, Y<F>, Fn(Y<F>) -> Y<F>, Fn(Y<F>) -> Y<F>>;

// pub struct F;

// impl F {
//     pub fn from_to<U, T: From<U>>(u: U) -> T {
//         T::from(u)
//     }

//     pub fn call<T, U, F_: FnMut(&mut F_, T) -> U>(f: &mut F_, t: T) -> U {
//         <F_ as FnMut(T) -> U>::call_mut(f, t)
//     }

//     pub fn map<T, U, V, F_: FnMut(T) -> U, G_: FnMut(U) -> V>() -> impl FnMut(T) -> V {
//         <G_ as FnMut<(U), Output = V>>::call_mut
//     }

// }
