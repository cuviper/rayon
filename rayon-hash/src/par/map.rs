/// Rayon extensions to `HashMap`

use rayon::iter::{ParallelIterator, IntoParallelIterator, FromParallelIterator};
use rayon::iter::internal::{UnindexedConsumer, bridge_unindexed};

use super::{Hash, HashMap, BuildHasher, RawTable, table};


pub struct ParIntoIter<K: Send, V: Send> {
    table: RawTable<K, V>,
}

pub struct ParIter<'a, K: Sync + 'a, V: Sync + 'a> {
    inner: table::ParIter<'a, K, V>,
}

pub struct ParIterMut<'a, K: Sync + 'a, V: Send + 'a> {
    inner: table::ParIterMut<'a, K, V>,
}

pub struct ParKeys<'a, K: Sync + 'a, V: Sync + 'a> {
    inner: ParIter<'a, K, V>,
}

pub struct ParValues<'a, K: Sync + 'a, V: Sync + 'a> {
    inner: ParIter<'a, K, V>,
}

pub struct ParValuesMut<'a, K: Sync + 'a, V: Send + 'a> {
    inner: ParIterMut<'a, K, V>,
}


impl<K: Sync, V: Sync, S> HashMap<K, V, S> {
    pub fn par_keys(&self) -> ParKeys<K, V> {
        ParKeys { inner: self.into_par_iter() }
    }

    pub fn par_values(&self) -> ParValues<K, V> {
        ParValues { inner: self.into_par_iter() }
    }
}

impl<K, V, S> HashMap<K, V, S>
    where K: Eq + Hash + Sync,
          V: PartialEq + Sync,
          S: BuildHasher + Sync
{
    pub fn par_eq(&self, other: &Self) -> bool {
        self.len() == other.len() &&
        self.into_par_iter().all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
    }
}

impl<K: Sync, V: Send, S> HashMap<K, V, S> {
    pub fn par_values_mut(&mut self) -> ParValuesMut<K, V> {
        ParValuesMut { inner: self.into_par_iter() }
    }
}


impl<K: Send, V: Send, S> IntoParallelIterator for HashMap<K, V, S> {
    type Item = (K, V);
    type Iter = ParIntoIter<K, V>;

    fn into_par_iter(self) -> Self::Iter {
        ParIntoIter { table: self.table }
    }
}

impl<'a, K: Sync, V: Sync, S> IntoParallelIterator for &'a HashMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type Iter = ParIter<'a, K, V>;

    fn into_par_iter(self) -> Self::Iter {
        ParIter { inner: self.table.par_iter() }
    }
}

impl<'a, K: Sync, V: Send, S> IntoParallelIterator for &'a mut HashMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type Iter = ParIterMut<'a, K, V>;

    fn into_par_iter(self) -> Self::Iter {
        ParIterMut { inner: self.table.par_iter_mut() }
    }
}


// This is equal to the normal `HashMap` -- no custom advantage.
impl<K, V, S> FromParallelIterator<(K, V)> for HashMap<K, V, S>
    where K: Eq + Hash + Send,
          V: Send,
          S: BuildHasher + Default + Send
{
    fn from_par_iter<P>(par_iter: P) -> Self
        where P: IntoParallelIterator<Item = (K, V)>
    {
        use std::collections::LinkedList;

        let list: LinkedList<_> = par_iter.into_par_iter()
            .fold(Vec::new, |mut vec, elem| {
                vec.push(elem);
                vec
            })
            .collect();

        let len = list.iter().map(Vec::len).sum();
        let start = HashMap::with_capacity_and_hasher(len, Default::default());
        list.into_iter().fold(start, |mut coll, vec| {
            coll.extend(vec);
            coll
        })
    }
}


impl<K: Send, V: Send> ParallelIterator for ParIntoIter<K, V> {
    type Item = (K, V);

    fn drive_unindexed<C>(mut self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        bridge_unindexed(self.table.par_drain(), consumer)
    }
}


impl<'a, K: Sync, V: Sync> ParallelIterator for ParIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        bridge_unindexed(self.inner, consumer)
    }
}


impl<'a, K: Sync, V: Send> ParallelIterator for ParIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        bridge_unindexed(self.inner, consumer)
    }
}


impl<'a, K: Sync, V: Sync> ParallelIterator for ParKeys<'a, K, V> {
    type Item = &'a K;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.inner.map(|(k, _)| k).drive_unindexed(consumer)
    }
}


impl<'a, K: Sync, V: Sync> ParallelIterator for ParValues<'a, K, V> {
    type Item = &'a V;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.inner.map(|(_, v)| v).drive_unindexed(consumer)
    }
}


impl<'a, K: Sync, V: Send> ParallelIterator for ParValuesMut<'a, K, V> {
    type Item = &'a mut V;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        self.inner.map(|(_, v)| v).drive_unindexed(consumer)
    }
}
