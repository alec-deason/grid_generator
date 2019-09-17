use noise::{Fbm, Seedable, NoiseFn};
use rand::{
    Rng,
};

use super::{
    Chunks, Generator,
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

impl<TextureType: Passable> Generator<TextureType> for FbmGenerator {
    fn new_chunk(&mut self, location: &(i32, i32), chunks: &mut Chunks<TextureType>) {
        let width = chunks.chunk_size.0;
        let height = chunks.chunk_size.1;
        let chunk = &mut chunks.get_chunk_mut(location).unwrap();
        for x in 0..width {
            for y in 0..height {
                let n = self.noise.get([location.0 as f64 + x as f64, location.1 as f64 + y as f64]);
                chunk[x][y].set_passable(n > 0.1);
            }
        }
    }
}
