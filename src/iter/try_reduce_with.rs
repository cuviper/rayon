use super::plumbing::*;
use super::private::ControlFlow::*;
use super::private::{Bubble, ControlFlow};
use super::ParallelIterator;

use std::sync::atomic::{AtomicBool, Ordering};

pub fn try_reduce_with<PI, R, T>(pi: PI, reduce_op: R) -> Option<T>
where
    PI: ParallelIterator<Item = T>,
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    T: Bubble + Send,
{
    let full = AtomicBool::new(false);
    let consumer = TryReduceWithConsumer {
        reduce_op: &reduce_op,
        full: &full,
    };
    pi.drive_unindexed(consumer)
}

struct TryReduceWithConsumer<'r, R: 'r> {
    reduce_op: &'r R,
    full: &'r AtomicBool,
}

impl<'r, R> Copy for TryReduceWithConsumer<'r, R> {}

impl<'r, R> Clone for TryReduceWithConsumer<'r, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'r, R, T> Consumer<T> for TryReduceWithConsumer<'r, R>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    T: Bubble + Send,
{
    type Folder = TryReduceWithFolder<'r, R, T>;
    type Reducer = Self;
    type Result = Option<T>;

    fn split_at(self, _index: usize) -> (Self, Self, Self) {
        (self, self, self)
    }

    fn into_folder(self) -> Self::Folder {
        TryReduceWithFolder {
            reduce_op: self.reduce_op,
            opt_control_flow: None,
            full: self.full,
        }
    }

    fn full(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }
}

impl<'r, R, T> UnindexedConsumer<T> for TryReduceWithConsumer<'r, R>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    T: Bubble + Send,
{
    fn split_off_left(&self) -> Self {
        *self
    }

    fn to_reducer(&self) -> Self::Reducer {
        *self
    }
}

impl<'r, R, T> Reducer<Option<T>> for TryReduceWithConsumer<'r, R>
where
    R: Fn(T::Inner, T::Inner) -> T + Sync,
    T: Bubble,
{
    fn reduce(self, left: Option<T>, right: Option<T>) -> Option<T> {
        let reduce_op = self.reduce_op;
        match (left, right) {
            (Some(left), Some(right)) => match left.bubble() {
                Continue(left) => match right.bubble() {
                    Continue(right) => Some(reduce_op(left, right)),
                    Break(right) => Some(right),
                },
                Break(left) => Some(left),
            },
            (None, value) | (value, None) => value,
        }
    }
}

struct TryReduceWithFolder<'r, R: 'r, T: Bubble> {
    reduce_op: &'r R,
    opt_control_flow: Option<ControlFlow<T::Inner, T>>,
    full: &'r AtomicBool,
}

impl<'r, R, T> Folder<T> for TryReduceWithFolder<'r, R, T>
where
    R: Fn(T::Inner, T::Inner) -> T,
    T: Bubble,
{
    type Result = Option<T>;

    fn consume(self, item: T) -> Self {
        let reduce_op = self.reduce_op;
        let control_flow = match self.opt_control_flow {
            Some(Continue(left)) => match item.bubble() {
                Continue(right) => reduce_op(left, right).bubble(),
                Break(right) => Break(right),
            },
            Some(Break(left)) => Break(left),
            None => item.bubble(),
        };
        if let Break(_) = control_flow {
            self.full.store(true, Ordering::Relaxed)
        }
        TryReduceWithFolder {
            opt_control_flow: Some(control_flow),
            ..self
        }
    }

    fn complete(self) -> Option<T> {
        self.opt_control_flow.map(ControlFlow::unbubble)
    }

    fn full(&self) -> bool {
        self.full.load(Ordering::Relaxed)
    }
}
