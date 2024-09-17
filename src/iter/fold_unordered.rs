use super::noop::NoopReducer;
use super::plumbing::*;
use super::*;

use std::fmt::{self, Debug};
use std::sync::Mutex;

/// `FoldUnordered` is an iterator that applies a function over an iterator producing a single value.
/// This struct is created by the [`fold_unordered()`] method on [`ParallelIterator`]
///
/// [`fold_unordered()`]: trait.ParallelIterator.html#method.fold_unordered
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct FoldUnordered<I, ID, F> {
    base: I,
    identity: ID,
    fold_op: F,
}

impl<I: ParallelIterator + Debug, ID, F> Debug for FoldUnordered<I, ID, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FoldUnordered")
            .field("base", &self.base)
            .finish()
    }
}

impl<U, I, ID, F> FoldUnordered<I, ID, F>
where
    I: ParallelIterator,
    F: Fn(U, I::Item) -> U + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    U: Send,
{
    pub(super) fn new(base: I, identity: ID, fold_op: F) -> Self {
        Self {
            base,
            identity,
            fold_op,
        }
    }
}

impl<U, I, ID, F> ParallelIterator for FoldUnordered<I, ID, F>
where
    I: ParallelIterator,
    F: Fn(U, I::Item) -> U + Sync + Send,
    ID: Fn() -> U + Sync + Send,
    U: Send,
{
    type Item = U;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let tls: Vec<_> = (0..crate::current_num_threads())
            .map(|_| Mutex::new(None))
            .collect();
        let extra = Mutex::new(vec![]);
        let driver = FoldUnorderedDriver {
            fold_op: &self.fold_op,
            identity: &self.identity,
            tls: &tls,
            extra: &extra,
        };
        self.base.drive_unindexed(driver);

        tls.into_par_iter()
            .filter_map(|acc| acc.into_inner().unwrap())
            .chain(extra.into_inner().unwrap())
            .drive_unindexed(consumer)
    }
}

struct FoldUnorderedDriver<'a, ID, F, Acc> {
    fold_op: &'a F,
    identity: &'a ID,
    tls: &'a [Mutex<Option<Acc>>],
    extra: &'a Mutex<Vec<Acc>>,
}

impl<ID, F, Acc> FoldUnorderedDriver<'_, ID, F, Acc>
where
    ID: Fn() -> Acc + Sync,
    Acc: Send,
{
    fn with(&self, f: impl FnOnce(Acc) -> Acc) {
        let tls = crate::current_thread_index().and_then(|i| self.tls.get(i));

        let mut acc = tls
            .and_then(|tls| tls.lock().unwrap().take())
            .or_else(|| self.extra.lock().unwrap().pop())
            .unwrap_or_else(self.identity);

        acc = f(acc);

        match tls.map(|tls| tls.lock().unwrap()) {
            Some(mut tls) if tls.is_none() => *tls = Some(acc),
            _ => self.extra.lock().unwrap().push(acc),
        }
    }
}

impl<T, ID, F, Acc> Consumer<T> for FoldUnorderedDriver<'_, ID, F, Acc>
where
    F: Fn(Acc, T) -> Acc + Sync,
    ID: Fn() -> Acc + Sync,
    Acc: Send,
{
    type Folder = Self;
    type Reducer = NoopReducer;
    type Result = ();

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (Self { ..self }, self, NoopReducer)
    }

    fn into_folder(self) -> Self::Folder {
        self
    }

    fn full(&self) -> bool {
        false
    }
}

impl<T, ID, F, Acc> UnindexedConsumer<T> for FoldUnorderedDriver<'_, ID, F, Acc>
where
    F: Fn(Acc, T) -> Acc + Sync,
    ID: Fn() -> Acc + Sync,
    Acc: Send,
{
    fn split_off_left(&self) -> Self {
        Self { ..*self }
    }

    fn to_reducer(&self) -> Self::Reducer {
        NoopReducer
    }
}

impl<T, ID, F, Acc> Folder<T> for FoldUnorderedDriver<'_, ID, F, Acc>
where
    F: Fn(Acc, T) -> Acc + Sync,
    ID: Fn() -> Acc + Sync,
    Acc: Send,
{
    type Result = ();

    fn consume(self, item: T) -> Self {
        self.with(|acc| (self.fold_op)(acc, item));
        self
    }

    fn consume_iter<I>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.with(|acc| iter.into_iter().fold(acc, self.fold_op));
        self
    }

    fn complete(self) {}

    fn full(&self) -> bool {
        false
    }
}
