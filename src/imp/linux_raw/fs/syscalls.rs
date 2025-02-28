//! linux_raw syscalls supporting `rustix::fs`.
//!
//! # Safety
//!
//! See the `rustix::imp` module documentation for details.
#![allow(unsafe_code)]
#![allow(dead_code)]
#![allow(clippy::undocumented_unsafe_blocks)]

use super::super::c;
#[cfg(all(
    target_pointer_width = "32",
    any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc")
))]
use super::super::conv::zero;
use super::super::conv::{
    by_ref, c_int, c_uint, dev_t, oflags_for_open_how, opt_mut, pass_usize, raw_fd, ret, ret_c_int,
    ret_c_uint, ret_owned_fd, ret_usize, size_of, slice_mut,
};
#[cfg(target_pointer_width = "64")]
use super::super::conv::{loff_t, loff_t_from_u64, ret_u64};
#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "mips64",
    target_pointer_width = "32"
))]
use crate::fd::AsFd;
use crate::fd::{BorrowedFd, RawFd};
use crate::ffi::CStr;
use crate::fs::{
    Access, Advice, AtFlags, FallocateFlags, FdFlags, FileType, FlockOperation, MemfdFlags, Mode,
    OFlags, RenameFlags, ResolveFlags, SealFlags, Stat, StatFs, StatxFlags, Timestamps,
};
use crate::io::{self, OwnedFd, SeekFrom};
use crate::process::{Gid, Uid};
use core::convert::TryInto;
use core::mem::MaybeUninit;
#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
use core::sync::atomic::{AtomicBool, Ordering::Relaxed};
#[cfg(target_arch = "mips64")]
use linux_raw_sys::general::stat as linux_stat64;
use linux_raw_sys::general::{
    __kernel_timespec, open_how, statx, AT_FDCWD, AT_REMOVEDIR, AT_SYMLINK_NOFOLLOW, F_ADD_SEALS,
    F_DUPFD, F_DUPFD_CLOEXEC, F_GETFD, F_GETFL, F_GETLEASE, F_GETOWN, F_GETPIPE_SZ, F_GETSIG,
    F_GET_SEALS, F_SETFD, F_SETFL, F_SETPIPE_SZ,
};
#[cfg(target_pointer_width = "32")]
use {
    super::super::conv::{hi, lo, slice_just_addr},
    linux_raw_sys::general::stat64 as linux_stat64,
    linux_raw_sys::general::timespec as __kernel_old_timespec,
};

#[inline]
pub(crate) fn open(filename: &CStr, flags: OFlags, mode: Mode) -> io::Result<OwnedFd> {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    {
        openat(crate::fs::cwd().as_fd(), filename, flags, mode)
    }
    #[cfg(all(
        target_pointer_width = "32",
        not(any(target_arch = "aarch64", target_arch = "riscv64"))
    ))]
    unsafe {
        ret_owned_fd(syscall_readonly!(__NR_open, filename, flags, mode))
    }
    #[cfg(all(
        target_pointer_width = "64",
        not(any(target_arch = "aarch64", target_arch = "riscv64"))
    ))]
    unsafe {
        ret_owned_fd(syscall_readonly!(__NR_open, filename, flags, mode))
    }
}

#[inline]
pub(crate) fn openat(
    dirfd: BorrowedFd<'_>,
    filename: &CStr,
    flags: OFlags,
    mode: Mode,
) -> io::Result<OwnedFd> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_owned_fd(syscall_readonly!(__NR_openat, dirfd, filename, flags, mode))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_owned_fd(syscall_readonly!(__NR_openat, dirfd, filename, flags, mode))
    }
}

#[inline]
pub(crate) fn openat2(
    dirfd: BorrowedFd<'_>,
    pathname: &CStr,
    flags: OFlags,
    mode: Mode,
    resolve: ResolveFlags,
) -> io::Result<OwnedFd> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_openat2,
            dirfd,
            pathname,
            by_ref(&open_how {
                flags: oflags_for_open_how(flags),
                mode: u64::from(mode.bits()),
                resolve: resolve.bits(),
            }),
            size_of::<open_how, _>()
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_openat2,
            dirfd,
            pathname,
            by_ref(&open_how {
                flags: oflags_for_open_how(flags),
                mode: u64::from(mode.bits()),
                resolve: resolve.bits(),
            }),
            size_of::<open_how, _>()
        ))
    }
}

#[inline]
pub(crate) fn chmod(filename: &CStr, mode: Mode) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_fchmodat,
            raw_fd(AT_FDCWD),
            filename,
            mode
        ))
    }
}

