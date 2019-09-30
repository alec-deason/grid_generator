use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use parking_lot_core::{park, unpark_one, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};

use crate::point::Point;

#[derive(Hash, PartialEq, Eq, Copy, Clone)]
pub struct LockKey(usize, bool);

struct Inner<Point> {
    read: HashMap<LockKey, (Vec<[Point; 2]>, Vec<LockKey>)>,
    write: HashMap<LockKey, (Vec<[Point; 2]>, Vec<LockKey>)>,
    lock_id: usize,
    pending: HashSet<LockKey>,
}

pub struct Lock<Point> {
    lock: Mutex<Inner<Point>>,
}

impl<P: Point> Lock<P> {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(Inner {
                read: HashMap::new(),
                write: HashMap::new(),
                lock_id: 0,
                pending: HashSet::new(),
            }),
        }
    }

    pub fn lock_region(&self, regions: &[[P; 2]], is_write: bool, blocking: bool) -> Option<Guard<P>> {
        let mut conflict = true;
        let mut key = LockKey(0, is_write);
        while conflict {
            conflict = false;
            {
                let mut inner = self.lock.lock().unwrap();
                if inner.read.is_empty() && inner.write.is_empty() && inner.pending.is_empty() {
                    inner.lock_id = 0;
                }
                key = LockKey(inner.lock_id, is_write);
                inner.lock_id += 1;
                'outer: for (_, (lock_regions, queue)) in &mut inner.write {
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
                    'outer2: for (_, (lock_regions, queue)) in &mut inner.read {
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
                    if is_write {
                        inner.write.insert(key, (regions.to_vec(), vec![]));
                    } else {
                        inner.read.insert(key, (regions.to_vec(), vec![]));
                    }
                } else {
                    if blocking {
                        inner.pending.insert(key);
                        unsafe {
                            park(
                                key.0,
                                || true,
                                || {},
                                |_,_| {},
                                DEFAULT_PARK_TOKEN,
                                None,
                            );
                        }
                    } else {
                        return None;
                    }
                }
            }
        }

        Some(Guard {
            owner: self,
            key: key
        })
    }

    pub fn read_region(&self, regions: &[[P; 2]]) -> Guard<P> {
        self.lock_region(regions, false, true).unwrap()
    }

    pub fn try_read_region(&self, regions: &[[P; 2]]) -> Option<Guard<P>> {
        self.lock_region(regions, false, false)
    }

    pub fn write_region(&self, regions: &[[P; 2]]) -> Guard<P> {
        self.lock_region(regions, true, true).unwrap()
    }

    pub fn try_write_region(&self, regions: &[[P; 2]]) -> Option<Guard<P>> {
        self.lock_region(regions, true, false)
    }

    fn unlock_region(&self, key: &LockKey) {
        let mut inner = self.lock.lock().unwrap();
        let queue;
        if key.1 {
            queue = inner.write.remove(key).unwrap().1;
        } else {
            queue = inner.read.remove(key).unwrap().1;
        }
        for other_lock in queue {
            unsafe {
                unpark_one(
                    other_lock.0,
                    |_| DEFAULT_UNPARK_TOKEN,
                );
            }
        }
    }
}

pub struct Guard<'a, P> where P: Point{
    owner: &'a Lock<P>,
    key: LockKey,
}

impl<'a, P: Point> Drop for Guard<'a, P> {
    fn drop(&mut self) {
        self.owner.unlock_region(&self.key);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_lock_region() {
        let mut lock = Lock::new();
        let _read_key = lock.read_region(&[[[0, 0], [100, 100]]]);
        assert!(lock.try_read_region(&[[[200, 200], [250, 250]]]).is_some());
        assert!(lock.try_read_region(&[[[20, 20], [25, 25]]]).is_some());
        assert!(lock.try_write_region(&[[[20, 20], [25, 25]]]).is_none());
    }

    #[test]
    fn write_lock_region() {
        let mut lock = Lock::new();
        let _write_key = lock.write_region(&[[[0, 0], [100, 100]]]);
        assert!(lock.try_read_region(&[[[200, 200], [250, 250]]]).is_some());
        assert!(lock.try_read_region(&[[[20, 20], [25, 25]]]).is_none());
        assert!(lock.try_write_region(&[[[20, 20], [25, 25]]]).is_none());
    }

    #[test]
    fn unlock_read() {
        let mut lock = Lock::new();
        {
            let read_key = lock.read_region(&[[[0, 0], [100, 100]]]);
            assert!(lock.try_write_region(&[[[20, 20], [25, 25]]]).is_none());
        }
        assert!(lock.try_write_region(&[[[20, 20], [25, 25]]]).is_some());
    }

    #[test]
    fn unlock_write() {
        let mut lock = Lock::new();
        {
            let write_key = lock.write_region(&[[[0, 0], [100, 100]]]);
            assert!(lock.try_read_region(&[[[20, 20], [25, 25]]]).is_none());
        }
        assert!(lock.try_read_region(&[[[20, 20], [25, 25]]]).is_some());
    }
}
