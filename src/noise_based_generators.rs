use noise::{Fbm, Seedable, NoiseFn};
use rand::{
    Rng,
};

use super::{
    generator::Generator, WriteGuard,
    point::Point, analysis::Passable,
};

#[derive(Debug)]
pub struct FbmGenerator {
    noise: Fbm,
}

impl FbmGenerator {
    pub fn new(octaves: usize, persistence: f64, frequency: f64) -> Self {
        let mut noise = Fbm::new().set_seed(rand::thread_rng().gen());
        noise.octaves = octaves;
        noise.persistence = persistence;
        noise.frequency = frequency;
        Self {
            noise,
        }
    }
}

impl<T: Passable> Generator<[i32; 2], T> for FbmGenerator {
    fn generate(&mut self, chunk: &mut WriteGuard<'_, [i32; 2], T>, core_region: &[[i32; 2]; 2], _umbra: &[[i32; 2]; 2]) {
        for p in <[i32; 2] as Point>::points_in_region(core_region) {
            let mut tile = chunk.get_mut(&p).unwrap();
            let n = self.noise.get([p[0] as f64, p[1] as f64]);
            tile.set_passable(n > 0.1);
        }
    }
}
