use std::sync::{
    Mutex, Convar, Arc, Weak,
    atomic::{AtomicUsize, Ordering},
};
use std::cell::{UnsafeCell, RefCell};
use std::collections::{HashMap, HashSet};

use parking_lot_core::{park, unpark, DEFAULT_PARK_TOKEN};

#[cfg(feature = "noise_based_generators")]
pub mod noise_based_generators;
pub mod postprocessors;
pub mod analysis;

pub type Point = (i32, i32, i32);

pub struct Chunk<TileType> {
    pub tiles: Vec<Vec<Vec<TileType>>>,
}

pub struct Chunks<TileType> {
    pub chunks: HashMap<Point, Chunk<TileType>>,
    pub chunk_size: (usize, usize, usize),
    pub dirty_chunks: Vec<Point>,
}

impl<TileType> Chunks<TileType> {
    fn decompose_location(&self, location: &Point) -> (Point, Point) {
        let chunk_loc = (
            (location.0 as f32 / self.chunk_size.0 as f32).floor() as i32,
            (location.1 as f32 / self.chunk_size.1 as f32).floor() as i32,
            (location.2 as f32 / self.chunk_size.2 as f32).floor() as i32,
        );
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
        let iz = if chunk_loc.2 < 0 {
            (self.chunk_size.2 as i32 + location.2 % self.chunk_size.2 as i32) - 1
        } else {
            location.2 % self.chunk_size.2 as i32
        };

        let inner_loc = (ix, iy, iz);
        (chunk_loc, inner_loc)
    }

    pub fn get_tile(&self, location: &Point) -> Option<&TileType> {
        let (_, inner_loc) = self.decompose_location(location);
        self.get_chunk(location).and_then(|c| c.get(inner_loc.0 as usize)).and_then(|c| c.get(inner_loc.1 as usize)).and_then(|c| c.get(inner_loc.2 as usize))
    }

    pub fn get_tile_mut(&mut self, location: &Point) -> Option<&mut TileType> {
        let (_, inner_loc) = self.decompose_location(location);
        self.get_chunk_mut(location).and_then(|c| c.get_mut(inner_loc.0 as usize)).and_then(|c| c.get_mut(inner_loc.1 as usize)).and_then(|c| c.get_mut(inner_loc.2 as usize))
    }

    pub fn get_chunk(&self, location: &Point) -> Option<&Vec<Vec<Vec<TileType>>>> {
        let (chunk_loc, _) = self.decompose_location(location);
        self.chunks.get(&chunk_loc).and_then(|c| Some(&c.tiles))
    }

    pub fn get_chunk_mut(&mut self, location: &Point) -> Option<&mut Vec<Vec<Vec<TileType>>>> {
        let (chunk_loc, _) = self.decompose_location(location);
        self.chunks.get_mut(&chunk_loc).and_then(|c| Some(&mut c.tiles))
    }

    pub fn iter_tiles(&self) -> impl Iterator<Item=&TileType> {
        self.chunks.values().map(|c| {
            c.tiles.iter().map(|cc| {
                cc.iter().map(|ccc| {
                    ccc.iter()
                }).flatten()
            }).flatten()
        }).flatten()
    }

    //FIXME: How do you make a mut version of this?
    pub fn enumerate_tiles(&self) -> impl Iterator<Item=(Point, &TileType)> {
        let width = self.chunk_size.0 as i32;
        let height = self.chunk_size.1 as i32;
        self.chunks.iter().map(move |((cx, cy, cz), c)| {
            let cx = *cx * width;
            let cy = *cy * height;
            c.tiles.iter().enumerate().map(move |(tx, r)| {
                r.iter().enumerate().map(move |(ty, c)|
                    c.iter().enumerate().map(move |(tz, t)|
                        ((cx+tx as i32, cy+ty as i32, cz+tz as i32), t)
                    )
                ).flatten()
            }).flatten()
        }).flatten()
    }

    pub fn iter_tiles_mut(&mut self) -> impl Iterator<Item=&mut TileType> {
        self.chunks.values_mut().map(|c| {
            c.tiles.iter_mut().map(|cc| {
                cc.iter_mut().map(|ccc| {
                    ccc.iter_mut()
                }).flatten()
            }).flatten()
        }).flatten()
    }
}

type LockMap = RefCell<HashMap<usize, ([[i32; 3]; 2], Vec<usize>)>>;

pub struct RegionLock<Tile> {
    data: UnsafeCell<HashMap<[i32; 3], Tile>>,

    next_key: AtomicUsize,
    read: LockMap,
    write: LockMap,
}

impl<Tile: Default> RegionLock<Tile> {
    fn get_tile(&self, p: &[i32; 3]) -> Option<&Tile> {
        let container = unsafe { &*self.data.get() };
        container.get(p)
    }

    fn get_tile_mut(&self, p: &[i32; 3]) -> &mut Tile {
        let container = unsafe { &mut *self.data.get() };
        container.entry(*p).or_insert_with(Tile::default)
    }

