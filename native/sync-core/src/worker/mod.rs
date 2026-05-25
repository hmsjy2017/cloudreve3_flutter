mod worker_impl;
mod pool;

pub use worker_impl::Worker;
#[cfg(feature = "windows-cfapi")]
pub use worker_impl::PlaceholderCreator;
pub use pool::WorkerPool;
