use noise::{Fbm, Seedable, NoiseFn};
use rand::{
    Rng,
};

use super::{
    Generator, WriteGuard,
    analysis::Passable,
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

impl<Tile: Passable+Default> Generator<[i32; 2], Tile> for FbmGenerator {
    fn new_chunk<'a>(&self, chunk: &'a mut WriteGuard<'a, [i32; 2], Tile>, _: &'a mut WriteGuard<'a, [i32; 2], Tile>) {
        for (p, tile) in chunk.enumerate_mut() {
            let n = self.noise.get([p[0] as f64, p[1] as f64]);
            tile.set_passable(n > 0.1);
        }
    }
}
