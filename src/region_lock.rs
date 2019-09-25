use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use parking_lot_core::{park, unpark_one, DEFAULT_PARK_TOKEN, DEFAULT_UNPARK_TOKEN};

use crate::Point;

#[derive(Hash, PartialEq, Eq, Copy, Clone)]
pub struct LockKey(usize);

pub struct Lock<Point> {
    lock: Mutex<()>,
    read: HashMap<LockKey, (Vec<[Point; 2]>, Vec<LockKey>)>,
    write: HashMap<LockKey, (Vec<[Point; 2]>, Vec<LockKey>)>,
    pending: HashSet<LockKey>,
    lock_id: usize,
}

impl<P: Point> Lock<P> {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            read: HashMap::new(),
            write: HashMap::new(),
            pending: HashSet::new(),
            lock_id: 0,
        }
    }

    pub fn lock_region(&mut self, regions: &[[P; 2]], is_write: bool, blocking: bool) -> Option<LockKey> {
        let mut conflict = true;
        let mut key = LockKey(0);
        while conflict {
            conflict = false;
            {
                let _lock = self.lock.lock().unwrap();
                if self.read.is_empty() && self.write.is_empty() && self.pending.is_empty() {
                    self.lock_id = 0;
                }
                key = LockKey(self.lock_id);
                self.lock_id += 1;
                'outer: for (_, (lock_regions, queue)) in &mut self.write {
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
                    'outer2: for (_, (lock_regions, queue)) in &mut self.read {
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
                        self.write.insert(key, (regions.to_vec(), vec![]));
                    } else {
                        self.read.insert(key, (regions.to_vec(), vec![]));
                    }
                } else {
                    if blocking {
                        self.pending.insert(key);
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

        Some(key)
    }

    pub fn read_region(&mut self, regions: &[[P; 2]]) -> LockKey {
        self.lock_region(regions, false, true).unwrap()
    }

    pub fn try_read_region(&mut self, regions: &[[P; 2]]) -> Option<LockKey> {
        self.lock_region(regions, false, false)
    }

    pub fn write_region(&mut self, regions: &[[P; 2]]) -> LockKey {
        self.lock_region(regions, true, true).unwrap()
    }

    pub fn try_write_region(&mut self, regions: &[[P; 2]]) -> Option<LockKey> {
        self.lock_region(regions, true, false)
    }

    pub fn unlock_region(&mut self, lock_id: LockKey, is_write: bool) {
        let _lock = self.lock.lock().unwrap();
        let queue;
        if is_write {
            queue = self.write.remove(&lock_id).unwrap().1;
        } else {
            queue = self.read.remove(&lock_id).unwrap().1;
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
}