#[inline]
pub(crate) fn chmodat(dirfd: BorrowedFd<'_>, filename: &CStr, mode: Mode) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_fchmodat, dirfd, filename, mode)) }
}

#[inline]
pub(crate) fn fchmod(fd: BorrowedFd<'_>, mode: Mode) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_fchmod, fd, mode)) }
}

#[inline]
pub(crate) fn chownat(
    dirfd: BorrowedFd<'_>,
    filename: &CStr,
    owner: Uid,
    group: Gid,
    flags: AtFlags,
) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_fchownat,
            dirfd,
            filename,
            c_uint(owner.as_raw()),
            c_uint(group.as_raw()),
            flags
        ))
    }
}

#[inline]
pub(crate) fn fchown(fd: BorrowedFd<'_>, owner: Uid, group: Gid) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_fchown,
            fd,
            c_uint(owner.as_raw()),
            c_uint(group.as_raw())
        ))
    }
}

#[inline]
pub(crate) fn mknodat(
    dirfd: BorrowedFd<'_>,
    filename: &CStr,
    file_type: FileType,
    mode: Mode,
    dev: u64,
) -> io::Result<()> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall_readonly!(
            __NR_mknodat,
            dirfd,
            filename,
            (mode, file_type),
            dev_t(dev)?
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_mknodat,
            dirfd,
            filename,
            (mode, file_type),
            dev_t(dev)
        ))
    }
}

#[inline]
pub(crate) fn seek(fd: BorrowedFd<'_>, pos: SeekFrom) -> io::Result<u64> {
    let (whence, offset) = match pos {
        SeekFrom::Start(pos) => {
            let pos: u64 = pos;
            // Silently cast; we'll get `EINVAL` if the value is negative.
            (linux_raw_sys::general::SEEK_SET, pos as i64)
        }
        SeekFrom::End(offset) => (linux_raw_sys::general::SEEK_END, offset),
        SeekFrom::Current(offset) => (linux_raw_sys::general::SEEK_CUR, offset),
    };
    _seek(fd, offset, whence)
}

#[inline]
pub(crate) fn _seek(fd: BorrowedFd<'_>, offset: i64, whence: c::c_uint) -> io::Result<u64> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        let mut result = MaybeUninit::<u64>::uninit();
        ret(syscall!(
            __NR__llseek,
            fd,
            // Don't use the hi/lo functions here because Linux's llseek
            // takes its 64-bit argument differently from everything else.
            pass_usize((offset >> 32) as usize),
            pass_usize(offset as usize),
            &mut result,
            c_uint(whence)
        ))
        .map(|()| result.assume_init())
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_u64(syscall_readonly!(
            __NR_lseek,
            fd,
            loff_t(offset),
            c_uint(whence)
        ))
    }
}

#[inline]
pub(crate) fn tell(fd: BorrowedFd<'_>) -> io::Result<u64> {
    _seek(fd, 0, linux_raw_sys::general::SEEK_CUR).map(|x| x as u64)
}

#[inline]
pub(crate) fn ftruncate(fd: BorrowedFd<'_>, length: u64) -> io::Result<()> {
    // <https://github.com/torvalds/linux/blob/fcadab740480e0e0e9fa9bd272acd409884d431a/arch/arm64/kernel/sys32.c#L81-L83>
    #[cfg(all(
        target_pointer_width = "32",
        any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc")
    ))]
    unsafe {
        ret(syscall_readonly!(
            __NR_ftruncate64,
            fd,
            zero(),
            hi(length),
            lo(length)
        ))
    }
    #[cfg(all(
        target_pointer_width = "32",
        not(any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc"))
    ))]
    unsafe {
        ret(syscall_readonly!(
            __NR_ftruncate64,
            fd,
            hi(length),
            lo(length)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_ftruncate,
            fd,
            loff_t_from_u64(length)
        ))
    }
}

#[inline]
pub(crate) fn fallocate(
    fd: BorrowedFd<'_>,
    mode: FallocateFlags,
    offset: u64,
    len: u64,
) -> io::Result<()> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fallocate,
            fd,
            mode,
            hi(offset),
            lo(offset),
            hi(len),
            lo(len)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fallocate,
            fd,
            mode,
            loff_t_from_u64(offset),
            loff_t_from_u64(len)
        ))
    }
}

