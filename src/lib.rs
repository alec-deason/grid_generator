use std::sync::{Mutex,};
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet,};

use parking_lot_core::{park, unpark_one, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};

pub mod sparse;
pub mod region_lock;

use std::hash::Hash;
use std::iter::{from_fn,};

pub trait Point: Hash+Eq+Sized+Clone {
    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool;
    fn contained(&self, r: &[Self; 2]) -> bool;
    fn chunk_index(&self, chunk_size: u32) -> (Self, usize);
    fn max_unrolled_index(chunk_size: u32) -> usize;
    fn chunks_in_region<'a>(r: &'a [Self; 2], chunk_size: u32) -> Vec<[Self; 2]>;
}

impl Point for [i32; 2] {
    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool {
        (a[0][0] >= other[0][0] && a[0][0] < other[1][0]) ||
        (a[1][0] >= other[0][0] && a[1][0] < other[1][0]) ||
        (a[0][1] >= other[0][1] && a[0][1] < other[1][1]) ||
        (a[1][1] >= other[0][1] && a[1][1] < other[1][1])
    }

    fn contained(&self, r: &[Self; 2]) -> bool {
        (self[0] >= r[0][0] && self[0] < r[1][0]) ||
        (self[1] >= r[0][1] && self[1] < r[1][1])
    }

    fn max_unrolled_index(chunk_size: u32) -> usize {
        chunk_size as usize * chunk_size as usize
    }

    fn chunk_index(&self, chunk_size: u32) -> (Self, usize) {
        let x = (self[0] / chunk_size as i32) * chunk_size as i32;
        let x_r = self[0] - x;
        let y = (self[1] / chunk_size as i32) * chunk_size as i32;
        let y_r = self[1] - y;
        ([x, y], y_r as usize * chunk_size as usize + x_r as usize)
    }

    fn chunks_in_region<'a>(r: &'a [Self; 2], chunk_size: u32) -> Vec<[Self; 2]> {
        let mut x = r[0][0];
        let mut y = r[0][1];

        //FIXME: I'd really rather just return the iterator but I'm not sure how to make the types
        //work
        from_fn(move || {
            if x < r[1][0] || y < r[1][1] {
                let p = Some([[x, y], [x+chunk_size as i32, y+chunk_size as i32]]);
                if x > r[1][0] {
                    x = r[0][0];
                    y += chunk_size as i32;
                } else {
                    x += chunk_size as i32;
                }
                p
            } else {
                None
            }
        }).collect()
    }
}
//#[cfg(feature = "noise_based_generators")]
//pub mod noise_based_generators;
//pub mod postprocessors;
//pub mod analysis;


