/// Rayon extensions to `RawTable`

use std::marker;
use std::ptr;

use rayon::iter::internal::{UnindexedProducer, Folder};

use super::{RawTable, RawBucket, EMPTY_BUCKET};

impl<K, V> RawTable<K, V> {
    pub fn par_iter(&self) -> ParIter<K, V> {
        ParIter { iter: SplitBuckets::new(self) }
    }

    pub fn par_iter_mut(&mut self) -> ParIterMut<K, V> {
        ParIterMut {
            iter: SplitBuckets::new(self),
            marker: marker::PhantomData,
        }
    }

    pub fn par_drain(&mut self) -> ParDrain<K, V> {
        // Pre-set the map size to zero, indicating all items drained.
        // FIXME: If the `ParDrain` or any of its splits are leaked, then there
        // may remain buckets that aren't `EMPTY_BUCKET`!  When this is used for
        // `into_iter()`, that doesn't matter -- just more leaked values.  But
        // if we ever make a `par_drain` available outside the crate, we may
        // need to fixup the size and/or buckets properly.
        self.size = 0;

        // Replace the marker regardless of lifetime bounds on parameters.
        ParDrain {
            iter: SplitBuckets::new(self),
            marker: marker::PhantomData,
        }
    }
}


struct SplitBuckets<'a, K, V> {
    bucket: RawBucket<K, V>,
    end: usize,
    marker: marker::PhantomData<&'a ()>,
}

impl<'a, K, V> SplitBuckets<'a, K, V> {
    fn new(table: &'a RawTable<K, V>) -> Self {
        SplitBuckets {
            bucket: table.raw_bucket_at(0),
            end: table.capacity(),
            marker: marker::PhantomData,
        }
    }

    fn split(&self) -> (Self, Option<Self>) {
        let mut left = SplitBuckets { ..*self };
        let len = left.end - left.bucket.idx;
        if len > 1 {
            let mut right = SplitBuckets { ..left };
            right.bucket.idx += len / 2;
            left.end = right.bucket.idx;
            (left, Some(right))
        } else {
            (left, None)
        }
    }
}

impl<'a, K, V> Iterator for SplitBuckets<'a, K, V> {
    type Item = RawBucket<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.bucket.idx < self.end {
            let item = self.bucket;
            self.bucket.idx += 1;

            unsafe {
                if *item.hash() != EMPTY_BUCKET {
                    return Some(item);
                }
            }
        }
        None
    }
}


/// Parallel iterator over shared references to entries in a table.
pub struct ParIter<'a, K: 'a, V: 'a> {
    iter: SplitBuckets<'a, K, V>,
}

unsafe impl<'a, K: Sync, V: Sync> Send for ParIter<'a, K, V> {}

impl<'a, K: Sync, V: Sync> UnindexedProducer for ParIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn split(mut self) -> (Self, Option<Self>) {
        let (left, right) = self.iter.split();
        self.iter = left;
        let right = right.map(|iter| ParIter { iter: iter });
        (self, right)
    }

    fn fold_with<F>(self, folder: F) -> F
        where F: Folder<Self::Item>
    {
        let iter = self.iter
            .map(|bucket| unsafe {
                     let pair_ptr = bucket.pair();
                     (&(*pair_ptr).0, &(*pair_ptr).1)
                 });
        folder.consume_iter(iter)
    }
}


/// Parallel iterator over mutable references to entries in a table.
pub struct ParIterMut<'a, K: 'a, V: 'a> {
    iter: SplitBuckets<'a, K, V>,
    // To ensure invariance with respect to V
    marker: marker::PhantomData<&'a mut V>,
}

unsafe impl<'a, K: Sync, V: Send> Send for ParIterMut<'a, K, V> {}

impl<'a, K: Sync, V: Send> UnindexedProducer for ParIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn split(mut self) -> (Self, Option<Self>) {
        let (left, right) = self.iter.split();
        self.iter = left;
        let right = right.map(|iter| ParIterMut { iter: iter, ..self });
        (self, right)
    }

    fn fold_with<F>(self, folder: F) -> F
        where F: Folder<Self::Item>
    {
        let iter = self.iter
            .map(|bucket| unsafe {
                     let pair_ptr = bucket.pair();
                     (&(*pair_ptr).0, &mut (*pair_ptr).1)
                 });
        folder.consume_iter(iter)
    }
}

/// Parallel iterator over the entries in a table, clearing the table.
pub struct ParDrain<'a, K: 'a, V: 'a> {
    iter: SplitBuckets<'a, K, V>,
    marker: marker::PhantomData<&'a RawTable<K, V>>,
}

unsafe impl<'a, K: Send, V: Send> Send for ParDrain<'a, K, V> {}

impl<'a, K: Send, V: Send> UnindexedProducer for ParDrain<'a, K, V> {
    type Item = (K, V);

    fn split(mut self) -> (Self, Option<Self>) {
        let (left, right) = self.iter.split();
        self.iter = left;
        let right = right.map(|iter| ParDrain { iter: iter, ..self });
        (self, right)
    }

    fn fold_with<F>(mut self, folder: F) -> F
        where F: Folder<Self::Item>
    {
        let iter = self.iter
            .by_ref()
            .map(|bucket| unsafe {
                     *bucket.hash() = EMPTY_BUCKET;
                     ptr::read(bucket.pair())
                 });
        folder.consume_iter(iter)
    }
}

impl<'a, K: 'a, V: 'a> Drop for ParDrain<'a, K, V> {
    fn drop(&mut self) {
        for bucket in self.iter.by_ref() {
            unsafe {
                *bucket.hash() = EMPTY_BUCKET;
                ptr::drop_in_place(bucket.pair());
            }
        }
    }
}
