use image;
use grid_builder::{
    Map,
    Generator,
    noise_based_generators::FbmGenerator,
    postprocessors::AllConnected,
};

fn main() {
    let fbm = FbmGenerator::new(4, 0.8, 1.0/20.0);
    let b: Vec<Box<dyn Generator<bool>>> = vec![Box::new(AllConnected::new(fbm))];
    let mut map: Map<bool> = Map::new(b, (30, 30), 1);
    map.maybe_generate_chunk(&(0,0));
    map.maybe_generate_chunk(&(0,1));
    map.maybe_generate_chunk(&(0,2));
    map.maybe_generate_chunk(&(1,0));
    map.maybe_generate_chunk(&(1,1));
    map.maybe_generate_chunk(&(1,2));
    map.maybe_generate_chunk(&(2,0));
    map.maybe_generate_chunk(&(2,1));
    map.maybe_generate_chunk(&(2,2));
    let mut imgbuf = image::ImageBuffer::new(90, 90);
    for (x,y,p) in imgbuf.enumerate_pixels_mut() {
        if *map.get_or_generate_tile(&(x as i32, y as i32), 0) {
            *p = image::Rgb([0, 0, 0]);
        } else {
            *p = image::Rgb([255, 255, 255]);
        }
    }
    imgbuf.save("/tmp/test.png").unwrap()
}
