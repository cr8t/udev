use std::os::unix::net::UnixStream;
use std::{
    mem,
    net::{SocketAddrV4, SocketAddrV6},
};

use libc::{sockaddr_ll, sockaddr_nl};

use crate::{Error, Result};

/// Represents socket variants on a Linux system.
pub enum UdevSocket {
    SocketAddrV4(SocketAddrV4),
    SocketAddrV6(SocketAddrV6),
    Unix(UnixStream),
    Netlink(sockaddr_nl),
    Physical(sockaddr_ll),
}

impl UdevSocket {
    /// Gets the kernel-provided PID for the socket.
    pub fn pid(&self) -> Result<u32> {
        match self {
            Self::Netlink(socket) => Ok(socket.nl_pid),
            _ => Err(Error::Udev("socket: PID unsupported".into())),
        }
    }

    /// Creates a new [UdevSocket] for a [`sockaddr_nl`] Netlink socket type.
    pub fn new_nl(family: i32, pid: u32, groups: u32) -> Self {
        // SAFETY: `sockaddr_nl` is a well-aligned struct, so zeroing its memory results in a valid
        // stack allocation.
        let mut nl = unsafe { mem::zeroed::<sockaddr_nl>() };

        nl.nl_family = family as u16;
        nl.nl_pid = pid;
        nl.nl_groups = groups;

        Self::Netlink(nl)
    }

    /// Gets the [UdevSocket] as a reference to a [`sockaddr_nl`](libc::sockaddr_nl).
    ///
    /// Returns `Err(Error)` if not a [UdevSocket::Netlink] variant.
    pub fn as_nl(&self) -> Result<&sockaddr_nl> {
        match self {
            Self::Netlink(nl) => Ok(nl),
            _ => Err(Error::Udev("socket: expected sockaddr_nl".into())),
        }
    }

    /// Gets the [UdevSocket] as a mutable reference to a [`sockaddr_nl`](libc::sockaddr_nl).
    ///
    /// Returns `Err(Error)` if not a [UdevSocket::Netlink] variant.
    pub fn as_nl_mut(&mut self) -> Result<&mut sockaddr_nl> {
        match self {
            Self::Netlink(nl) => Ok(nl),
            _ => Err(Error::Udev("socket: expected sockaddr_nl".into())),
        }
    }

    /// Gets the [UdevSocket] as a const pointer to a [`sockaddr_nl`](libc::sockaddr_nl).
    ///
    /// Returns `Err(Error)` if not a [UdevSocket::Netlink] variant.
    pub fn as_nl_ptr(&self) -> Result<*const sockaddr_nl> {
        match self {
            Self::Netlink(nl) => Ok(nl as *const _),
            _ => Err(Error::Udev("socket: expected sockaddr_nl".into())),
        }
    }

    /// Gets the [UdevSocket] as a const pointer to a [`sockaddr_nl`](libc::sockaddr_nl).
    ///
    /// Returns `Err(Error)` if not a [UdevSocket::Netlink] variant.
    pub fn as_nl_ptr_mut(&mut self) -> Result<*mut sockaddr_nl> {
        match self {
            Self::Netlink(nl) => Ok(nl as *mut _),
            _ => Err(Error::Udev("socket: expected sockaddr_nl".into())),
        }
    }
}