#[inline]
pub(crate) fn fadvise(fd: BorrowedFd<'_>, pos: u64, len: u64, advice: Advice) -> io::Result<()> {
    // On ARM, the arguments are reordered so that the len and pos argument
    // pairs are aligned. And ARM has a custom syscall code for this.
    #[cfg(target_arch = "arm")]
    unsafe {
        ret(syscall_readonly!(
            __NR_arm_fadvise64_64,
            fd,
            advice,
            hi(pos),
            lo(pos),
            hi(len),
            lo(len)
        ))
    }

    // On powerpc, the arguments are reordered as on ARM.
    #[cfg(target_arch = "powerpc")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fadvise64_64,
            fd,
            advice,
            hi(pos),
            lo(pos),
            hi(len),
            lo(len)
        ))
    }
    // On mips, the arguments are not reordered, and padding is inserted
    // instead to ensure alignment.
    #[cfg(target_arch = "mips")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fadvise64,
            fd,
            zero(),
            hi(pos),
            lo(pos),
            hi(len),
            lo(len),
            advice
        ))
    }
    #[cfg(all(
        target_pointer_width = "32",
        not(any(target_arch = "arm", target_arch = "mips", target_arch = "powerpc"))
    ))]
    unsafe {
        ret(syscall_readonly!(
            __NR_fadvise64_64,
            fd,
            hi(pos),
            lo(pos),
            hi(len),
            lo(len),
            advice
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fadvise64,
            fd,
            loff_t_from_u64(pos),
            loff_t_from_u64(len),
            advice
        ))
    }
}

#[inline]
pub(crate) fn fsync(fd: BorrowedFd<'_>) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_fsync, fd)) }
}

#[inline]
pub(crate) fn fdatasync(fd: BorrowedFd<'_>) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_fdatasync, fd)) }
}

#[inline]
pub(crate) fn flock(fd: BorrowedFd<'_>, operation: FlockOperation) -> io::Result<()> {
    unsafe { ret(syscall!(__NR_flock, fd, c_uint(operation as c::c_uint))) }
}

/// `statx` was introduced in Linux 4.11.
#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
static NO_STATX: AtomicBool = AtomicBool::new(false);

#[inline]
pub(crate) fn fstat(fd: BorrowedFd<'_>) -> io::Result<Stat> {
    #[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
    {
        if !NO_STATX.load(Relaxed) {
            match statx(fd, cstr!(""), AtFlags::EMPTY_PATH, StatxFlags::ALL) {
                Ok(x) => return statx_to_stat(x),
                Err(io::Errno::NOSYS) => NO_STATX.store(true, Relaxed),
                Err(e) => return Err(e),
            }
        }
        fstat_old(fd)
    }

    #[cfg(all(target_pointer_width = "64", not(target_arch = "mips64")))]
    unsafe {
        let mut result = MaybeUninit::<Stat>::uninit();
        ret(syscall!(__NR_fstat, fd, &mut result)).map(|()| result.assume_init())
    }
}

#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
fn fstat_old(fd: BorrowedFd<'_>) -> io::Result<Stat> {
    let mut result = MaybeUninit::<linux_stat64>::uninit();

    #[cfg(target_arch = "mips64")]
    unsafe {
        ret(syscall!(__NR_fstat, fd, &mut result))?;
        stat_to_stat(result.assume_init())
    }

    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall!(__NR_fstat64, fd, &mut result))?;
        stat_to_stat(result.assume_init())
    }
}

#[inline]
pub(crate) fn stat(filename: &CStr) -> io::Result<Stat> {
    #[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
    {
        if !NO_STATX.load(Relaxed) {
            match statx(
                crate::fs::cwd().as_fd(),
                filename,
                AtFlags::empty(),
                StatxFlags::ALL,
            ) {
                Ok(x) => return statx_to_stat(x),
                Err(io::Errno::NOSYS) => NO_STATX.store(true, Relaxed),
                Err(e) => return Err(e),
            }
        }
        stat_old(filename)
    }

    #[cfg(all(target_pointer_width = "64", not(target_arch = "mips64")))]
    unsafe {
        let mut result = MaybeUninit::<Stat>::uninit();
        ret(syscall!(
            __NR_newfstatat,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(0)
        ))
        .map(|()| result.assume_init())
    }
}

#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
fn stat_old(filename: &CStr) -> io::Result<Stat> {
    let mut result = MaybeUninit::<linux_stat64>::uninit();

    #[cfg(target_arch = "mips64")]
    unsafe {
        ret(syscall!(
            __NR_newfstatat,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(0)
        ))?;
        stat_to_stat(result.assume_init())
    }

    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall!(
            __NR_fstatat64,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(0)
        ))?;
        stat_to_stat(result.assume_init())
    }
}

