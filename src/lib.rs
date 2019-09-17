use std::collections::{HashMap, HashSet};

#[cfg(feature = "noise_based_generators")]
pub mod noise_based_generators;
pub mod postprocessors;
pub mod analysis;

pub struct Chunk<TileType> {
    pub tiles: Vec<Vec<TileType>>,
}

pub struct Chunks<TileType> {
    pub chunks: HashMap<(i32, i32), Chunk<TileType>>,
    pub chunk_size: (usize, usize),
    pub dirty_chunks: Vec<(i32, i32)>,
}

impl<TileType> Chunks<TileType> {
    fn decompose_location(&self, location: &(i32, i32)) -> ((i32, i32), (i32, i32)) {
        let chunk_loc = ((location.0 as f32 / self.chunk_size.0 as f32).floor() as i32, (location.1 as f32 / self.chunk_size.1 as f32).floor() as i32);
        let ix = if chunk_loc.0 < 0 {
            (self.chunk_size.0 as i32 + location.0 % self.chunk_size.0 as i32) - 1
        } else {
            location.0 % self.chunk_size.0 as i32
        };
        let iy = if chunk_loc.1 < 0 {
            (self.chunk_size.1 as i32 + location.1 % self.chunk_size.1 as i32) - 1
        } else {
            location.1 % self.chunk_size.1 as i32
        };

        let inner_loc = (ix, iy);
        (chunk_loc, inner_loc)
    }

    pub fn get_tile(&self, location: &(i32, i32)) -> Option<&TileType> {
        let (_, inner_loc) = self.decompose_location(location);
        self.get_chunk(location).and_then(|c| c.get(inner_loc.0 as usize)).and_then(|c| c.get(inner_loc.1 as usize))
    }

    pub fn get_tile_mut(&mut self, location: &(i32, i32)) -> Option<&mut TileType> {
        let (_, inner_loc) = self.decompose_location(location);
        self.get_chunk_mut(location).and_then(|c| c.get_mut(inner_loc.0 as usize)).and_then(|c| c.get_mut(inner_loc.1 as usize))
    }

    pub fn get_chunk(&self, location: &(i32, i32)) -> Option<&Vec<Vec<TileType>>> {
        let (chunk_loc, _) = self.decompose_location(location);
        self.chunks.get(&chunk_loc).and_then(|c| Some(&c.tiles))
    }

    pub fn get_chunk_mut(&mut self, location: &(i32, i32)) -> Option<&mut Vec<Vec<TileType>>> {
        let (chunk_loc, _) = self.decompose_location(location);
        self.chunks.get_mut(&chunk_loc).and_then(|c| Some(&mut c.tiles))
    }

    pub fn iter_tiles(&self) -> impl Iterator<Item=&TileType> {
        self.chunks.values().map(|c| c.tiles.iter().map(|r| r.iter()).flatten()).flatten()
    }

    //FIXME: How do you make a mut version of this?
    pub fn enumerate_tiles(&self) -> impl Iterator<Item=((i32, i32), &TileType)> {
        let width = self.chunk_size.0 as i32;
        let height = self.chunk_size.1 as i32;
        self.chunks.iter().map(move |((cx, cy), c)| {
            let cx = *cx * width;
            let cy = *cy * height;
            c.tiles.iter().enumerate().map(move |(tx, r)| {
                r.iter().enumerate().map(move |(ty, t)|
                    ((cx+tx as i32, cy+ty as i32), t)
                )
            }).flatten()
        }).flatten()
    }

    pub fn iter_tiles_mut(&mut self) -> impl Iterator<Item=&mut TileType> {
        self.chunks.values_mut().map(|c| c.tiles.iter_mut().map(|r| r.iter_mut()).flatten()).flatten()
    }
}

pub struct Map<TileType> {
    generators: Vec<Box<dyn Generator<TileType>>>,
    pub chunks: Chunks<TileType>,
}


impl<TileType: Default> Map<TileType> {
    pub fn new(generators: Vec<Box<dyn Generator<TileType>>>, chunk_size: (usize, usize)) -> Self {
        Self {
            generators,
            chunks: Chunks {
                chunks: HashMap::new(),
                chunk_size,
                dirty_chunks: vec![],
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
        self.maybe_generate_chunk(location);
        self.get_tile_mut(location).unwrap()
    }

    pub fn get_chunk(&self, location: &(i32, i32)) -> Option<&Vec<Vec<TileType>>> {
        self.chunks.get_chunk(location)
    }

    pub fn get_chunk_mut(&mut self, location: &(i32, i32)) -> Option<&mut Vec<Vec<TileType>>> {
        self.chunks.get_chunk_mut(location)
    }

    pub fn maybe_generate_chunk(&mut self, location: &(i32, i32)) -> bool {
        if let Some(_) = self.chunks.get_chunk(location) {
            return false;
        }
        let width = self.chunks.chunk_size.0 as i32;
        let height = self.chunks.chunk_size.1 as i32;
        let (chunk_loc, _) = self.chunks.decompose_location(location);
        self.chunks.dirty_chunks.push((chunk_loc.0*self.chunks.chunk_size.0 as i32, chunk_loc.1*self.chunks.chunk_size.1 as i32));
        let chunks = &mut self.chunks;
        chunks.chunks.insert(chunk_loc, Chunk {
            tiles: (0..chunks.chunk_size.0).map(|_| (0..chunks.chunk_size.1).map(|_| TileType::default()).collect()).collect(),
        });
        for generator in &mut self.generators {
            generator.new_chunk(&(chunk_loc.0 * width, chunk_loc.1 * height), chunks);
        }
        true
    }

    pub fn chunks_in_region(&self, region: [[i32; 2]; 2]) -> HashSet<(i32, i32)> {
        let mut result = HashSet::new();
        for x in (region[0][0]..region[1][0]+self.chunks.chunk_size.0 as i32).step_by(self.chunks.chunk_size.0) {
            for y in (region[0][1]..region[1][1]+self.chunks.chunk_size.1 as i32).step_by(self.chunks.chunk_size.1) {
                let (chunk_loc, _) = self.chunks.decompose_location(&(x, y));
                result.insert((chunk_loc.0*self.chunks.chunk_size.0 as i32, chunk_loc.1*self.chunks.chunk_size.1 as i32));
            }
        }
        result
    }
}

pub trait Generator<TileType>: Send+Sync {
    fn new_chunk(&mut self, location: &(i32, i32), chunks: &mut Chunks<TileType>);
}

pub struct GeneratorSequence<TileType> {
    generators: Vec<Box<dyn Generator<TileType>>>,
}

impl<TileType> GeneratorSequence<TileType> {
    pub fn new(generators: Vec<Box<dyn Generator<TileType>>>) -> Self {
        Self {
            generators,
        }
    }
}

impl<TileType> Generator<TileType> for GeneratorSequence<TileType> {
    fn new_chunk(&mut self, location: &(i32, i32), chunks: &mut Chunks<TileType>) {
        for g in &mut self.generators {
            g.new_chunk(location, chunks);
        }
    }
}
