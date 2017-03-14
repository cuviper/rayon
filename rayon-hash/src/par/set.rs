/// Rayon extensions for `HashSet`

use rayon::iter::{ParallelIterator, IntoParallelIterator, FromParallelIterator};
use rayon::iter::internal::UnindexedConsumer;

use super::{Hash, HashSet, BuildHasher, map};


pub struct ParIntoIter<T: Send> {
    inner: map::ParIntoIter<T, ()>,
}

pub struct ParIter<'a, T: Sync + 'a> {
    inner: map::ParKeys<'a, T, ()>,
}

pub struct ParDifference<'a, T: Sync + 'a, S: Sync + 'a> {
    a: &'a HashSet<T, S>,
    b: &'a HashSet<T, S>,
}

pub struct ParSymmetricDifference<'a, T: Sync + 'a, S: Sync + 'a> {
    a: &'a HashSet<T, S>,
    b: &'a HashSet<T, S>,
}

pub struct ParIntersection<'a, T: Sync + 'a, S: Sync + 'a> {
    a: &'a HashSet<T, S>,
    b: &'a HashSet<T, S>,
}

pub struct ParUnion<'a, T: Sync + 'a, S: Sync + 'a> {
    a: &'a HashSet<T, S>,
    b: &'a HashSet<T, S>,
}


impl<T, S> HashSet<T, S>
    where T: Eq + Hash + Sync,
          S: BuildHasher + Sync
{
    pub fn par_difference<'a>(&'a self, other: &'a Self) -> ParDifference<'a, T, S> {
        ParDifference {
            a: self,
            b: other,
        }
    }

    pub fn par_symmetric_difference<'a>(&'a self,
                                        other: &'a Self)
                                        -> ParSymmetricDifference<'a, T, S> {
        ParSymmetricDifference {
            a: self,
            b: other,
        }
    }

    pub fn par_intersection<'a>(&'a self, other: &'a Self) -> ParIntersection<'a, T, S> {
        ParIntersection {
            a: self,
            b: other,
        }
    }

    pub fn par_union<'a>(&'a self, other: &'a Self) -> ParUnion<'a, T, S> {
        ParUnion {
            a: self,
            b: other,
        }
    }

    pub fn par_is_disjoint(&self, other: &Self) -> bool {
        self.into_par_iter().all(|x| !other.contains(x))
    }

    pub fn par_is_subset(&self, other: &Self) -> bool {
        self.into_par_iter().all(|x| other.contains(x))
    }

    pub fn par_is_superset(&self, other: &Self) -> bool {
        other.is_subset(self)
    }

    pub fn par_eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.par_is_subset(other)
    }
}


impl<T: Send, S> IntoParallelIterator for HashSet<T, S> {
    type Item = T;
    type Iter = ParIntoIter<T>;

    fn into_par_iter(self) -> Self::Iter {
        ParIntoIter { inner: self.map.into_par_iter() }
    }
}

impl<'a, T: Sync, S> IntoParallelIterator for &'a HashSet<T, S> {
    type Item = &'a T;
    type Iter = ParIter<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        ParIter { inner: self.map.par_keys() }
    }
}


// This is equal to the normal `HashSet` -- no custom advantage.
impl<T, S> FromParallelIterator<T> for HashSet<T, S>
    where T: Eq + Hash + Send,
          S: BuildHasher + Default + Send
{
    fn from_par_iter<P>(par_iter: P) -> Self
        where P: IntoParallelIterator<Item = T>
    {
        use std::collections::LinkedList;

        let list: LinkedList<_> = par_iter.into_par_iter()
            .fold(Vec::new, |mut vec, elem| {
                vec.push(elem);
                vec
            })
            .collect();

        let len = list.iter().map(Vec::len).sum();
        let start = HashSet::with_capacity_and_hasher(len, Default::default());
        list.into_iter().fold(start, |mut coll, vec| {
            coll.extend(vec);
            coll
        })
    }
}


impl<T: Send> ParallelIterator for ParIntoIter<T> {
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.inner.map(|(k, _)| k).drive_unindexed(consumer)
    }
}


impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.inner.drive_unindexed(consumer)
    }
}


impl<'a, T, S> ParallelIterator for ParDifference<'a, T, S>
    where T: Eq + Hash + Sync,
          S: BuildHasher + Sync
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.a
            .into_par_iter()
            .filter(|&x| !self.b.contains(x))
            .drive_unindexed(consumer)
    }
}


impl<'a, T, S> ParallelIterator for ParSymmetricDifference<'a, T, S>
    where T: Eq + Hash + Sync,
          S: BuildHasher + Sync
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.a
            .par_difference(self.b)
            .chain(self.b.par_difference(self.a))
            .drive_unindexed(consumer)
    }
}


impl<'a, T, S> ParallelIterator for ParIntersection<'a, T, S>
    where T: Eq + Hash + Sync,
          S: BuildHasher + Sync
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.a
            .into_par_iter()
            .filter(|&x| self.b.contains(x))
            .drive_unindexed(consumer)
    }
}


impl<'a, T, S> ParallelIterator for ParUnion<'a, T, S>
    where T: Eq + Hash + Sync,
          S: BuildHasher + Sync
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.a
            .into_par_iter()
            .chain(self.b.par_difference(self.a))
            .drive_unindexed(consumer)
    }
}