#[inline]
pub(crate) fn statat(dirfd: BorrowedFd<'_>, filename: &CStr, flags: AtFlags) -> io::Result<Stat> {
    #[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
    {
        if !NO_STATX.load(Relaxed) {
            match statx(dirfd, filename, flags, StatxFlags::ALL) {
                Ok(x) => return statx_to_stat(x),
                Err(io::Errno::NOSYS) => NO_STATX.store(true, Relaxed),
                Err(e) => return Err(e),
            }
        }
        statat_old(dirfd, filename, flags)
    }

    #[cfg(all(target_pointer_width = "64", not(target_arch = "mips64")))]
    unsafe {
        let mut result = MaybeUninit::<Stat>::uninit();
        ret(syscall!(
            __NR_newfstatat,
            dirfd,
            filename,
            &mut result,
            flags
        ))
        .map(|()| result.assume_init())
    }
}

#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
fn statat_old(dirfd: BorrowedFd<'_>, filename: &CStr, flags: AtFlags) -> io::Result<Stat> {
    let mut result = MaybeUninit::<linux_stat64>::uninit();

    #[cfg(target_arch = "mips64")]
    unsafe {
        ret(syscall!(
            __NR_newfstatat,
            dirfd,
            filename,
            &mut result,
            flags
        ))?;
        stat_to_stat(result.assume_init())
    }

    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall!(
            __NR_fstatat64,
            dirfd,
            filename,
            &mut result,
            flags
        ))?;
        stat_to_stat(result.assume_init())
    }
}

#[inline]
pub(crate) fn lstat(filename: &CStr) -> io::Result<Stat> {
    #[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
    {
        if !NO_STATX.load(Relaxed) {
            match statx(
                crate::fs::cwd().as_fd(),
                filename,
                AtFlags::SYMLINK_NOFOLLOW,
                StatxFlags::ALL,
            ) {
                Ok(x) => return statx_to_stat(x),
                Err(io::Errno::NOSYS) => NO_STATX.store(true, Relaxed),
                Err(e) => return Err(e),
            }
        }
        lstat_old(filename)
    }

    #[cfg(all(target_pointer_width = "64", not(target_arch = "mips64")))]
    unsafe {
        let mut result = MaybeUninit::<Stat>::uninit();
        ret(syscall!(
            __NR_newfstatat,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(AT_SYMLINK_NOFOLLOW)
        ))
        .map(|()| result.assume_init())
    }
}

#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
fn lstat_old(filename: &CStr) -> io::Result<Stat> {
    let mut result = MaybeUninit::<linux_stat64>::uninit();

    #[cfg(target_arch = "mips64")]
    unsafe {
        ret(syscall!(
            __NR_newfstatat,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(AT_SYMLINK_NOFOLLOW)
        ))?;
        stat_to_stat(result.assume_init())
    }

    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall!(
            __NR_fstatat64,
            raw_fd(AT_FDCWD),
            filename,
            &mut result,
            c_uint(AT_SYMLINK_NOFOLLOW)
        ))?;
        stat_to_stat(result.assume_init())
    }
}

/// Convert from a Linux `statx` value to rustix's `Stat`.
#[cfg(any(target_pointer_width = "32", target_arch = "mips64"))]
fn statx_to_stat(x: crate::fs::Statx) -> io::Result<Stat> {
    Ok(Stat {
        st_dev: crate::fs::makedev(x.stx_dev_major, x.stx_dev_minor),
        st_mode: x.stx_mode.into(),
        st_nlink: x.stx_nlink.into(),
        st_uid: x.stx_uid.into(),
        st_gid: x.stx_gid.into(),
        st_rdev: crate::fs::makedev(x.stx_rdev_major, x.stx_rdev_minor),
        st_size: x.stx_size.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_blksize: x.stx_blksize.into(),
        st_blocks: x.stx_blocks.into(),
        st_atime: x
            .stx_atime
            .tv_sec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_atime_nsec: x.stx_atime.tv_nsec.into(),
        st_mtime: x
            .stx_mtime
            .tv_sec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_mtime_nsec: x.stx_mtime.tv_nsec.into(),
        st_ctime: x
            .stx_ctime
            .tv_sec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_ctime_nsec: x.stx_ctime.tv_nsec.into(),
        st_ino: x.stx_ino.into(),
    })
}

