//! The BSD sockets API requires us to read the `ss_family` field before
//! we can interpret the rest of a `sockaddr` produced by the kernel.
#![allow(unsafe_code)]

use super::super::c;
use crate::io;
use crate::net::{Ipv4Addr, Ipv6Addr, SocketAddrAny, SocketAddrUnix, SocketAddrV4, SocketAddrV6};
use alloc::vec::Vec;
use core::mem::size_of;
use linux_raw_sys::general::{__kernel_sockaddr_storage, sockaddr};

// This must match the header of `sockaddr`.
#[repr(C)]
struct sockaddr_header {
    ss_family: u16,
}

/// Read the `ss_family` field from a socket address returned from the OS.
///
/// # Safety
///
/// `storage` must point to a valid socket address returned from the OS.
#[inline]
unsafe fn read_ss_family(storage: *const sockaddr) -> u16 {
    // Assert that we know the layout of `sockaddr`.
    let _ = sockaddr {
        __storage: __kernel_sockaddr_storage {
            __bindgen_anon_1: linux_raw_sys::general::__kernel_sockaddr_storage__bindgen_ty_1 {
                __bindgen_anon_1:
                    linux_raw_sys::general::__kernel_sockaddr_storage__bindgen_ty_1__bindgen_ty_1 {
                        ss_family: 0_u16,
                        __data: [0; 126_usize],
                    },
            },
        },
    };

    (*storage.cast::<sockaddr_header>()).ss_family
}

/// Set the `ss_family` field of a socket address to `AF_UNSPEC`, so that we
/// can test for `AF_UNSPEC` to test whether it was stored to.
#[inline]
pub(crate) unsafe fn initialize_family_to_unspec(storage: *mut sockaddr) {
    (*storage.cast::<sockaddr_header>()).ss_family = c::AF_UNSPEC as _;
}

/// Read a socket address encoded in a platform-specific format.
///
/// # Safety
///
/// `storage` must point to valid socket address storage.
pub(crate) unsafe fn read_sockaddr(
    storage: *const sockaddr,
    len: usize,
) -> io::Result<SocketAddrAny> {
    let offsetof_sun_path = super::addr::offsetof_sun_path();

    if len < size_of::<c::sa_family_t>() {
        return Err(io::Errno::INVAL);
    }
    match read_ss_family(storage).into() {
        c::AF_INET => {
            if len < size_of::<c::sockaddr_in>() {
                return Err(io::Errno::INVAL);
            }
            let decode = *storage.cast::<c::sockaddr_in>();
            Ok(SocketAddrAny::V4(SocketAddrV4::new(
                Ipv4Addr::from(u32::from_be(decode.sin_addr.s_addr)),
                u16::from_be(decode.sin_port),
            )))
        }
        c::AF_INET6 => {
            if len < size_of::<c::sockaddr_in6>() {
                return Err(io::Errno::INVAL);
            }
            let decode = *storage.cast::<c::sockaddr_in6>();
            Ok(SocketAddrAny::V6(SocketAddrV6::new(
                Ipv6Addr::from(decode.sin6_addr.in6_u.u6_addr8),
                u16::from_be(decode.sin6_port),
                u32::from_be(decode.sin6_flowinfo),
                decode.sin6_scope_id,
            )))
        }
        c::AF_UNIX => {
            if len < offsetof_sun_path {
                return Err(io::Errno::INVAL);
            }
            if len == offsetof_sun_path {
                Ok(SocketAddrAny::Unix(SocketAddrUnix::new(&[][..])?))
            } else {
                let decode = *storage.cast::<c::sockaddr_un>();
                assert_eq!(
                    decode.sun_path[len - 1 - offsetof_sun_path],
                    b'\0' as c::c_char
                );
                Ok(SocketAddrAny::Unix(SocketAddrUnix::new(
                    decode.sun_path[..len - 1 - offsetof_sun_path]
                        .iter()
                        .map(|c| *c as u8)
                        .collect::<Vec<u8>>(),
                )?))
            }
        }
        _ => Err(io::Errno::NOTSUP),
    }
}

/// Read a socket address returned from the OS.
///
/// # Safety
///
/// `storage` must point to a valid socket address returned from the OS.
pub(crate) unsafe fn maybe_read_sockaddr_os(
    storage: *const sockaddr,
    len: usize,
) -> Option<SocketAddrAny> {
    if len == 0 {
        None
    } else {
        Some(read_sockaddr_os(storage, len))
    }
}

/// Read a socket address returned from the OS.
///
/// # Safety
///
/// `storage` must point to a valid socket address returned from the OS.
pub(crate) unsafe fn read_sockaddr_os(storage: *const sockaddr, len: usize) -> SocketAddrAny {
    let offsetof_sun_path = super::addr::offsetof_sun_path();

    assert!(len >= size_of::<c::sa_family_t>());
    match read_ss_family(storage).into() {
        c::AF_INET => {
            assert!(len >= size_of::<c::sockaddr_in>());
            let decode = *storage.cast::<c::sockaddr_in>();
            SocketAddrAny::V4(SocketAddrV4::new(
                Ipv4Addr::from(u32::from_be(decode.sin_addr.s_addr)),
                u16::from_be(decode.sin_port),
            ))
        }
        c::AF_INET6 => {
            assert!(len >= size_of::<c::sockaddr_in6>());
            let decode = *storage.cast::<c::sockaddr_in6>();
            SocketAddrAny::V6(SocketAddrV6::new(
                Ipv6Addr::from(decode.sin6_addr.in6_u.u6_addr8),
                u16::from_be(decode.sin6_port),
                u32::from_be(decode.sin6_flowinfo),
                decode.sin6_scope_id,
            ))
        }
        c::AF_UNIX => {
            assert!(len >= offsetof_sun_path);
            if len == offsetof_sun_path {
                SocketAddrAny::Unix(SocketAddrUnix::new(&[][..]).unwrap())
            } else {
                let decode = *storage.cast::<c::sockaddr_un>();
                assert_eq!(
                    decode.sun_path[len - 1 - offsetof_sun_path],
                    b'\0' as c::c_char
                );
                SocketAddrAny::Unix(
                    SocketAddrUnix::new(
                        decode.sun_path[..len - 1 - offsetof_sun_path]
                            .iter()
                            .map(|c| *c as u8)
                            .collect::<Vec<u8>>(),
                    )
                    .unwrap(),
                )
            }
        }
        other => unimplemented!("{:?}", other),
    }
}
