use crate::{
    WriteGuard,
    point::Point,
};
pub trait Generator<P, T>: Send where P: Point {
    fn generate(&mut self, chunk: &mut WriteGuard<'_, P, T>, core_region: &[P; 2], umbra: &[P; 2]);
}

pub struct GeneratorSequence<P, T> where P: Point {
    generators: Vec<Box<dyn Generator<P, T>>>,
}

impl<P: Point, T> GeneratorSequence<P, T> {
    pub fn new(generators: Vec<Box<dyn Generator<P, T>>>) -> Self {
        Self {
            generators,
        }
    }
}

impl<P: Point, T> Generator<P, T> for GeneratorSequence<P, T> {
    fn generate(&mut self, chunk: &mut WriteGuard<'_, P, T>, core_region: &[P; 2], umbra: &[P; 2]) {
        for generator in &mut self.generators {
            generator.generate(chunk, core_region, umbra);
        }
    }
}
