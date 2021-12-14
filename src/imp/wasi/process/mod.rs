mod types;

pub(crate) mod syscalls;

pub use types::{EXIT_FAILURE, EXIT_SUCCESS};

#[inline]
pub(crate) fn page_size() -> usize {
    // WebAssembly pages are always this size.
    65536
}
