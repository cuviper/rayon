use super::plumbing::*;
use super::private::ControlFlow::*;
use super::private::{Bubble, ControlFlow};
use super::*;

use std::fmt::{self, Debug};
use std::marker::PhantomData;

pub fn try_fold<U, I, ID, F>(base: I, identity: ID, fold_op: F) -> TryFold<I, U, ID, F>
where
    I: ParallelIterator,
    F: Fn(U::Inner, I::Item) -> U + Sync + Send,
    ID: Fn() -> U::Inner + Sync + Send,
    U: Bubble + Send,
{
    TryFold {
        base: base,
        identity: identity,
        fold_op: fold_op,
        marker: PhantomData,
    }
}

/// `TryFold` is an iterator that applies a function over an iterator producing a single value.
/// This struct is created by the [`try_fold()`] method on [`ParallelIterator`]
///
/// [`try_fold()`]: trait.ParallelIterator.html#method.try_fold
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct TryFold<I, U, ID, F> {
    base: I,
    identity: ID,
    fold_op: F,
    marker: PhantomData<U>,
}

impl<U, I: ParallelIterator + Debug, ID, F> Debug for TryFold<I, U, ID, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TryFold").field("base", &self.base).finish()
    }
}

impl<U, I, ID, F> ParallelIterator for TryFold<I, U, ID, F>
where
    I: ParallelIterator,
    F: Fn(U::Inner, I::Item) -> U + Sync + Send,
    ID: Fn() -> U::Inner + Sync + Send,
    U: Bubble + Send,
{
    type Item = U;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let consumer1 = TryFoldConsumer {
            base: consumer,
            identity: &self.identity,
            fold_op: &self.fold_op,
            marker: PhantomData,
        };
        self.base.drive_unindexed(consumer1)
    }
}

struct TryFoldConsumer<'c, U, C, ID: 'c, F: 'c> {
    base: C,
    identity: &'c ID,
    fold_op: &'c F,
    marker: PhantomData<U>,
}

impl<'r, U, T, C, ID, F> Consumer<T> for TryFoldConsumer<'r, U, C, ID, F>
where
    C: Consumer<U>,
    F: Fn(U::Inner, T) -> U + Sync,
    ID: Fn() -> U::Inner + Sync,
    U: Bubble + Send,
{
    type Folder = TryFoldFolder<'r, C::Folder, U, F>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);
        (
            TryFoldConsumer { base: left, ..self },
            TryFoldConsumer {
                base: right,
                ..self
            },
            reducer,
        )
    }

    fn into_folder(self) -> Self::Folder {
        TryFoldFolder {
            base: self.base.into_folder(),
            control_flow: Continue((self.identity)()),
            fold_op: self.fold_op,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<'r, U, T, C, ID, F> UnindexedConsumer<T> for TryFoldConsumer<'r, U, C, ID, F>
where
    C: UnindexedConsumer<U>,
    F: Fn(U::Inner, T) -> U + Sync,
    ID: Fn() -> U::Inner + Sync,
    U: Bubble + Send,
{
    fn split_off_left(&self) -> Self {
        TryFoldConsumer {
            base: self.base.split_off_left(),
            ..*self
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}

struct TryFoldFolder<'r, C, U: Bubble, F: 'r> {
    base: C,
    fold_op: &'r F,
    control_flow: ControlFlow<U::Inner, U>,
}

impl<'r, C, U, F, T> Folder<T> for TryFoldFolder<'r, C, U, F>
where
    C: Folder<U>,
    F: Fn(U::Inner, T) -> U + Sync,
    U: Bubble,
{
    type Result = C::Result;

    fn consume(self, item: T) -> Self {
        let fold_op = self.fold_op;
        let control_flow = match self.control_flow {
            Continue(acc) => fold_op(acc, item).bubble(),
            Break(value) => Break(value),
        };
        TryFoldFolder {
            control_flow: control_flow,
            ..self
        }
    }

    fn complete(self) -> C::Result {
        let item = self.control_flow.unbubble();
        self.base.consume(item).complete()
    }

    fn full(&self) -> bool {
        match self.control_flow {
            Break(_) => true,
            _ => self.base.full(),
        }
    }
}

// ///////////////////////////////////////////////////////////////////////////

pub fn try_fold_with<U, I, F>(base: I, item: U::Inner, fold_op: F) -> TryFoldWith<I, U, F>
where
    I: ParallelIterator,
    F: Fn(U::Inner, I::Item) -> U + Sync,
    U: Bubble + Send,
    U::Inner: Clone + Send,
{
    TryFoldWith {
        base: base,
        item: item,
        fold_op: fold_op,
    }
}

/// `TryFoldWith` is an iterator that applies a function over an iterator producing a single value.
/// This struct is created by the [`try_fold_with()`] method on [`ParallelIterator`]
///
/// [`try_fold_with()`]: trait.ParallelIterator.html#method.try_fold_with
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct TryFoldWith<I, U: Bubble, F> {
    base: I,
    item: U::Inner,
    fold_op: F,
}

impl<I: ParallelIterator + Debug, U: Bubble, F> Debug for TryFoldWith<I, U, F>
where
    U::Inner: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TryFoldWith")
            .field("base", &self.base)
            .field("item", &self.item)
            .finish()
    }
}

impl<U, I, F> ParallelIterator for TryFoldWith<I, U, F>
where
    I: ParallelIterator,
    F: Fn(U::Inner, I::Item) -> U + Sync + Send,
    U: Bubble + Send,
    U::Inner: Clone + Send,
{
    type Item = U;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let consumer1 = TryFoldWithConsumer {
            base: consumer,
            item: self.item,
            fold_op: &self.fold_op,
        };
        self.base.drive_unindexed(consumer1)
    }
}

struct TryFoldWithConsumer<'c, C, U: Bubble, F: 'c> {
    base: C,
    item: U::Inner,
    fold_op: &'c F,
}

impl<'r, U, T, C, F> Consumer<T> for TryFoldWithConsumer<'r, C, U, F>
where
    C: Consumer<U>,
    F: Fn(U::Inner, T) -> U + Sync,
    U: Bubble + Send,
    U::Inner: Clone + Send,
{
    type Folder = TryFoldFolder<'r, C::Folder, U, F>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);
        (
            TryFoldWithConsumer {
                base: left,
                item: self.item.clone(),
                ..self
            },
            TryFoldWithConsumer {
                base: right,
                ..self
            },
            reducer,
        )
    }

    fn into_folder(self) -> Self::Folder {
        TryFoldFolder {
            base: self.base.into_folder(),
            control_flow: Continue(self.item),
            fold_op: self.fold_op,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

impl<'r, U, T, C, F> UnindexedConsumer<T> for TryFoldWithConsumer<'r, C, U, F>
where
    C: UnindexedConsumer<U>,
    F: Fn(U::Inner, T) -> U + Sync,
    U: Bubble + Send,
    U::Inner: Clone + Send,
{
    fn split_off_left(&self) -> Self {
        TryFoldWithConsumer {
            base: self.base.split_off_left(),
            item: self.item.clone(),
            ..*self
        }
    }

    fn to_reducer(&self) -> Self::Reducer {
        self.base.to_reducer()
    }
}
