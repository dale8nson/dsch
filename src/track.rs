use std::any::Any;

pub type Transformer<T> = fn(data: T) -> T;

pub struct Seq<T>
where
    T:,
{
    layers: Vec<Transformer<T>>,
}

impl<T> Seq<T>
where
    T: Iterator,
{
    pub fn new() -> Self {
        Self {
            layers: Vec::<Transformer<T>>::new(),
        }
    }

    pub fn layer(mut self, l: Transformer<T>) -> Self {
        self.layers.push(l);
        self
    }

    pub fn add_layer(&mut self, l: Transformer<T>) {
        self.layers.push(l);
    }

    pub fn build(&self) {
        todo!();
    }
}