/// Convert from a Linux `stat64` value to rustix's `Stat`.
#[cfg(target_pointer_width = "32")]
fn stat_to_stat(s64: linux_raw_sys::general::stat64) -> io::Result<Stat> {
    Ok(Stat {
        st_dev: s64.st_dev.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_mode: s64.st_mode.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_nlink: s64.st_nlink.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_uid: s64.st_uid.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_gid: s64.st_gid.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_rdev: s64.st_rdev.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_size: s64.st_size.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_blksize: s64.st_blksize.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_blocks: s64.st_blocks.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_atime: s64.st_atime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_atime_nsec: s64
            .st_atime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_mtime: s64.st_mtime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_mtime_nsec: s64
            .st_mtime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_ctime: s64.st_ctime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_ctime_nsec: s64
            .st_ctime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_ino: s64.st_ino.try_into().map_err(|_| io::Errno::OVERFLOW)?,
    })
}

/// Convert from a Linux `stat` value to rustix's `Stat`.
#[cfg(target_arch = "mips64")]
fn stat_to_stat(s: linux_raw_sys::general::stat) -> io::Result<Stat> {
    Ok(Stat {
        st_dev: s.st_dev.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_mode: s.st_mode.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_nlink: s.st_nlink.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_uid: s.st_uid.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_gid: s.st_gid.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_rdev: s.st_rdev.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_size: s.st_size.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_blksize: s.st_blksize.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_blocks: s.st_blocks.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_atime: s.st_atime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_atime_nsec: s
            .st_atime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_mtime: s.st_mtime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_mtime_nsec: s
            .st_mtime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_ctime: s.st_ctime.try_into().map_err(|_| io::Errno::OVERFLOW)?,
        st_ctime_nsec: s
            .st_ctime_nsec
            .try_into()
            .map_err(|_| io::Errno::OVERFLOW)?,
        st_ino: s.st_ino.try_into().map_err(|_| io::Errno::OVERFLOW)?,
    })
}

#[inline]
pub(crate) fn statx(
    dirfd: BorrowedFd<'_>,
    pathname: &CStr,
    flags: AtFlags,
    mask: StatxFlags,
) -> io::Result<statx> {
    unsafe {
        let mut statx_buf = MaybeUninit::<statx>::uninit();
        ret(syscall!(
            __NR_statx,
            dirfd,
            pathname,
            flags,
            mask,
            &mut statx_buf
        ))
        .map(|()| statx_buf.assume_init())
    }
}

#[inline]
pub(crate) fn fstatfs(fd: BorrowedFd<'_>) -> io::Result<StatFs> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        let mut result = MaybeUninit::<StatFs>::uninit();
        ret(syscall!(
            __NR_fstatfs64,
            fd,
            size_of::<StatFs, _>(),
            &mut result
        ))
        .map(|()| result.assume_init())
    }

    #[cfg(target_pointer_width = "64")]
    unsafe {
        let mut result = MaybeUninit::<StatFs>::uninit();
        ret(syscall!(__NR_fstatfs, fd, &mut result)).map(|()| result.assume_init())
    }
}

#[inline]
pub(crate) fn statfs(filename: &CStr) -> io::Result<StatFs> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        let mut result = MaybeUninit::<StatFs>::uninit();
        ret(syscall!(
            __NR_statfs64,
            filename,
            size_of::<StatFs, _>(),
            &mut result
        ))
        .map(|()| result.assume_init())
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        let mut result = MaybeUninit::<StatFs>::uninit();
        ret(syscall!(__NR_statfs, filename, &mut result)).map(|()| result.assume_init())
    }
}

#[inline]
pub(crate) fn readlink(path: &CStr, buf: &mut [u8]) -> io::Result<usize> {
    let (buf_addr_mut, buf_len) = slice_mut(buf);
    unsafe {
        ret_usize(syscall!(
            __NR_readlinkat,
            raw_fd(AT_FDCWD),
            path,
            buf_addr_mut,
            buf_len
        ))
    }
}

#[inline]
pub(crate) fn readlinkat(dirfd: BorrowedFd<'_>, path: &CStr, buf: &mut [u8]) -> io::Result<usize> {
    let (buf_addr_mut, buf_len) = slice_mut(buf);
    unsafe {
        ret_usize(syscall!(
            __NR_readlinkat,
            dirfd,
            path,
            buf_addr_mut,
            buf_len
        ))
    }
}

