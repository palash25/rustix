//! The `Errno` type, which is a minimal wrapper around an error code.
//!
//! We define the error constants as individual `const`s instead of an
//! enum because we may not know about all of the host's error values
//! and we don't want unrecognized values to create UB.

use crate::imp;
use core::{fmt, result};
#[cfg(feature = "std")]
use std::error;

/// A specialized `Result` type for `rustix` APIs.
pub type Result<T> = result::Result<T, Errno>;

/// `errno`—An error code.
///
/// The error type for `rustix` APIs. This is similar to `std::io::Error`, but
/// only holds an OS error code, and no extra error value.
///
/// # References
///  - [POSIX]
///  - [Linux]
///
/// [POSIX]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/errno.html
/// [Linux]: https://man7.org/linux/man-pages/man3/errno.3.html
pub use imp::io::errno::Errno;

impl Errno {
    /// Shorthand for `std::io::Error::from(self).kind()`.
    #[cfg(feature = "std")]
    #[inline]
    pub fn kind(self) -> std::io::ErrorKind {
        std::io::Error::from(self).kind()
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            std::io::Error::from(*self).fmt(fmt)
        }
        #[cfg(not(feature = "std"))]
        {
            write!(fmt, "os error {}", self.raw_os_error())
        }
    }
}

impl fmt::Debug for Errno {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "std")]
        {
            std::io::Error::from(*self).fmt(fmt)
        }
        #[cfg(not(feature = "std"))]
        {
            write!(fmt, "os error {}", self.raw_os_error())
        }
    }
}

#[cfg(feature = "std")]
impl error::Error for Errno {}

#[cfg(feature = "std")]
impl From<Errno> for std::io::Error {
    #[inline]
    fn from(err: Errno) -> Self {
        Self::from_raw_os_error(err.raw_os_error() as _)
    }
}

/// Call `f` until it either succeeds or fails other than [`Errno::INTR`].
#[inline]
pub fn with_retrying<T, F: FnMut() -> Result<T>>(mut f: F) -> Result<T> {
    loop {
        match f() {
            Err(Errno::INTR) => (),
            result => return result,
        }
    }
}
