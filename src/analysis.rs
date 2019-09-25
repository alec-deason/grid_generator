use std::collections::{HashSet, HashMap,};

use super::{
    Generator, WriteGuard, Point,
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

impl<P: 'static+Point, Tile: Connected<P> + Passable + Default> Generator<P, Tile> for Connectivity {
    fn new_chunk<'a>(&self, chunk: &'a mut WriteGuard<'a, P, Tile>, umbra: &'a mut WriteGuard<'a, P, Tile>) {
        let mut to_add = HashMap::new();
        let mut umbra_to_add = HashMap::new();
        for (p, tile) in chunk.enumerate() {
            if tile.is_passable() {
                for pp in p.neighboors() {
                    if let Some(other) = chunk.get(&pp) {
                        if other.is_passable() {
                            to_add.entry(p).or_insert(HashSet::new()).insert(pp);
                            to_add.entry(pp).or_insert(HashSet::new()).insert(p);
                        }
                    } else {
                        if let Some(other) = umbra.get(&pp) {
                            if other.is_passable() {
                                to_add.entry(p).or_insert(HashSet::new()).insert(pp);
                                umbra_to_add.entry(pp).or_insert(HashSet::new()).insert(p);
                            }
                        }
                    }
                }
            }
        }
        for (p, edges) in to_add {
            chunk.get_mut(&p).unwrap().get_edges_mut().extend(edges);
        }
        for (p, edges) in umbra_to_add {
            umbra.get_mut(&p).unwrap().get_edges_mut().extend(edges);
        }
    }
}