#[inline]
pub(crate) fn fcntl_dupfd(fd: BorrowedFd<'_>, min: RawFd) -> io::Result<OwnedFd> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_fcntl64,
            fd,
            c_uint(F_DUPFD),
            raw_fd(min)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_fcntl,
            fd,
            c_uint(F_DUPFD),
            raw_fd(min)
        ))
    }
}

#[inline]
pub(crate) fn fcntl_dupfd_cloexec(fd: BorrowedFd<'_>, min: RawFd) -> io::Result<OwnedFd> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_fcntl64,
            fd,
            c_uint(F_DUPFD_CLOEXEC),
            raw_fd(min)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_owned_fd(syscall_readonly!(
            __NR_fcntl,
            fd,
            c_uint(F_DUPFD_CLOEXEC),
            raw_fd(min)
        ))
    }
}

#[inline]
pub(crate) fn fcntl_getfd(fd: BorrowedFd<'_>) -> io::Result<FdFlags> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_uint(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETFD)))
            .map(FdFlags::from_bits_truncate)
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_uint(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETFD)))
            .map(FdFlags::from_bits_truncate)
    }
}

#[inline]
pub(crate) fn fcntl_setfd(fd: BorrowedFd<'_>, flags: FdFlags) -> io::Result<()> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_SETFD), flags))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(__NR_fcntl, fd, c_uint(F_SETFD), flags))
    }
}

#[inline]
pub(crate) fn fcntl_getfl(fd: BorrowedFd<'_>) -> io::Result<OFlags> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_uint(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETFL)))
            .map(OFlags::from_bits_truncate)
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_uint(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETFL)))
            .map(OFlags::from_bits_truncate)
    }
}

#[inline]
pub(crate) fn fcntl_setfl(fd: BorrowedFd<'_>, flags: OFlags) -> io::Result<()> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_SETFL), flags))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(__NR_fcntl, fd, c_uint(F_SETFL), flags))
    }
}

#[inline]
pub(crate) fn fcntl_getlease(fd: BorrowedFd<'_>) -> io::Result<c::c_int> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETLEASE)))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETLEASE)))
    }
}

#[inline]
pub(crate) fn fcntl_getown(fd: BorrowedFd<'_>) -> io::Result<c::c_int> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETOWN)))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETOWN)))
    }
}

#[inline]
pub(crate) fn fcntl_getsig(fd: BorrowedFd<'_>) -> io::Result<c::c_int> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETSIG)))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETSIG)))
    }
}

#[inline]
pub(crate) fn fcntl_getpipe_sz(fd: BorrowedFd<'_>) -> io::Result<usize> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_usize(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GETPIPE_SZ)))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_usize(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GETPIPE_SZ)))
    }
}

#[inline]
pub(crate) fn fcntl_setpipe_sz(fd: BorrowedFd<'_>, size: c::c_int) -> io::Result<usize> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_usize(syscall_readonly!(
            __NR_fcntl64,
            fd,
            c_uint(F_SETPIPE_SZ),
            c_int(size)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_usize(syscall_readonly!(
            __NR_fcntl,
            fd,
            c_uint(F_SETPIPE_SZ),
            c_int(size)
        ))
    }
}

#[inline]
pub(crate) fn fcntl_get_seals(fd: BorrowedFd<'_>) -> io::Result<SealFlags> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl64, fd, c_uint(F_GET_SEALS)))
            .map(|seals| SealFlags::from_bits_unchecked(seals as u32))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_c_int(syscall_readonly!(__NR_fcntl, fd, c_uint(F_GET_SEALS)))
            .map(|seals| SealFlags::from_bits_unchecked(seals as u32))
    }
}

#[inline]
pub(crate) fn fcntl_add_seals(fd: BorrowedFd<'_>, seals: SealFlags) -> io::Result<()> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fcntl64,
            fd,
            c_uint(F_ADD_SEALS),
            seals
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_fcntl,
            fd,
            c_uint(F_ADD_SEALS),
            seals
        ))
    }
}

#[inline]
pub(crate) fn rename(oldname: &CStr, newname: &CStr) -> io::Result<()> {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_renameat2,
            raw_fd(AT_FDCWD),
            oldname,
            raw_fd(AT_FDCWD),
            newname,
            c_uint(0)
        ))
    }
    #[cfg(not(target_arch = "riscv64"))]
    unsafe {
        ret(syscall_readonly!(
            __NR_renameat,
            raw_fd(AT_FDCWD),
            oldname,
            raw_fd(AT_FDCWD),
            newname
        ))
    }
}

