use std::fmt;
use std::sync::Arc;

use crossbeam_deque::{Injector, Steal};

use crate::job::{Job, JobRef};
use crate::registry::Registry;

use super::{ScopeBase, get_in_place_thread_registry};

/// Execute `init` to push initial work, then execute `op` for each work item.
/// The `op` can push additional work as needed.
///
/// ```
/// # use rayon_core as rayon;
/// rayon::scope_for_each(
///     |scope| {
///         for i in 1..10 {
///             scope.push(i);
///         }
///     },
///     |i, scope| {
///         if dbg!(i) < 100 {
///             scope.push(10 * i);
///         }
///     },
/// );
/// ```
///
/// ```
/// # use rayon_core as rayon;
/// use std::fs;
/// use std::path::PathBuf;
/// use std::sync::atomic::{AtomicU64, Ordering};
///
/// fn main() {
///     let total = AtomicU64::new(0);
///     rayon::scope_for_each(
///         |scope| scope.push(PathBuf::from(".")),
///         |dir, scope| {
///             // Silently ignore filesystem errors
///             let Ok(read_dir) = fs::read_dir(dir) else { return };
///             for entry in read_dir.filter_map(Result::ok) {
///                 let Ok(metadata) = entry.metadata() else { continue };
///                 if metadata.is_file() {
///                     // Accumulate the sum of all file sizes
///                     total.fetch_add(metadata.len(), Ordering::Relaxed);
///                 } else if metadata.is_dir() {
///                     // Enqueue the subdirectory for parallel traversal
///                     scope.push(entry.path());
///                 }
///             }
///         },
///     );
///     println!("total size is {}", total.into_inner());
/// }
/// ```
pub fn scope_for_each<'scope, T, INIT, OP, R>(init: INIT, op: OP) -> R
where
    T: Send + 'scope,
    INIT: FnOnce(&ScopeForEach<'scope, T>) -> R,
    OP: Fn(T, &ScopeForEach<'scope, T>) + Sync + 'scope,
{
    ScopeForEach::with_registry(None, init, op)
}

/// Represents a scope which can be used to uniformly process any number of items.
/// See [`scope_for_each()`] for more information.
pub struct ScopeForEach<'scope, T> {
    base: ScopeBase<'scope>,
    items: Injector<T>,
    op: Box<dyn Fn(T, &Self) + Sync + 'scope>,
}

impl<'scope, T> fmt::Debug for ScopeForEach<'scope, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("ScopeForEach")
            .field("queued_items", &self.items.len())
            .field("pool_id", &self.base.registry.id())
            .field("panic", &self.base.panic)
            .field("job_completed_latch", &self.base.job_completed_latch)
            .finish()
    }
}

impl<'scope, T: Send + 'scope> ScopeForEach<'scope, T> {
    pub(crate) fn with_registry<INIT, OP, R>(
        registry: Option<&Arc<Registry>>,
        init: INIT,
        op: OP,
    ) -> R
    where
        INIT: FnOnce(&ScopeForEach<'scope, T>) -> R,
        OP: Fn(T, &ScopeForEach<'scope, T>) + Sync + 'scope,
    {
        let (thread, registry) = get_in_place_thread_registry(registry);
        let scope = Self {
            base: ScopeBase::new(thread, registry),
            items: Injector::new(),
            op: Box::new(op),
        };
        scope.base.complete(thread, || init(&scope))
    }

    /// Push an item into the scope's queue.
    pub fn push(&self, item: T) {
        self.items.push(item);
        // SAFETY: incrementing the latch ensures the scope will wait in
        // `ScopeBase::complete` until this job decrements it again.
        let job_ref = unsafe {
            self.base.job_completed_latch.increment();
            JobRef::new(self as *const Self)
        };
        self.base.registry.inject_or_push(job_ref);
    }
}

impl<'scope, T: Send + 'scope> Job for ScopeForEach<'scope, T> {
    unsafe fn execute(this: *const ()) {
        // SAFETY: the scope will wait on its latch until this job decrements
        // the counter at the end of `ScopeBase::execute_job`.
        unsafe {
            let scope = &*(this as *const Self);
            let item = loop {
                match scope.items.steal() {
                    Steal::Success(item) => break item,
                    Steal::Empty => panic!("no item for job"),
                    Steal::Retry => std::hint::spin_loop(),
                }
            };
            let op = &*scope.op;
            ScopeBase::execute_job(&scope.base, move || op(item, scope))
        }
    }
}
