#![allow(unused)]
pub struct Category<T> {
    objs: Vec<T>,
    morphs: Vec<Box<dyn Fn(T) -> T>>,
}

// pub enum Class {
//     Monad(Monad),
//     Functor(Functor),
//     Combinator(Combinator),
// }

pub const fn id<A>(a: A) -> A {
    a
}

#[derive(Debug, Clone, Copy)]
pub struct Monad<A>(A);

impl<A> Monad<A> {
    pub fn ret(a: A) -> Monad<A> {
        Monad(a)
    }
    pub fn bind<B, F: FnOnce(A) -> Monad<B>>(self, mut f: F) -> Monad<B> {
        f(self.0)
    }
}

pub fn compose<A, B, C, F, G>(f: F, g: G) -> Box<impl Fn(A) -> C>
where
    F: Fn(A) -> B + 'static,
    G: Fn(B) -> C + 'static,
{
    Box::new(move |t| g(f(t)))
}

pub type F<'a, Args, Ret> = Box<dyn FnMut(Args) -> Ret + 'a>;
// pub type M<'a, Args, Ret> = Monad<F<'a, Args, Ret>>;

pub struct Functor<A, B>(Box<dyn Fn(A) -> B>);

impl<A, B> Functor<A, B> {
    pub fn apply(&self, items: Vec<A>) -> Vec<B> {
        items.into_iter().map(|item| self.0(item)).collect()
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

// Y f = f(Y f)
// F: Fn(A) -> B, G: Fn(C) -> D, H: Fn(A) -> D -> fn(F, G) -> H
// f . g
// type Y<F: Fn(Y<F>) -> Y<F>> = Combinator<F, Y<F>, Fn(Y<F>) -> Y<F>, Fn(Y<F>) -> Y<F>>;