#[inline]
pub(crate) fn renameat(
    old_dirfd: BorrowedFd<'_>,
    oldname: &CStr,
    new_dirfd: BorrowedFd<'_>,
    newname: &CStr,
) -> io::Result<()> {
    #[cfg(target_arch = "riscv64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_renameat2,
            old_dirfd,
            oldname,
            new_dirfd,
            newname,
            c_uint(0)
        ))
    }
    #[cfg(not(target_arch = "riscv64"))]
    unsafe {
        ret(syscall_readonly!(
            __NR_renameat,
            old_dirfd,
            oldname,
            new_dirfd,
            newname
        ))
    }
}

#[inline]
pub(crate) fn renameat2(
    old_dirfd: BorrowedFd<'_>,
    oldname: &CStr,
    new_dirfd: BorrowedFd<'_>,
    newname: &CStr,
    flags: RenameFlags,
) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_renameat2,
            old_dirfd,
            oldname,
            new_dirfd,
            newname,
            flags
        ))
    }
}

#[inline]
pub(crate) fn unlink(pathname: &CStr) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_unlinkat,
            raw_fd(AT_FDCWD),
            pathname,
            c_uint(0)
        ))
    }
}

#[inline]
pub(crate) fn unlinkat(dirfd: BorrowedFd<'_>, pathname: &CStr, flags: AtFlags) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_unlinkat, dirfd, pathname, flags)) }
}

#[inline]
pub(crate) fn rmdir(pathname: &CStr) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_unlinkat,
            raw_fd(AT_FDCWD),
            pathname,
            c_uint(AT_REMOVEDIR)
        ))
    }
}

#[inline]
pub(crate) fn link(oldname: &CStr, newname: &CStr) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_linkat,
            raw_fd(AT_FDCWD),
            oldname,
            raw_fd(AT_FDCWD),
            newname,
            c_uint(0)
        ))
    }
}

#[inline]
pub(crate) fn linkat(
    old_dirfd: BorrowedFd<'_>,
    oldname: &CStr,
    new_dirfd: BorrowedFd<'_>,
    newname: &CStr,
    flags: AtFlags,
) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_linkat,
            old_dirfd,
            oldname,
            new_dirfd,
            newname,
            flags
        ))
    }
}

#[inline]
pub(crate) fn symlink(oldname: &CStr, newname: &CStr) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_symlinkat,
            oldname,
            raw_fd(AT_FDCWD),
            newname
        ))
    }
}

#[inline]
pub(crate) fn symlinkat(oldname: &CStr, dirfd: BorrowedFd<'_>, newname: &CStr) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_symlinkat, oldname, dirfd, newname)) }
}

#[inline]
pub(crate) fn mkdir(pathname: &CStr, mode: Mode) -> io::Result<()> {
    unsafe {
        ret(syscall_readonly!(
            __NR_mkdirat,
            raw_fd(AT_FDCWD),
            pathname,
            mode
        ))
    }
}

#[inline]
pub(crate) fn mkdirat(dirfd: BorrowedFd<'_>, pathname: &CStr, mode: Mode) -> io::Result<()> {
    unsafe { ret(syscall_readonly!(__NR_mkdirat, dirfd, pathname, mode)) }
}

#[inline]
pub(crate) fn getdents(fd: BorrowedFd<'_>, dirent: &mut [u8]) -> io::Result<usize> {
    let (dirent_addr_mut, dirent_len) = slice_mut(dirent);

    unsafe { ret_usize(syscall!(__NR_getdents64, fd, dirent_addr_mut, dirent_len)) }
}

#[inline]
pub(crate) fn utimensat(
    dirfd: BorrowedFd<'_>,
    pathname: &CStr,
    times: &Timestamps,
    flags: AtFlags,
) -> io::Result<()> {
    _utimensat(dirfd, Some(pathname), times, flags)
}

#[inline]
fn _utimensat(
    dirfd: BorrowedFd<'_>,
    pathname: Option<&CStr>,
    times: &Timestamps,
    flags: AtFlags,
) -> io::Result<()> {
    // Assert that `Timestamps` has the expected layout.
    let _ = unsafe { core::mem::transmute::<Timestamps, [__kernel_timespec; 2]>(times.clone()) };

    #[cfg(target_pointer_width = "32")]
    unsafe {
        match ret(syscall_readonly!(
            __NR_utimensat_time64,
            dirfd,
            pathname,
            by_ref(times),
            flags
        )) {
            Err(io::Errno::NOSYS) => _utimensat_old(dirfd, pathname, times, flags),
            otherwise => otherwise,
        }
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret(syscall_readonly!(
            __NR_utimensat,
            dirfd,
            pathname,
            by_ref(times),
            flags
        ))
    }
}

