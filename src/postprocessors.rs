use std::collections::{HashMap, HashSet};

use super::{
    Chunks, Generator,
    analysis::Passable,
};

pub struct AllConnected<'a, TileType> where TileType: Passable {
    inner: Box<dyn Generator<TileType> + 'a>,
}

impl<'a, TileType: Passable> AllConnected<'a, TileType> {
    pub fn new<G: Generator<TileType> + 'a>(inner: G) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}

impl<'a, TileType: Passable + Clone> Generator<TileType> for AllConnected<'a, TileType> {
    fn new_chunk(&mut self, location: &(i32, i32), chunks: &mut Chunks<TileType>) {
        self.inner.new_chunk(location, chunks);
        //FIXME: I shouldn't need to clone but I'm having trouble with the borrow checker
        let mut chunk = chunks.get_chunk_mut(location).unwrap().clone();
        let mut changed = true;
        let mut tile_to_region = HashMap::new();
        let mut regions = HashMap::new();
        let mut region_id = 0;
        let width = chunk.len() as i32;
        let height = chunk[0].len() as i32;
        while changed {
            changed = false;
            for (x, col) in chunk.iter().enumerate() {
                let x = x as i32;
                for (y, t) in col.iter().enumerate() {
                    let y = y as i32;
                    if t.is_passable() {
                        let region = *tile_to_region.entry((x, y)).or_insert_with(|| {
                            let id = region_id;
                            region_id += 1;
                            regions.insert(id, vec![(x,y)]);
                            id
                        });
                        for (dx, dy) in &[(-1, 0), (0, -1), (1, 0), (0, 1)] {
                            let xx = x + dx;
                            let yy = y + dy;
                            if !(xx == x && yy == y) && xx >= 0 && xx < width && yy >= 0 && yy < height {
                                if chunk[xx as usize][yy as usize].is_passable() {
                                    if let Some(other_region) = tile_to_region.get(&(xx, yy)) {
                                        if region != *other_region {
                                            let other_tiles = regions.remove(other_region).unwrap();
                                            for tile in &other_tiles {
                                                tile_to_region.insert(*tile, region);
                                            }                                        regions.get_mut(&region).unwrap().extend(other_tiles);
                                            changed = true;
                                        }
                                    } else {
                                        tile_to_region.insert((xx,yy), region);
                                        regions.get_mut(&region).unwrap().push((xx,yy));
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        //Assign open regions in neighbooring chunks to the largest region so that we tend to
        //connect with them
        let (largest_region, largest_region_tiles) = regions.iter_mut().max_by_key(|(_, tiles)| tiles.len()).unwrap();
        for (dx, dy) in &[(-1, 0), (0, -1), (1, 0), (0, 1)] {
            let xx = location.0 + dx * width;
            let yy = location.1 + dy * height;
            if let Some(other) = chunks.get_chunk(&(xx, yy)) {
                let width = other.len();
                let height = other[0].len();
                let edge:Vec<(i32, i32)> = match (dx, dy) {
                    (-1, 0) => (0..height).map(|y| (width as i32-1, y as i32)).collect(),
                    (0, -1) => (0..width).map(|x| (x as i32, 0)).collect(),
                    (1, 0) => (0..height).map(|y| (0, y as i32)).collect(),
                    (0, 1) => (0..width).map(|x| (x as i32, height as i32 -1)).collect(),
                    _ => panic!(),
                };
                for (x,y) in edge {
                    if other[x as usize][y as usize].is_passable() {
                        let x = x + width as i32*dx;
                        let y = y + height as i32*dy;
                        tile_to_region.insert((x,y), *largest_region);
                        largest_region_tiles.push((x,y))
                    }
                }
            }
        }

        while regions.len() > 1 {
            let (region, region_tiles) = regions.iter().min_by_key(|(_, tiles)| tiles.len()).unwrap();
            let region = *region;
            let mut to_expand = HashSet::new();

            for (x,y) in region_tiles {
                for (dx, dy) in &[(-1, 0), (0, -1), (1, 0), (0, 1)] {
                    let xx = x + dx;
                    let yy = y + dy;
                    if xx >= 0 && xx < width && yy >= 0 && yy < height {
                        if !chunk[xx as usize][yy as usize].is_passable() {
                            to_expand.insert((xx, yy));
                        }
                    }
                }
            }

            // Repeatedly expand the smallest region until all regions are connected
            for (x,y) in to_expand {
                chunk[x as usize][y as usize].set_passable(true);
                regions.get_mut(&region).unwrap().push((x, y));
                tile_to_region.insert((x, y), region);
                for (dx, dy) in &[(-1, 0), (0, -1), (1, 0), (0, 1)] {
                    let xx = x + dx;
                    let yy = y + dy;
                    if let Some(other_region) = tile_to_region.get(&(xx, yy)) {
                        if *other_region != region {
                            let other_tiles = regions.remove(other_region).unwrap();
                            for tile in &other_tiles {
                                tile_to_region.insert(*tile, region);
                            }
                            regions.get_mut(&region).unwrap().extend(other_tiles);
                        }
                    }
                }
            }
        }
        *chunks.get_chunk_mut(location).unwrap() = chunk;
    }
}
