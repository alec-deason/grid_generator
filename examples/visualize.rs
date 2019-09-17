use std::collections::HashSet;

use image;
use grid_builder::{
    Map,
    Generator,
    noise_based_generators::FbmGenerator,
    postprocessors::{AllConnected,},
    analysis::{Connectivity, Passable, Connected,},
};

#[derive(Clone)]
struct Tile {
    edges: HashSet<Point>,
    passable: bool,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            edges: HashSet::new(),
            passable: true,
        }
    }
}

impl Passable for Tile {
    fn is_passable(&self) -> bool { self.passable }
    fn set_passable(&mut self, passable: bool) { self.passable = passable; }
}

impl Connected for Tile {
    fn get_edges(&self) -> &HashSet<Point> { &self.edges }
    fn get_edges_mut(&mut self) -> &mut HashSet<Point> { &mut self.edges }
}

fn main() {
    let fbm = FbmGenerator::new(4, 0.8, 1.0/20.0);
    let b: Vec<Box<dyn Generator<Tile>>> = vec![
        Box::new(AllConnected::new(fbm)),
        Box::new(Connectivity),
    ];
    let mut map: Map<Tile> = Map::new(b, (30, 30));
    let mut imgbuf = image::ImageBuffer::new(90, 90);
    for (x,y,p) in imgbuf.enumerate_pixels_mut() {
        if map.get_or_generate_tile(&(x as i32, y as i32)).is_passable() {
            *p = image::Rgb([0, 0, 0]);
        } else {
            *p = image::Rgb([255, 255, 255]);
        }
    }
    imgbuf.save("/tmp/test.png").unwrap()
}