    pub fn read_region(&self, region: [[i32; 3]; 2]) -> ReadGuard<Tile> {
        let key = self.next_key.fetch_add(1, Ordering::Relaxed);
        unsafe {
            park(
                key,
                || {
                    self.maybe_lock_region(key, region, false)
                },
                || {},
                |_,_| {},
                DEFAULT_PARK_TOKEN,
                None,
            );
            ReadGuard {
                owner: self,
                key: key,
                region,
            }
        }
    }

    /*
    pub fn try_write_region(&mut self, region: [[i32; 3]; 2]) -> Option<WriteGuard<Tile>> {

        let mut locks = self.locks.lock().unwrap();
        if !locks.write.values().any(|other| overlap(&region, other)) &&
           !locks.read.values().any(|other| overlap(&region, other)) {
            locks.next_lock_id += 1;
            let id = locks.next_lock_id;
            locks.write.insert(id, region);

            Some(WriteGuard {
                owner: self,
                id,
                region,
            })
        } else {
            None
        }
    }
    */

    fn has_conflict(&self, key: usize, region[[i32; 3]; 2], is_write: bool, insert: bool) -> bool {
        let write = self.write.borrow_mut();
        let mut conflict = false;
        for (other_region, queue) in write.values_mut() {
            if overlap(&region, other_region) {
                if insert {
                    queue.push(key);
                }
                conflict = true;
            }
        }
        let read = self.write.borrow_mut();
        if is_write {
            for (other_region, queue) in read.values_mut() {
                if overlap(&region, other_region) {
                    if insert {
                        queue.push(key);
                    }
                    conflict = true;
                }
            }
            write.insert(key, (region, vec![]));
        } else {
            read.insert(key, (region, vec![]));
        }
        conflict
    }

    fn maybe_lock_region(&self, key: usize, region: [[i32; 3]; 2], is_write: bool) -> bool {
        let conflict = self.has_conflict(key, region, is_write);
        if is_write {
            let write = self.write.borrow_mut();
            write.insert(key, (region, vec![]));
        } else {
            let read = self.write.borrow_mut();
            read.insert(key, (region, vec![]));
        }
        conflict
    }

    fn unlock_region(&self, key: usize, is_write: bool) {
        unpark(
            key,
            |_| {
                let (_, others) = if is_write {
                    self.write.borrow_mut().remove(&key)
                } else {
                    self.read.borrow_mut().remove(&key)
                }.unwrap();
                for other in others {
                    let region;
                    let is_write;
                    if self.write.borrow().contains_key(&other) {
                        region = self.write.borrow().get(&other).unwrap().0;
                        is_write = true;
                    } else {
                        region = self.read.borrow().get(&other).unwrap().0;
                        is_write = false;
                    }
                }
            },
        );
    }
}

trait Guard<Tile> {
    fn owner(&self) -> &'_ RegionLock<Tile>;
    fn region(&self) -> &[[i32; 3]; 2];
}

fn within(p: &[i32; 3], region: &[[i32; 3]; 2]) -> bool {
    (p[0] >= region[0][0] && p[0] < region[1][0]) ||
    (p[1] >= region[0][1] && p[1] < region[1][1]) ||
    (p[2] >= region[0][2] && p[2] < region[1][2])
}

trait RegionReader<Tile>: Guard<Tile>
    where Tile: Default {
    fn get_tile(&self, p: &[i32; 3]) -> Result<Option<&Tile>, ()> {
        if within(p, self.region()) {
            Ok(self.owner().get_tile(p))
        } else {
            Err(())
        }
    }
}

trait RegionWriter<Tile>: Guard<Tile> 
    where Tile: Default {
    fn get_tile_mut(&self, p: &[i32; 3]) -> Result<&mut Tile, ()> {
        if within(p, self.region()) {
            Ok(self.owner().get_tile_mut(p))
        } else {
            Err(())
        }
    }
}

pub struct ReadGuard<'a, Tile> {
    owner: &'a RegionLock<Tile>,
    key: usize,
    region: [[i32; 3]; 2],
}
impl<'a, Tile: Default> RegionReader<Tile> for ReadGuard<'a, Tile> {}

impl<'a, Tile> Guard<Tile> for ReadGuard<'a, Tile> {
    fn owner(&self) -> &RegionLock<Tile> {
        self.owner
    }
    fn region(&self) -> &[[i32; 3]; 2] {
        &self.region
    }
}

impl<'a, Tile> Drop for ReadGuard<'a, Tile> {
    fn drop(&mut self) {
        self.owner.locks.lock().unwrap().read.remove(&self.id);
    }
}

impl<'a, Tile: Default> RegionReader<Tile> for WriteGuard<'a, Tile> {}
impl<'a, Tile: Default> RegionWriter<Tile> for WriteGuard<'a, Tile> {}

pub struct WriteGuard<'a, Tile> {
    owner: &'a RegionLock<Tile>,
    id: usize,
    region: [[i32; 3]; 2],
}

