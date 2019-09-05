use std::collections::HashMap;

#[cfg(feature = "noise_based_generators")]
pub mod noise_based_generators;
pub mod postprocessors;
pub mod analysis;

pub struct Chunk<TileType> {
    pub tiles: Vec<Vec<TileType>>,
}

pub struct Chunks<TileType> {
    chunks: HashMap<(i32, i32), Chunk<TileType>>,
    chunk_size: (usize, usize),
}

impl<TileType> Chunks<TileType> {
    pub fn get_tile(&self, location: &(i32, i32)) -> Option<&TileType> {
        let chunk_loc = (location.0 / self.chunk_size.0 as i32, location.1 / self.chunk_size.1 as i32);
        let inner_loc = (location.0 - chunk_loc.0*self.chunk_size.0 as i32, location.1 - chunk_loc.1*self.chunk_size.1 as i32);
        self.get_chunk(&chunk_loc).and_then(|c| c.get(inner_loc.0 as usize)).and_then(|c| c.get(inner_loc.1 as usize))
    }

    pub fn get_tile_mut(&mut self, location: &(i32, i32)) -> Option<&mut TileType> {
        let chunk_loc = (location.0 / self.chunk_size.0 as i32, location.1 / self.chunk_size.1 as i32);
        let inner_loc = (location.0 - chunk_loc.0*self.chunk_size.0 as i32, location.1 - chunk_loc.1*self.chunk_size.1 as i32);
        self.get_chunk_mut(&chunk_loc).and_then(|c| c.get_mut(inner_loc.0 as usize)).and_then(|c| c.get_mut(inner_loc.1 as usize))
    }

    pub fn get_chunk(&self, location: &(i32, i32)) -> Option<&Vec<Vec<TileType>>> {
        self.chunks.get(location).and_then(|c| Some(&c.tiles))
    }

    pub fn get_chunk_mut(&mut self, location: &(i32, i32)) -> Option<&mut Vec<Vec<TileType>>> {
        self.chunks.get_mut(location).and_then(|c| Some(&mut c.tiles))
    }
}

pub struct Map<TileType> {
    generators: Vec<Box<dyn Generator<TileType>>>,
    chunks: Chunks<TileType>,
}


impl<TileType: Default> Map<TileType> {
    pub fn new(generators: Vec<Box<dyn Generator<TileType>>>, chunk_size: (usize, usize)) -> Self {
        Self {
            generators,
            chunks: Chunks {
                chunks: HashMap::new(),
                chunk_size,
            }
        }
    }

    pub fn get_tile(&self, location: &(i32, i32)) -> Option<&TileType> {
        self.chunks.get_tile(location)
    }

    pub fn get_tile_mut(&mut self, location: &(i32, i32)) -> Option<&mut TileType> {
        self.chunks.get_tile_mut(location)
    }

    pub fn get_or_generate_tile(&mut self, location: &(i32, i32)) -> &mut TileType {
        let chunk_loc = (location.0 / self.chunks.chunk_size.0 as i32, location.1 / self.chunks.chunk_size.1 as i32);

        self.maybe_generate_chunk(&chunk_loc);
        self.get_tile_mut(location).unwrap()
    }

    pub fn get_chunk(&self, location: &(i32, i32)) -> Option<&Vec<Vec<TileType>>> {
        self.chunks.get_chunk(location)
    }

    pub fn get_chunk_mut(&mut self, location: &(i32, i32)) -> Option<&mut Vec<Vec<TileType>>> {
        self.chunks.get_chunk_mut(location)
    }

    pub fn maybe_generate_chunk(&mut self, location: &(i32, i32)) -> bool {
        if self.chunks.chunks.contains_key(location) {
            return false;
        }

        let chunks = &mut self.chunks;
        chunks.chunks.insert(*location, Chunk {
            tiles: (0..chunks.chunk_size.0).map(|_| (0..chunks.chunk_size.1).map(|_| TileType::default()).collect()).collect(),
        });
        for generator in &mut self.generators {
            generator.new_chunk(location, chunks);
        }
        true
    }
}

pub trait Generator<TileType>: Send {
    fn new_chunk(&mut self, location: &(i32, i32), chunks: &mut Chunks<TileType>);
}
