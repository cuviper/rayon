
pub trait ThreadableFn<Args, Output> {
    fn with_threaded<CB>(self, callback: CB) -> CB::Output
        where CB: ThreadedCallback<Args, Output>;

    private_decl!{}
}

pub trait ThreadedCallback<Args, Output> {
    type Output;

    fn callback<F>(self, threaded: F) -> Self::Output
        where F: ThreadedFn<Args, Output>;

    private_decl!{}
}

pub trait ThreadedFn<Args, Output>: Send + Sized {
    fn split_off_left(&self) -> Self;

    fn call(&mut self, args: Args) -> Output;

    private_decl!{}
}

pub struct ThreadableWith<F, T> {
    op: F,
    value: T,
}

struct ThreadedWith<'f, F: 'f, T> {
    op: &'f F,
    value: T,
}

pub fn threaded<F, T, U, R>(value: T, op: F) -> ThreadableWith<F, T>
where F: Fn(&mut T, U) -> R + Sync,
      T: Clone + Send
{
    ThreadableWith {
        op: op,
        value: value,
    }
}


macro_rules! imp {
    ( $( $n:ident ),* ) => {
        impl<F, $($n,)* R> ThreadableFn<($($n,)*), R> for F
            where F: Fn($($n),*) -> R + Sync
        {
            fn with_threaded<CB>(self, callback: CB) -> CB::Output
                where CB: ThreadedCallback<($($n,)*), R>
            {
                callback.callback(&self)
            }

            private_impl!{}
        }

        impl<'a, F, $($n,)* R> ThreadedFn<($($n,)*), R> for &'a F
            where F: Fn($($n),*) -> R + Sync
        {
            fn split_off_left(&self) -> Self {
                *self
            }

            fn call(&mut self, ($($n,)*): ($($n,)*)) -> R {
                (*self)($($n),*)
            }

            private_impl!{}
        }

        impl<F, A, $($n,)* R> ThreadableFn<($($n,)*), R> for ThreadableWith<F, A>
            where F: Fn(&mut A $(, $n)*) -> R + Sync,
                  A: Clone + Send
        {
            fn with_threaded<CB>(self, callback: CB) -> CB::Output
                where CB: ThreadedCallback<($($n,)*), R>
            {
                let threaded = ThreadedWith {
                    op: &self.op,
                    value: self.value,
                };
                callback.callback(threaded)
            }

            private_impl!{}
        }

        impl<'f, F, A, $($n,)* R> ThreadedFn<($($n,)*), R> for ThreadedWith<'f, F, A>
            where F: Fn(&mut A $(, $n)*) -> R + Sync,
                  A: Clone + Send
        {
            fn split_off_left(&self) -> Self {
                ThreadedWith { value: self.value.clone(), ..*self }
            }

            fn call(&mut self, ($($n,)*): ($($n,)*)) -> R {
                (*self.op)(&mut self.value, $($n),*)
            }

            private_impl!{}
        }
    }
}

imp!{}
imp!{T}
imp!{T, U}
