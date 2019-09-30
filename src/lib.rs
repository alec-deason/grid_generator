use std::sync::Mutex;
use std::collections::HashSet;

use log::debug;

use chashmap::CHashMap;

use crate::{
    point::Point,
    region_lock::{Lock as RegionLock, Guard},
};

pub mod sparse;
pub mod region_lock;
pub mod generator;
pub mod point;


#[cfg(feature = "noise_based_generators")]
pub mod noise_based_generators;
//pub mod postprocessors
pub mod analysis;

struct Lock<P, T> {
    generated: HashSet<[P; 2]>,
    generators: Vec<Box<dyn generator::Generator<P, T>>>,
    dirty_chunks: Vec<[P; 2]>,
}

pub struct Map<P, T> {
    lock: Mutex<Lock<P, T>>,

    chunk_size: u32,

    region_lock: RegionLock<P>,
    map: CHashMap<P, T>,
}

impl<P: Point, T: Default> Map<P, T> {
    pub fn new(generators: Vec<Box<dyn generator::Generator<P, T>>>, chunk_size: u32) -> Self {
        Self {
            lock: Mutex::new(Lock {
                generated: HashSet::new(),
                generators,
                dirty_chunks: vec![],
            }),

            chunk_size,

            region_lock: RegionLock::new(),
            map: CHashMap::new(),
        }
    }

    pub fn maybe_generate(&self, r: &[P; 2]) {
        let mut lock = self.lock.lock().unwrap();
        let chunks:HashSet<[P; 2]> = P::chunks_in_region(r, self.chunk_size).into_iter().collect();
        let to_generate:HashSet<[P; 2]> = chunks.difference(&lock.generated).cloned().collect();
        lock.dirty_chunks.extend(to_generate.iter().cloned());
        for chunk in &to_generate {
            let umbra = P::expand(chunk, 1);
            let region_lock = self.region_lock.write_region(&[umbra.clone()]);

            for p in P::points_in_region(chunk) {
                self.map.insert_new(p, T::default());
            }

            let mut writer = WriteGuard {
                data: &self.map,
                region_lock,
                region: chunk.clone(),
            };

            for generator in &mut lock.generators {
                generator.generate(&mut writer, chunk, &umbra);
            }
        }
        lock.generated.extend(to_generate);
    }

    pub fn drain_dirty_regions(&self) -> Vec<[P; 2]> {
        let mut lock = self.lock.lock().unwrap();
        lock.dirty_chunks.drain(..).collect()
    }

    pub fn get(&self, p: &P) -> TileReadGuard<'_, P, T> {
        let r = p.to_cube(1);
        let lock = self.region_lock.read_region(&[r]);
        TileReadGuard {
            data: self.map.get(p).unwrap(),
            region_lock: lock,
        }
    }

    pub fn get_mut(&self, p: &P) -> TileWriteGuard<'_, P, T> {
        let r = p.to_cube(1);
        let lock = self.region_lock.write_region(&[r]);
        TileWriteGuard {
            data: self.map.get_mut(p).unwrap(),
            region_lock: lock,
        }
    }

    pub fn region(&self, r: &[P; 2]) -> ReadGuard<'_, P, T> {
        let lock = self.region_lock.read_region(&[r.clone()]);
        ReadGuard {
            data: &self.map,
            region_lock: lock,
            region: r.clone(),
        }
    }

    pub fn region_mut(&self, r: &[P; 2]) -> WriteGuard<'_, P, T> {
        let lock = self.region_lock.read_region(&[r.clone()]);
        WriteGuard {
            data: &self.map,
            region_lock: lock,
            region: r.clone(),
        }
    }
}

pub type LightTileReadGuard<'a, P, T> = chashmap::ReadGuard<'a, P, T>;
pub type LightTileWriteGuard<'a, P, T> = chashmap::WriteGuard<'a, P, T>;

pub struct TileReadGuard<'a, P, T> where P: Point {
    data: chashmap::ReadGuard<'a, P, T>,
    #[allow(dead_code)]
    region_lock: Guard<'a, P>,
}

impl<'a, P: Point, T> std::ops::Deref for TileReadGuard<'a, P, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.data
    }
}

pub struct TileWriteGuard<'a, P, T> where P: Point {
    data: chashmap::WriteGuard<'a, P, T>,
    #[allow(dead_code)]
    region_lock: Guard<'a, P>,
}

impl<'a, P: Point, T> std::ops::Deref for TileWriteGuard<'a, P, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.data
    }
}

impl<'a, P: Point, T> std::ops::DerefMut for TileWriteGuard<'a, P, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

pub struct ReadGuard<'a, P, T> where P: Point {
    data: &'a CHashMap<P, T>,
    #[allow(dead_code)] // Never used because it's just here to hold the inner lock open while this object is in scope
    region_lock: Guard<'a, P>,
    region: [P; 2],
}

impl<'a, P: Point, T> ReadGuard<'a, P, T> {
    pub fn get(&self, p: &P) -> Result<LightTileReadGuard<'a, P, T>, ()> {
        if p.contained(&self.region) {
            Ok(self.data.get(p).unwrap())
        } else {
            Err(())
        }
    }
}

pub struct WriteGuard<'a, P, T> where P: Point {
    data: &'a CHashMap<P, T>,
    #[allow(dead_code)] // Never used because it's just here to hold the inner lock open while this object is in scope
    region_lock: Guard<'a, P>,
    region: [P; 2],
}

impl<'a, P: Point, T> WriteGuard<'a, P, T> {
    pub fn get(&self, p: &P) -> Result<LightTileReadGuard<'a, P, T>, ()> {
        if p.contained(&self.region) {
            Ok(self.data.get(p).unwrap())
        } else {
            Err(())
        }
    }

    pub fn get_mut(&mut self, p: &P) -> Result<LightTileWriteGuard<'a, P, T>, ()> {
        if p.contained(&self.region) {
            Ok(self.data.get_mut(p).unwrap())
        } else {
            Err(())
        }
    }
}
