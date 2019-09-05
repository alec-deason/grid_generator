use std::collections::HashMap;

#[cfg(feature = "noise_based_generators")]
pub mod noise_based_generators;
pub mod postprocessors;

pub struct Chunk<TileType> {
    pub layers: Vec<Vec<Vec<TileType>>>,
}

pub struct Map<TileType> {
    generators: Vec<Box<dyn Generator<TileType>>>,
    chunks: HashMap<(i32, i32), Chunk<TileType>>,
    chunk_size: (usize, usize),
    layer_count: usize,
}

impl<TileType: Default> Map<TileType> {
    pub fn new(generators: Vec<Box<dyn Generator<TileType>>>, chunk_size: (usize, usize), layer_count: usize) -> Self {
        Self {
            generators,
            chunks: HashMap::new(),
            chunk_size,
            layer_count,
        }
    }

    pub fn get_tile(&self, location: &(i32, i32), layer: usize) -> Option<&TileType> {
        let chunk_loc = (location.0 / self.chunk_size.0 as i32, location.1 / self.chunk_size.1 as i32);
        let inner_loc = (location.0 - chunk_loc.0, location.1 - chunk_loc.1);
        self.get_chunk(&chunk_loc, layer).and_then(|c| c.get(inner_loc.0 as usize)).and_then(|c| c.get(inner_loc.1 as usize))
    }

    pub fn get_tile_mut(&mut self, location: &(i32, i32), layer: usize) -> Option<&mut TileType> {
        let chunk_loc = (location.0 / self.chunk_size.0 as i32, location.1 / self.chunk_size.1 as i32);
        let inner_loc = (location.0 - chunk_loc.0*self.chunk_size.0 as i32, location.1 - chunk_loc.1*self.chunk_size.1 as i32);
        self.get_chunk_mut(&chunk_loc, layer).and_then(|c| c.get_mut(inner_loc.0 as usize)).and_then(|c| c.get_mut(inner_loc.1 as usize))
    }

    pub fn get_or_generate_tile(&mut self, location: &(i32, i32), layer: usize) -> &mut TileType {
        let chunk_loc = (location.0 / self.chunk_size.0 as i32, location.1 / self.chunk_size.1 as i32);

        self.maybe_generate_chunk(&chunk_loc);
        self.get_tile_mut(location, layer).unwrap()
    }

    pub fn get_chunk(&self, location: &(i32, i32), layer: usize) -> Option<&Vec<Vec<TileType>>> {
        self.chunks.get(location).and_then(|c| c.layers.get(layer))
    }

    pub fn get_chunk_mut(&mut self, location: &(i32, i32), layer: usize) -> Option<&mut Vec<Vec<TileType>>> {
        self.chunks.get_mut(location).and_then(|c| c.layers.get_mut(layer))
    }

    pub fn maybe_generate_chunk(&mut self, location: &(i32, i32)) -> bool {
        if self.chunks.contains_key(location) {
            return false;
        }

        let chunks = &self.chunks;
        let mut layers: Vec<Vec<Vec<TileType>>> = (0..self.layer_count).map(|_| (0..self.chunk_size.0).map(|_| (0..self.chunk_size.1).map(|_| TileType::default()).collect()).collect()).collect();
        for (i, generator) in self.generators.iter_mut().enumerate() {
            let (head, tail) = layers.split_at_mut(i);
            generator.new_chunk(location, chunks, head, &mut tail[0]);
        }
        let chunk = Chunk {
            layers,
        };
        self.chunks.insert(*location, chunk);
        true
    }
}

pub trait Generator<TileType> {
    fn new_chunk(&mut self, location: &(i32, i32), map: &HashMap<(i32, i32), Chunk<TileType>>, lower_layers: &[Vec<Vec<TileType>>], chunk: &mut [Vec<TileType>]);
}
