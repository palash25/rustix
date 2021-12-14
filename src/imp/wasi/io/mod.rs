mod error;
mod poll_fd;

pub(crate) mod syscalls;

pub use error::Error;
pub use poll_fd::{PollFd, PollFlags};

pub(crate) const AT_FDCWD: i32 = -2;
pub(crate) const STDIN_FILENO: i32 = 0;
pub(crate) const STDOUT_FILENO: i32 = 1;
pub(crate) const STDERR_FILENO: i32 = 2;