impl<'a, Tile> Drop for WriteGuard<'a, Tile> {
    fn drop(&mut self) {
        self.owner.locks.lock().unwrap().write.remove(&self.id);
    }
}

impl<'a, Tile> Guard<Tile> for WriteGuard<'a, Tile> {
    fn owner(&self) -> &RegionLock<Tile> {
        self.owner
    }
    fn region(&self) -> &[[i32; 3]; 2] {
        &self.region
    }
}

fn overlap(a: &[[i32; 3]; 2], b: &[[i32; 3]; 2]) -> bool {
    (a[0][0] >= b[0][0] && a[0][0] < b[1][0]) ||
    (a[1][0] >= b[0][0] && a[1][0] < b[1][0]) ||
    (a[0][1] >= b[0][1] && a[0][1] < b[1][1]) ||
    (a[1][1] >= b[0][1] && a[1][1] < b[1][1]) ||
    (a[0][2] >= b[0][2] && a[0][2] < b[1][2]) ||
    (a[1][2] >= b[0][2] && a[1][2] < b[1][2])
}

pub struct Map<TileType> {
    generators: Vec<Box<dyn Generator<TileType>>>,
    pub chunks: Chunks<TileType>,
}


impl<TileType: Default> Map<TileType> {
    pub fn new(generators: Vec<Box<dyn Generator<TileType>>>, chunk_size: (usize, usize, usize)) -> Self {
        Self {
            generators,
            chunks: Chunks {
                chunks: HashMap::new(),
                chunk_size,
                dirty_chunks: vec![],
            }
        }
    }

    pub fn get_tile(&self, location: &Point) -> Option<&TileType> {
        self.chunks.get_tile(location)
    }

    pub fn get_tile_mut(&mut self, location: &Point) -> Option<&mut TileType> {
        self.chunks.get_tile_mut(location)
    }

    pub fn get_or_generate_tile(&mut self, location: &Point) -> &mut TileType {
        self.maybe_generate_chunk(location);
        self.get_tile_mut(location).unwrap()
    }

    pub fn get_chunk(&self, location: &Point) -> Option<&Vec<Vec<Vec<TileType>>>> {
        self.chunks.get_chunk(location)
    }

    pub fn get_chunk_mut(&mut self, location: &Point) -> Option<&mut Vec<Vec<Vec<TileType>>>> {
        self.chunks.get_chunk_mut(location)
    }

    pub fn maybe_generate_chunk(&mut self, location: &Point) -> bool {
        if let Some(_) = self.chunks.get_chunk(location) {
            return false;
        }
        let width = self.chunks.chunk_size.0 as i32;
        let height = self.chunks.chunk_size.1 as i32;
        let depth = self.chunks.chunk_size.2 as i32;
        let (chunk_loc, _) = self.chunks.decompose_location(location);
        self.chunks.dirty_chunks.push((
                chunk_loc.0*self.chunks.chunk_size.0 as i32,
                chunk_loc.1*self.chunks.chunk_size.1 as i32,
                chunk_loc.2*self.chunks.chunk_size.2 as i32,
        ));
        let chunks = &mut self.chunks;
        chunks.chunks.insert(chunk_loc, Chunk {
            tiles: (0..chunks.chunk_size.0).map(|_| {
                (0..chunks.chunk_size.1).map(|_| {
                    (0..chunks.chunk_size.2).map(|_| {
                        TileType::default()
                    }).collect()
                }).collect()
            }).collect(),
        });
        for generator in &mut self.generators {
            generator.new_chunk(&(chunk_loc.0 * width, chunk_loc.1 * height, chunk_loc.2 * depth), chunks);
        }
        true
    }

    pub fn chunks_in_region(&self, region: [[i32; 3]; 2]) -> HashSet<Point> {
        let mut result = HashSet::new();
        for x in (region[0][0]..region[1][0]+self.chunks.chunk_size.0 as i32).step_by(self.chunks.chunk_size.0) {
            for y in (region[0][1]..region[1][1]+self.chunks.chunk_size.1 as i32).step_by(self.chunks.chunk_size.1) {
                for z in (region[0][2]..region[1][2]+self.chunks.chunk_size.2 as i32).step_by(self.chunks.chunk_size.2) {
                    let (chunk_loc, _) = self.chunks.decompose_location(&(x, y, z));
                    result.insert((
                            chunk_loc.0*self.chunks.chunk_size.0 as i32,
                            chunk_loc.1*self.chunks.chunk_size.1 as i32,
                            chunk_loc.2*self.chunks.chunk_size.2 as i32,
                    ));
                }
            }
        }
        result
    }
}

pub trait Generator<TileType>: Send+Sync {
    fn new_chunk(&mut self, location: &Point, chunks: &mut Chunks<TileType>);
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
    fn new_chunk(&mut self, location: &Point, chunks: &mut Chunks<TileType>) {
        for g in &mut self.generators {
            g.new_chunk(location, chunks);
        }
    }
}
