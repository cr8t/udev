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
}
