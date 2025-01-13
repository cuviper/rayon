#![cfg(feature = "indexmap")]
//! Minimal `indexmap` support for `rustc-rayon`

use crate::iter::plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer};
use crate::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

mod map {
    use super::*;
    use indexmap::map::{IndexMap, Iter, IterMut, Slice};

    impl<'a, K, V, S> IntoParallelIterator for &'a IndexMap<K, V, S>
    where
        K: Sync,
        V: Sync,
    {
        type Item = (&'a K, &'a V);
        type Iter = ParIter<'a, K, V>;

        fn into_par_iter(self) -> Self::Iter {
            ParIter {
                slice: self.as_slice(),
            }
        }
    }

    #[derive(Debug)]
    pub struct ParIter<'a, K, V> {
        slice: &'a Slice<K, V>,
    }

    impl<'a, K: Sync, V: Sync> ParallelIterator for ParIter<'a, K, V> {
        type Item = (&'a K, &'a V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.slice.len())
        }
    }

    impl<K: Sync, V: Sync> IndexedParallelIterator for ParIter<'_, K, V> {
        fn drive<C>(self, consumer: C) -> C::Result
        where
            C: Consumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn len(&self) -> usize {
            self.slice.len()
        }

        fn with_producer<CB>(self, callback: CB) -> CB::Output
        where
            CB: ProducerCallback<Self::Item>,
        {
            callback.callback(IterProducer { slice: self.slice })
        }
    }

    struct IterProducer<'a, K, V> {
        slice: &'a Slice<K, V>,
    }

    impl<'a, K: Sync, V: Sync> Producer for IterProducer<'a, K, V> {
        type Item = (&'a K, &'a V);
        type IntoIter = Iter<'a, K, V>;

        fn into_iter(self) -> Self::IntoIter {
            self.slice.iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let (left, right) = self.slice.split_at(index);
            (Self { slice: left }, Self { slice: right })
        }
    }

    impl<'a, K, V, S> IntoParallelIterator for &'a mut IndexMap<K, V, S>
    where
        K: Sync + Send,
        V: Send,
    {
        type Item = (&'a K, &'a mut V);
        type Iter = ParIterMut<'a, K, V>;

        fn into_par_iter(self) -> Self::Iter {
            ParIterMut {
                slice: self.as_mut_slice(),
            }
        }
    }

    #[derive(Debug)]
    pub struct ParIterMut<'a, K, V> {
        slice: &'a mut Slice<K, V>,
    }

    impl<'a, K: Sync + Send, V: Send> ParallelIterator for ParIterMut<'a, K, V> {
        type Item = (&'a K, &'a mut V);

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.slice.len())
        }
    }

    impl<K: Sync + Send, V: Send> IndexedParallelIterator for ParIterMut<'_, K, V> {
        fn drive<C>(self, consumer: C) -> C::Result
        where
            C: Consumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn len(&self) -> usize {
            self.slice.len()
        }

        fn with_producer<CB>(self, callback: CB) -> CB::Output
        where
            CB: ProducerCallback<Self::Item>,
        {
            callback.callback(IterMutProducer { slice: self.slice })
        }
    }

    struct IterMutProducer<'a, K, V> {
        slice: &'a mut Slice<K, V>,
    }

    impl<'a, K: Sync + Send, V: Send> Producer for IterMutProducer<'a, K, V> {
        type Item = (&'a K, &'a mut V);
        type IntoIter = IterMut<'a, K, V>;

        fn into_iter(self) -> Self::IntoIter {
            self.slice.iter_mut()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let (left, right) = self.slice.split_at_mut(index);
            (Self { slice: left }, Self { slice: right })
        }
    }
}

mod set {
    use super::*;
    use indexmap::set::{IndexSet, Iter, Slice};

    impl<'a, T: Sync, S> IntoParallelIterator for &'a IndexSet<T, S> {
        type Item = &'a T;
        type Iter = ParIter<'a, T>;

        fn into_par_iter(self) -> Self::Iter {
            ParIter {
                slice: self.as_slice(),
            }
        }
    }

    #[derive(Debug)]
    pub struct ParIter<'a, T> {
        slice: &'a Slice<T>,
    }

    impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
        type Item = &'a T;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.slice.len())
        }
    }

    impl<T: Sync> IndexedParallelIterator for ParIter<'_, T> {
        fn drive<C>(self, consumer: C) -> C::Result
        where
            C: Consumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn len(&self) -> usize {
            self.slice.len()
        }

        fn with_producer<CB>(self, callback: CB) -> CB::Output
        where
            CB: ProducerCallback<Self::Item>,
        {
            callback.callback(IterProducer { slice: self.slice })
        }
    }

    struct IterProducer<'a, T> {
        slice: &'a Slice<T>,
    }

    impl<'a, T: Sync> Producer for IterProducer<'a, T> {
        type Item = &'a T;
        type IntoIter = Iter<'a, T>;

        fn into_iter(self) -> Self::IntoIter {
            self.slice.iter()
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let (left, right) = self.slice.split_at(index);
            (Self { slice: left }, Self { slice: right })
        }
    }
}
