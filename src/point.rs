use std::hash::Hash;
use std::iter::from_fn;

pub trait Point: Hash+Eq+Sized+Clone+std::fmt::Debug+Send {
    fn to_cube(&self, size: u32) -> [Self; 2];
    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool;
    fn expand(r: &[Self; 2], margin: u32) -> [Self; 2];
    fn contained(&self, r: &[Self; 2]) -> bool;
    fn chunk_index(&self, chunk_size: u32) -> (Self, usize);
    fn max_unrolled_index(chunk_size: u32) -> usize;
    fn chunks_in_region(r: &[Self; 2], chunk_size: u32) -> Vec<[Self; 2]>;
    fn points_in_region(r: &[Self; 2]) -> Vec<Self>;
    fn neighboors(&self) -> Vec<Self>;
    fn mul(&self, m: i32) -> Self;
    fn div(&self, m: i32) -> Self;
}

impl Point for [i32; 2] {
    fn to_cube(&self, size: u32) -> [Self; 2] {
        [[self[0], self[1]], [self[0] + size as i32, self[1] + size as i32]]
    }

    fn overlap_rect(a: &[Self; 2], other: &[Self; 2]) -> bool {
        (a[0][0] >= other[0][0] && a[0][0] < other[1][0]) ||
        (a[1][0] >= other[0][0] && a[1][0] < other[1][0]) ||
        (a[0][1] >= other[0][1] && a[0][1] < other[1][1]) ||
        (a[1][1] >= other[0][1] && a[1][1] < other[1][1])
    }

    fn expand(r: &[Self; 2], margin: u32) -> [Self; 2] {
        [[r[0][0] - margin as i32, r[0][1] - margin as i32], [r[1][0] + margin as i32, r[1][1] + margin as i32]]
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

    fn chunks_in_region(r: &[Self; 2], chunk_size: u32) -> Vec<[Self; 2]> {
        let low_x = (r[0][0] as f64 / chunk_size as f64).floor() as i32 * chunk_size as i32;
        let mut y = (r[0][1] as f64 / chunk_size as f64).floor() as i32 * chunk_size as i32;
        let mut x = low_x;

        //FIXME: I'd really rather just return the iterator but I'm not sure how to make the types
        //work
        from_fn(move || {
            if x < r[1][0] || y < r[1][1] {
                let p = Some([[x, y], [x+chunk_size as i32, y+chunk_size as i32]]);
                if x > r[1][0] {
                    x = low_x;
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

    fn points_in_region(r: &[Self; 2]) -> Vec<Self> {
        (r[0][0]..r[1][0]).map(move |x| (r[0][1]..r[1][1]).map(move |y| [x, y])).flatten().collect()
    }

    fn neighboors(&self) -> Vec<Self> {
        [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().map(|(dx, dy)| [self[0]+dx, self[1]+dy]).collect()
    }

    fn mul(&self, m: i32) -> Self {
        [self[0] * m, self[1] * m]
    }

    fn div(&self, m: i32) -> Self {
        [self[0] / m, self[1] / m]
    }
}
