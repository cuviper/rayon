use super::plumbing::*;
use super::private::ControlFlow::*;
use super::private::{Bubble, ControlFlow};
use super::ParallelIterator;

use std::sync::atomic::{AtomicBool, Ordering};

pub fn try_reduce<PI, R, ID, T>(pi: PI, identity: ID, reduce_op: R) -> T
where
    PI: ParallelIterator<Item = T>,
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    ID: Fn() -> T::Inner + Sync,
    T: Bubble + Send,
{
    let full = AtomicBool::new(false);
    let consumer = TryReduceConsumer {
        identity: &identity,
        reduce_op: &reduce_op,
        full: &full,
    };
    pi.drive_unindexed(consumer)
}

struct TryReduceConsumer<'r, R: 'r, ID: 'r> {
    identity: &'r ID,
    reduce_op: &'r R,
    full: &'r AtomicBool,
}

impl<'r, R, ID> Copy for TryReduceConsumer<'r, R, ID> {}

impl<'r, R, ID> Clone for TryReduceConsumer<'r, R, ID> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'r, R, ID, T> Consumer<T> for TryReduceConsumer<'r, R, ID>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    ID: Fn() -> T::Inner + Sync,
    T: Bubble + Send,
{
    type Folder = TryReduceFolder<'r, R, T>;
    type Reducer = Self;
    type Result = T;

    fn split_at(self, _index: usize) -> (Self, Self, Self) {
        (self, self, self)
    }

    fn into_folder(self) -> Self::Folder {
        TryReduceFolder {
            reduce_op: self.reduce_op,
            control_flow: Continue((self.identity)()),
            full: self.full,
        }
    }

    fn full(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }
}

impl<'r, R, ID, T> UnindexedConsumer<T> for TryReduceConsumer<'r, R, ID>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    ID: Fn() -> T::Inner + Sync,
    T: Bubble + Send,
{
    fn split_off_left(&self) -> Self {
        *self
    }

    fn to_reducer(&self) -> Self::Reducer {
        *self
    }
}

impl<'r, R, ID, T> Reducer<T> for TryReduceConsumer<'r, R, ID>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    T: Bubble,
{
    fn reduce(self, left: T, right: T) -> T {
        let reduce_op = self.reduce_op;
        match left.bubble() {
            Continue(left) => match right.bubble() {
                Continue(right) => reduce_op(left, right),
                Break(right) => right,
            },
            Break(left) => left,
        }
    }
}

struct TryReduceFolder<'r, R: 'r, T: Bubble> {
    reduce_op: &'r R,
    control_flow: ControlFlow<T::Inner, T>,
    full: &'r AtomicBool,
}

impl<'r, R, T> Folder<T> for TryReduceFolder<'r, R, T>
where
    R: Fn(T::Inner, T::Inner) -> T,
    T: Bubble,
{
    type Result = T;

    fn consume(self, item: T) -> Self {
        let reduce_op = self.reduce_op;
        let control_flow = match self.control_flow {
            Continue(left) => match item.bubble() {
                Continue(right) => reduce_op(left, right).bubble(),
                Break(right) => Break(right),
            },
            Break(left) => Break(left),
        };
        if let Break(_) = control_flow {
            self.full.store(true, Ordering::Relaxed)
        }
        TryReduceFolder {
            control_flow: control_flow,
            ..self
        }
    }

    fn complete(self) -> T {
        self.control_flow.unbubble()
    }

    fn full(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }
}
