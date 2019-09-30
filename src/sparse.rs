use std::collections::HashMap;

use crate::point::Point;

pub struct SparseMap<P, T> {
    index: HashMap<P, usize>,
    chunks: Vec<Vec<T>>,

    pub chunk_size: u32,
}

impl<P: Point, T: Default> SparseMap<P, T> {
    pub fn new(chunk_size: u32) -> Self {
        Self {
            index: HashMap::new(),
            chunks: vec![],

            chunk_size: chunk_size,
        }
    }

    pub fn empty_in_region(&self, r: &[P; 2]) -> Vec<[P; 2]> {
        P::chunks_in_region(r, self.chunk_size).into_iter().filter(|chunk_idx| {
            !self.index.contains_key(&chunk_idx[0])
        }).collect()
    }

    fn get(&self, p: &P) -> Option<&T> {
        let (c, p) = p.chunk_index(self.chunk_size);
        self.index.get(&c).and_then(|i| Some(&self.chunks[*i][p]))
    }

    fn get_mut(&mut self, p: &P) -> Option<&mut T> {
        let (c, p) = p.chunk_index(self.chunk_size);
        self.index.get(&c).cloned().and_then(move |i| Some(&mut self.chunks[i][p]))
    }

    fn set(&mut self, p: &P, t: T) {
        let (c, p) = p.chunk_index(self.chunk_size);
        if let Some(i) = self.index.get(&c) {
            self.chunks[*i][p] = t;
        } else {
            let i = self.chunks.len();
            self.chunks.push((0..P::max_unrolled_index(self.chunk_size)).map(|_| T::default()).collect());
            self.index.insert(c, i);
            self.chunks[i][p] = t;
        }
    }

    pub fn region(&self, r: &[P; 2]) -> ReadGuard<'_, P, T> {
        ReadGuard {
            owner: self,
            region: r.clone(),
        }
    }

    pub fn region_mut(&mut self, r: &[P; 2]) -> WriteGuard<'_, P, T> {
        WriteGuard {
            owner: self,
            region: r.clone(),
        }
    }
}

pub struct ReadGuard<'a, P, T> {
    owner: &'a SparseMap<P, T>,
    region: [P; 2],
}

pub struct WriteGuard<'a, P, T> {
    owner: &'a mut SparseMap<P, T>,
    region: [P; 2],
}

impl<'a, P: Point, T: Default> ReadGuard<'a, P, T> {
    pub fn get(&self, p: &P) -> Result<Option<&T>, ()> {
        if p.contained(&self.region) {
            Ok(self.owner.get(p))
        } else {
            Err(())
        }
    }
}

impl<'a, P: Point, T: Default> WriteGuard<'a, P, T> {
    pub fn get(&self, p: &P) -> Result<Option<&T>, ()> {
        if p.contained(&self.region) {
            Ok(self.owner.get(p))
        } else {
            Err(())
        }
    }

    pub fn get_mut(&mut self, p: &P) -> Result<Option<&mut T>, ()> {
        if p.contained(&self.region) {
            Ok(self.owner.get_mut(p))
        } else {
            Err(())
        }
    }

    pub fn set(&mut self, p: &P, t: T) -> Result<(), ()> {
        if p.contained(&self.region) {
            self.owner.set(p, t);
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug)]
    struct Tile {
        a: i32,
        b: i32,
    }

    #[test]
    fn write_points() {
        let mut map:SparseMap<[i32; 2], Tile> = SparseMap::new(10);
        let mut region = map.region_mut(&[[0, 0], [100, 100]]);
        region.set(&[50, 50], Tile { a: 42, b: 43 }).unwrap();
        eprintln!("{:?}", region.get(&[50, 50]).unwrap().unwrap());
        assert!(region.get(&[50, 50]).unwrap().unwrap().a == 42);
    }

    #[test]
    fn write_points_then_read() {
        let mut map:SparseMap<[i32; 2], Tile> = SparseMap::new(10);
        let mut region = map.region_mut(&[[0, 0], [100, 100]]);
        region.set(&[50, 50], Tile { a: 42, b: 43 }).unwrap();

        let region = map.region(&[[0, 0], [100, 100]]);
        let t = region.get(&[50, 50]).unwrap().unwrap();
        assert!(t.a == 42);
    }
}
