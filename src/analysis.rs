use std::collections::{HashSet, HashMap,};

use super::{
    generator::Generator, WriteGuard, point::Point,
};

pub trait Passable {
    fn is_passable(&self) -> bool;
    fn set_passable(&mut self, passable: bool);
}

pub trait Connected<Point> {
    fn get_edges(&self) -> &HashSet<Point>;
    fn get_edges_mut(&mut self) -> &mut HashSet<Point>;
}

pub struct Connectivity;

impl<P: Point, T: Connected<P> + Passable> Generator<P, T> for Connectivity {
    fn generate(&mut self, chunk: &mut WriteGuard<'_, P, T>, core_region: &[P; 2], _umbra: &[P; 2]) {
        let mut to_add = HashMap::new();
        for p in P::points_in_region(core_region) {
            let tile = chunk.get(&p).unwrap();
            if tile.is_passable() {
                for pp in p.neighboors() {
                    let other = chunk.get(&pp).unwrap();
                    if other.is_passable() {
                        to_add.entry(p.clone()).or_insert(HashSet::new()).insert(pp.clone());
                        to_add.entry(pp).or_insert(HashSet::new()).insert(p.clone());
                    }
                }
            }
        }
        for (p, edges) in to_add {
            chunk.get_mut(&p).unwrap().get_edges_mut().extend(edges);
        }
    }
}