/*
struct Locks<Point> {
    read: HashMap<usize, (Vec<[Point; 2]>, Vec<usize>)>,
    write: HashMap<usize, (Vec<[Point; 2]>, Vec<usize>)>,
    pending: HashSet<usize>,
    lock_id: usize,
}

struct Data<Point, Tile> {
    index: HashMap<Point, usize>,
    chunks: Vec<HashMap<Point, Tile>>,
    chunk_size: u32,
}

impl<P: Point, Tile: Default> Data<P, Tile> {
    fn assure_region(&mut self, r: &[P; 2]) -> Vec<[P; 2]> {
        let mut need_generation = vec![];
        for i in P::chunk_indexes(r, self.chunk_size) {
            if !self.index.contains_key(&i) {
                let chunk_rect = i.to_rect(self.chunk_size);
                let chunk:HashMap<P, Tile> = P::index(&chunk_rect).map(|p| (p, Tile::default())).collect();
                need_generation.push(chunk_rect);
                let new_idx = self.chunks.len();
                self.chunks.push(chunk);
                self.index.insert(i, new_idx);
            }
        }
        need_generation
    }

    pub fn get(&self, p: &P) -> Option<&Tile> {
        let i = p.to_chunk_index(self.chunk_size);
        if let Some(chunk) = self.index.get(&i).and_then(|i| self.chunks.get(*i)) {
            let i = p.diff(&i);
            Some(&chunk[&i])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, p: &P) -> Option<&mut Tile> {
        let i = p.to_chunk_index(self.chunk_size);
        let chunk = self.index.get(&i).cloned().and_then(move |i| self.chunks.get_mut(i));
        if let Some(chunk) = chunk {
            let i = p.diff(&i);
            chunk.get_mut(&i)
        } else {
            None
        }
    }
}

pub struct Map<Point, Tile> {
    generators: Vec<Box<dyn Generator<Point, Tile>>>,

    locks: Mutex<Locks<Point>>,
    data_lock: Mutex<()>,
    data: UnsafeCell<Data<Point, Tile>>,
}

impl<P: Point, Tile: Default> Map<P, Tile> {
    pub fn new(generators: Vec<Box<dyn Generator<P, Tile>>>, chunk_size: u32) -> Self {
        Self {
            generators,

            locks: Mutex::new(Locks {
                read: HashMap::new(),
                write: HashMap::new(),
                pending: HashSet::new(),
                lock_id: 0,
            }),
            data_lock: Mutex::new(()),
            data: UnsafeCell::new(Data {
                index: HashMap::new(),
                chunks: Vec::new(),
                chunk_size,
            }),
        }
    }

    fn lock_region(&self, regions: &[[P; 2]], is_write: bool) -> usize{
        let mut conflict = true;
        let mut key = 0;
        let mut need_generation = vec![];
        while conflict {
            {
                let mut locks = self.locks.lock().unwrap();
                if locks.read.is_empty() && locks.write.is_empty() && locks.pending.is_empty() {
                    locks.lock_id = 0;
                }
                key = locks.lock_id;
                locks.lock_id += 1;
                'outer: for (_, (lock_regions, queue)) in &mut locks.write {
                    for region in regions {
                        for lock_region in lock_regions.iter() {
                            if Point::overlap_rect(region, lock_region) {
                                conflict = true;
                                queue.push(key);
                                break 'outer;
                            }
                        }
                    }
                }
                if is_write {
                    'outer2: for (_, (lock_regions, queue)) in &mut locks.read {
                        for region in regions {
                            for lock_region in lock_regions.iter() {
                                if Point::overlap_rect(region, lock_region) {
                                    conflict = true;
                                    queue.push(key);
                                    break 'outer2;
                                }
                            }
                        }
                    }
                }
                if !conflict {
                    {
                        let _data_lock = self.data_lock.lock();
                        let data = unsafe { &mut *self.data.get() };
                        for region in regions {
                            need_generation.extend(data.assure_region(region));
                        }
                    }
                    if is_write {
                        locks.write.insert(key, (regions.to_vec(), vec![]));
                    } else {
                        locks.read.insert(key, (regions.to_vec(), vec![]));
                    }
                } else {
                    locks.pending.insert(key);
                    unsafe {
                        park(
                            key,
                            || true,
                            || {},
                            |_,_| {},
                            DEFAULT_PARK_TOKEN,
                            None,
                        );
                    }
                }
            }
        }

        for region in need_generation {
            self.generate_region(WriteGuard
        }

        key
    }

    fn read_region(&self, regions: &[[P; 2]]) -> ReadGuard<'_, P, Tile> {
        ReadGuard(Guard {
            lock_id: self.lock_region(regions, false),
            region: regions.to_vec(),
            owner: self,
        })
    }

    fn write_region(&self, regions: &[[P; 2]]) -> WriteGuard<'_, P, Tile> {
        WriteGuard(Guard {
            lock_id: self.lock_region(regions, false),
            region: regions.to_vec(),
            owner: self,
        })
    }

    fn unlock_region(&self, lock_id: usize, is_write: bool) {
        let mut locks = self.locks.lock().unwrap();
        let queue;
        if is_write {
            queue = locks.write.remove(&lock_id).unwrap().1;
        } else {
            queue = locks.read.remove(&lock_id).unwrap().1;
        }
        for other_lock in queue {
            unsafe {
                unpark_one(
                    other_lock,
                    |_| DEFAULT_UNPARK_TOKEN,
                );
            }
        }
    }
}

struct Guard<'a, Point, Tile> {
    lock_id: usize,
    region: Vec<[Point; 2]>,
    owner: &'a Map<Point, Tile>,
}

impl<'a, P: Point, Tile: Default> Guard<'a, P, Tile> {
    pub fn get(&self, p: &P) -> Option<&Tile> {
        if self.region.iter().any(|r| p.in_region(r)) {
            let data = unsafe { &mut *self.owner.data.get() };
            let r = data.get(p);
            if r.is_none() {
                panic!("Guard created for uninitialized region");
            }
            r
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, p: &P) -> Option<&'a mut Tile> {
        if self.region.iter().any(|r| p.in_region(r)) {
            let data = unsafe { &mut *self.owner.data.get() };
            let r = data.get_mut(p);
            if r.is_none() {
                panic!("Guard created for uninitialized region");
            }
            r
        } else {
            None
        }
    }
}

pub struct ReadGuard<'a, P, Tile>(Guard<'a, P, Tile>) where P: Point+Eq + Clone + std::hash::Hash, Tile: Default;
pub struct WriteGuard<'a, P, Tile>(Guard<'a, P, Tile>) where P: Point+Eq + Clone + std::hash::Hash, Tile: Default;

pub struct TileEnumerator<'a, P, Tile> where P: Point{
    guard: &'a Guard<'a, P, Tile>,
    //TODO: Get rid of the heap alocation?
    points:  Box<dyn Iterator<Item=P>>,
}

//FIXME: I don't fully understand why I need the 'static bound here
impl<'a, P: 'static + Point + Clone + Eq + std::hash::Hash, Tile: Default> Iterator for TileEnumerator<'a, P, Tile> {
    type Item = (P, &'a Tile);
    fn next(&mut self) -> Option<Self::Item> {
        self.points.next().and_then(|p| Some((p, self.guard.get(&p).unwrap())))
    }
}

pub struct TileEnumeratorMut<'a, P, Tile> where P: Point {
    guard: &'a mut Guard<'a, P, Tile>,
    //TODO: Get rid of the heap alocation?
    points:  Box<dyn Iterator<Item=P>>,
}

impl<'a, P: 'static + Point, Tile: Default> Iterator for TileEnumeratorMut<'a, P, Tile> {
    type Item = (P, &'a mut Tile);
    fn next(&mut self) -> Option<Self::Item> {
        self.points.next().and_then(|p| Some((p, self.guard.get_mut(&p).unwrap())))
    }
}

impl<'a, P: 'static+Point, Tile: Default> ReadGuard<'a, P, Tile> {
    pub fn get(&self, p: &P) -> Option<&Tile> {
        self.0.get(p)
    }

    pub fn enumerate(&self) -> TileEnumerator<'_, P, Tile> {
        TileEnumerator {
            guard: &self.0,
            points: Box::new(self.0.region.clone().into_iter().map(|r| P::index(&r)).flatten()),
        }
    }
}

impl<'a, P: 'static+Point, Tile:Default> WriteGuard<'a, P, Tile> {
    pub fn get(&self, p: &P) -> Option<&Tile> {
        self.0.get(p)
    }

    pub fn get_mut(&mut self, p: &P) -> Option<&'a mut Tile> {
        self.0.get_mut(p)
    }

    pub fn enumerate(&self) -> TileEnumerator<'_, P, Tile> {
        let r = self.0.region.clone();
        TileEnumerator {
            guard: &self.0,
            points: Box::new(r.into_iter().map(|r| P::index(&r)).flatten()),
        }
    }

    pub fn enumerate_mut(&'a mut self) -> TileEnumeratorMut<'a, P, Tile> {
        let r = self.0.region.clone();
        TileEnumeratorMut {
            guard: &mut self.0,
            points: Box::new(r.into_iter().map(|r| P::index(&r)).flatten()),
        }
    }
}

impl<P: Point, Tile: Default> Drop for ReadGuard<'_, P, Tile> {
    fn drop(&mut self) {
        self.0.owner.unlock_region(self.0.lock_id, false);
    }
}

impl<P: Point, Tile: Default> Drop for WriteGuard<'_, P, Tile> {
    fn drop(&mut self) {
        self.0.owner.unlock_region(self.0.lock_id, true);
    }
}

pub trait Point: Sized+Copy+Eq+std::hash::Hash {
    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool;
    fn chunk_indexes(r: &[Self; 2], chunk_size: u32) -> Vec<Self>;
    fn to_rect(&self, size: u32) -> [Self; 2];
    fn diff(&self, other: &Self) -> Self;
    fn index(r: &[Self; 2]) -> Box<dyn Iterator<Item=Self>>;
    fn to_chunk_index(&self, chunk_size: u32) -> Self;
    fn from_chunk_index(&self, chunk_size: u32) -> Self;
    fn in_region(&self, r: &[Self; 2]) -> bool;
    fn neighboors(&self) -> Vec<Self>;
}

impl Point for [i32; 2] {
    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool {
        (a[0][0] >= other[0][0] && a[0][0] < other[1][0]) ||
        (a[1][0] >= other[0][0] && a[1][0] < other[1][0]) ||
        (a[0][1] >= other[0][1] && a[0][1] < other[1][1]) ||
        (a[1][1] >= other[0][1] && a[1][1] < other[1][1])
    }

    fn chunk_indexes(r: &[Self; 2], chunk_size: u32) -> Vec<Self> {
        let i = r[0].to_chunk_index(chunk_size);
        let mut x = i[0];
        let mut y = i[1];
        let mut result = vec![];
        while x < r[1][0] || y < r[1][1] {
            result.push([x, y]);
            x += chunk_size as i32;
            y += chunk_size as i32;
        }
        result
    }

    fn diff(&self, other: &Self) -> Self {
        [self[0]-other[0], self[1]-other[1]]
    }

    fn to_rect(&self, size: u32) -> [Self; 2] {
        [*self, [self[0]+size as i32, self[1]+size as i32]]
    }

    fn index(r: &[Self; 2]) -> Box<dyn Iterator<Item=Self>> {
        let r = *r;
        Box::new((r[0][0]..r[1][0]).map(move |x| (r[0][1]..r[1][1]).map(move |y| [x, y])).flatten())
    }

    fn to_chunk_index(&self, chunk_size: u32) -> Self {
        [self[0]/chunk_size as i32, self[1]/chunk_size as i32]
    }

    fn from_chunk_index(&self, chunk_size: u32) -> Self {
        [self[0]*chunk_size as i32, self[1]*chunk_size as i32]
    }

    fn in_region(&self, r: &[Self; 2]) -> bool {
        (self[0] >= r[0][0] && self[0] < r[1][0]) ||
        (self[1] >= r[0][1] && self[1] < r[1][1])
    }

    fn neighboors(&self) -> Vec<Self> {
        [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().map(|(dx, dy)| [self[0]+dx, self[1]+dy]).collect()
    }
}


pub trait Generator<P, Tile>: Send+Sync where P: Point+Clone+Eq+std::hash::Hash, Tile: Default {
    fn new_chunk(&self, chunk: &WriteGuard<'_, P, Tile>, umbra: &WriteGuard<'_, P, Tile>);
}

pub struct GeneratorSequence<P, Tile> {
    generators: Vec<Box<dyn Generator<P, Tile>>>,
}

impl<P, Tile> GeneratorSequence<P, Tile> {
    pub fn new(generators: Vec<Box<dyn Generator<P, Tile>>>) -> Self {
        Self {
            generators,
        }
    }
}

impl<P: Point + Clone + Eq + std::hash::Hash, Tile: Default> Generator<P, Tile> for GeneratorSequence<P, Tile> {
    fn new_chunk(&self, chunk: &WriteGuard<'_, P, Tile>, umbra: &WriteGuard<'_, P, Tile>) {
        for g in &self.generators {
            g.new_chunk(chunk, umbra);
        }
    }
}
*/