#[cfg(target_pointer_width = "32")]
unsafe fn _utimensat_old(
    dirfd: BorrowedFd<'_>,
    pathname: Option<&CStr>,
    times: &Timestamps,
    flags: AtFlags,
) -> io::Result<()> {
    // See the comments in `rustix_clock_gettime_via_syscall` about
    // emulation.
    let old_times = [
        __kernel_old_timespec {
            tv_sec: times
                .last_access
                .tv_sec
                .try_into()
                .map_err(|_| io::Errno::INVAL)?,
            tv_nsec: times
                .last_access
                .tv_nsec
                .try_into()
                .map_err(|_| io::Errno::INVAL)?,
        },
        __kernel_old_timespec {
            tv_sec: times
                .last_modification
                .tv_sec
                .try_into()
                .map_err(|_| io::Errno::INVAL)?,
            tv_nsec: times
                .last_modification
                .tv_nsec
                .try_into()
                .map_err(|_| io::Errno::INVAL)?,
        },
    ];
    // The length of the array is fixed and not passed into the syscall.
    let old_times_addr = slice_just_addr(&old_times);
    ret(syscall_readonly!(
        __NR_utimensat,
        dirfd,
        pathname,
        old_times_addr,
        flags
    ))
}

#[inline]
pub(crate) fn futimens(fd: BorrowedFd<'_>, times: &Timestamps) -> io::Result<()> {
    _utimensat(fd, None, times, AtFlags::empty())
}

pub(crate) fn accessat(
    dirfd: BorrowedFd<'_>,
    path: &CStr,
    access: Access,
    flags: AtFlags,
) -> io::Result<()> {
    // Linux's `faccessat` doesn't have a flags parameter. If we have
    // `AT_EACCESS` and we're not setuid or setgid, we can emulate it.
    if flags.is_empty()
        || (flags.bits() == linux_raw_sys::general::AT_EACCESS
            && crate::process::getuid() == crate::process::geteuid()
            && crate::process::getgid() == crate::process::getegid())
    {
        return unsafe { ret(syscall_readonly!(__NR_faccessat, dirfd, path, access)) };
    }

    if flags.bits() != linux_raw_sys::general::AT_EACCESS {
        return Err(io::Errno::INVAL);
    }

    // TODO: Use faccessat2 in newer Linux versions.
    Err(io::Errno::NOSYS)
}

#[inline]
pub(crate) fn copy_file_range(
    fd_in: BorrowedFd<'_>,
    off_in: Option<&mut u64>,
    fd_out: BorrowedFd<'_>,
    off_out: Option<&mut u64>,
    len: u64,
) -> io::Result<u64> {
    let len: usize = len.try_into().unwrap_or(usize::MAX);
    _copy_file_range(fd_in, off_in, fd_out, off_out, len, 0).map(|result| result as u64)
}

#[inline]
fn _copy_file_range(
    fd_in: BorrowedFd<'_>,
    off_in: Option<&mut u64>,
    fd_out: BorrowedFd<'_>,
    off_out: Option<&mut u64>,
    len: usize,
    flags: c::c_uint,
) -> io::Result<usize> {
    unsafe {
        ret_usize(syscall!(
            __NR_copy_file_range,
            fd_in,
            opt_mut(off_in),
            fd_out,
            opt_mut(off_out),
            pass_usize(len),
            c_uint(flags)
        ))
    }
}

#[inline]
pub(crate) fn memfd_create(name: &CStr, flags: MemfdFlags) -> io::Result<OwnedFd> {
    unsafe { ret_owned_fd(syscall_readonly!(__NR_memfd_create, name, flags)) }
}

#[inline]
pub(crate) fn sendfile(
    out_fd: BorrowedFd<'_>,
    in_fd: BorrowedFd<'_>,
    offset: Option<&mut u64>,
    count: usize,
) -> io::Result<usize> {
    #[cfg(target_pointer_width = "32")]
    unsafe {
        ret_usize(syscall!(
            __NR_sendfile64,
            out_fd,
            in_fd,
            opt_mut(offset),
            pass_usize(count)
        ))
    }
    #[cfg(target_pointer_width = "64")]
    unsafe {
        ret_usize(syscall!(
            __NR_sendfile,
            out_fd,
            in_fd,
            opt_mut(offset),
            pass_usize(count)
        ))
    }
}
