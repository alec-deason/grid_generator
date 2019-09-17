use std::collections::HashSet;

use super::{
    Generator, Chunks, Point,
};

pub trait Passable {
    fn is_passable(&self) -> bool;
    fn set_passable(&mut self, passable: bool);
}

pub trait Connected {
    fn get_edges(&self) -> &HashSet<Point>;
    fn get_edges_mut(&mut self) -> &mut HashSet<Point>;
}

pub struct Connectivity;

impl<TileType: Connected + Passable + Send> Generator<TileType> for Connectivity {
    fn new_chunk(&mut self, location: &Point, chunks: &mut Chunks<TileType>) {
        let (width, height, depth) = chunks.chunk_size;
        let width = width as i32;
        let height = height as i32;
        let depth = depth as i32;
        for x in 0..width {
            let x = x + location.0;
            for y in 0..height {
                let y = y + location.1;
                for z in 0..depth {
                    let z = z + location.2;
                    let passable = chunks.get_tile(&(x, y, z)).unwrap().is_passable();
                    if passable {
                        let mut edges_to_add = HashSet::new();
                        for (dx, dy) in &[(-1, 0), (0, -1), (1, 0), (0, 1)] {
                            let xx = x as i32 + dx;
                            let yy = y as i32 + dy;
                            if let Some(other) = chunks.get_tile_mut(&(xx, yy, z as i32)) {
                                if other.is_passable() {
                                    edges_to_add.insert((xx, yy, z));
                                    let edges = other.get_edges_mut();
                                    edges.insert((x,y,z));
                                }
                            }
                        }
                        chunks.get_tile_mut(&(x,y,z)).unwrap().get_edges_mut().extend(&edges_to_add);
                    }
                }
            }
        }
    }
}
