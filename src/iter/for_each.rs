use super::ParallelIterator;
use super::internal::*;
use super::noop::*;
use super::threadable::*;

pub fn for_each<I, F, T>(pi: I, op: F)
    where I: ParallelIterator<Item = T>,
          F: ThreadableFn<(T,), ()>,
          T: Send
{
    return op.with_threaded(Callback { pi: pi });

    struct Callback<I> {
        pi: I,
    }

    impl<T, I> ThreadedCallback<(T,), ()> for Callback<I>
        where I: ParallelIterator<Item = T>,
              T: Send
    {
        type Output = ();

        fn callback<F>(self, op: F)
            where F: ThreadedFn<(T,), ()>
        {
            use super::ParallelIterator;

            let consumer = ForEachConsumer { op: op };
            self.pi.drive_unindexed(consumer)
        }

        private_impl!{}
    }
}

struct ForEachConsumer<F> {
    op: F,
}

impl<F, T> Consumer<T> for ForEachConsumer<F>
    where F: ThreadedFn<(T,), ()>
{
    type Folder = Self;
    type Reducer = NoopReducer;
    type Result = ();

    fn split_at(self, _index: usize) -> (Self, Self, NoopReducer) {
        (self.split_off_left(), self, NoopReducer)
    }

    fn into_folder(self) -> Self {
        self
    }

    fn full(&self) -> bool {
        false
    }
}

impl<F, T> Folder<T> for ForEachConsumer<F>
    where F: ThreadedFn<(T,), ()>
{
    type Result = ();

    fn consume(mut self, item: T) -> Self {
        self.op.call((item,));
        self
    }

    fn complete(self) {}

    fn full(&self) -> bool {
        false
    }
}

impl<F, T> UnindexedConsumer<T> for ForEachConsumer<F>
    where F: ThreadedFn<(T,), ()>
{
    fn split_off_left(&self) -> Self {
        ForEachConsumer { op: self.op.split_off_left() }
    }

    fn to_reducer(&self) -> NoopReducer {
        NoopReducer
    }
}
