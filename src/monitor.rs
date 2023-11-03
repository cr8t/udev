//! Connects to a device event source.

use std::{fmt, fs, io, mem, sync::Arc};

use crate::{
    util, Error, Result, Udev, UdevDevice, UdevEntry, UdevEntryList, UdevList, UdevSocket,
};

/// UDEV Monitor magic bytes
pub const UDEV_MONITOR_MAGIC: u32 = 0xfeedcafe;
// FIXME: put behind a feature flag or conditional compilation for platforms with a different run
// directory.
/// Default filesystem path for the UDEV `run` directory.
pub const UDEV_ROOT_RUN: &str = "/run";

/// Handles device event sources.
pub struct UdevMonitor {
    udev: Arc<Udev>,
    sock: i32,
    snl: UdevSocket,
    snl_group: UdevMonitorNetlinkGroup,
    snl_trusted_sender: UdevSocket,
    snl_destination: UdevSocket,
    snl_destination_group: UdevMonitorNetlinkGroup,
    addrlen: usize,
    filter_subsystem_list: UdevList,
    filter_tag_list: UdevList,
    bound: bool,
}

impl UdevMonitor {
    /// Creates a new [UdevMonitor].
    pub fn new(udev: Arc<Udev>) -> Result<Self> {
        let filter_subsystem_list = UdevList::new(Arc::clone(&udev));
        let filter_tag_list = UdevList::new(Arc::clone(&udev));

        Ok(Self {
            udev,
            sock: 0,
            snl: UdevSocket::new_nl(libc::AF_NETLINK, 0, 0),
            snl_group: UdevMonitorNetlinkGroup::None,
            snl_trusted_sender: UdevSocket::new_nl(libc::AF_NETLINK, 0, 0),
            snl_destination: UdevSocket::new_nl(libc::AF_NETLINK, 0, 0),
            snl_destination_group: UdevMonitorNetlinkGroup::None,
            addrlen: mem::size_of::<libc::sockaddr_nl>(),
            filter_subsystem_list,
            filter_tag_list,
            bound: false,
        })
    }

    /// Creates a [UdevMonitor] from group name and socket file descriptor.
    pub fn new_from_netlink_fd<N: Into<UdevMonitorNetlinkGroup> + fmt::Display + Copy>(
        udev: Arc<Udev>,
        name: N,
        fd: i32,
    ) -> Result<Self> {
        let group = match name.into() {
            UdevMonitorNetlinkGroup::Udev => {
                if fs::OpenOptions::new()
                    .read(true)
                    .open(format!("{UDEV_ROOT_RUN}/udev/control"))
                    .is_ok()
                {
                    let err_msg = "the udev service seems not to be active, disable the monitor";
                    log::debug!("{err_msg}");
                    Ok(UdevMonitorNetlinkGroup::None)
                } else {
                    Ok(UdevMonitorNetlinkGroup::Udev)
                }
            }
            UdevMonitorNetlinkGroup::Kernel => Ok(UdevMonitorNetlinkGroup::Kernel),

            UdevMonitorNetlinkGroup::None => {
                Err(Error::UdevMonitor(format!("invalid netlink group: {name}")))
            }
        }?;

        let mut udev_monitor = Self::new(udev)?;

        udev_monitor.set_snl_group(group);
        udev_monitor.set_snl_destination_group(UdevMonitorNetlinkGroup::Udev);

        if fd < 0 {
            udev_monitor.set_sock(unsafe {
                libc::socket(
                    libc::PF_NETLINK,
                    libc::SOCK_RAW | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                    libc::NETLINK_KOBJECT_UEVENT,
                )
            });

            if udev_monitor.sock() < 0 {
                let errno = io::Error::last_os_error();
                let err_msg = format!("error getting socket: {errno}");

                log::error!("{err_msg}");

                Err(Error::Io(err_msg))
            } else {
                Ok(udev_monitor)
            }
        } else {
            udev_monitor.set_sock(fd);
            udev_monitor.set_nl_address()?;

            Ok(udev_monitor.with_bound(true))
        }
    }

    /// Creates a new [UdevMonitor] from the provided parameters.
    ///
    /// Parameters:
    ///
    /// `udev`: udev library context
    /// `name`: name of event source
    ///
    /// From the `libudev` documentation:
    ///
    /// Create new udev monitor and connect to a specified event
    /// source. Valid sources identifiers are "udev" and "kernel".
    ///
    /// Applications should usually not connect directly to the
    /// "kernel" events, because the devices might not be usable
    /// at that time, before `udev` has configured them, and created
    /// device nodes. Accessing devices at the same time as `udev`,
    /// might result in unpredictable behavior. The "`udev`" events
    /// are sent out after `udev` has finished its event processing,
    /// all rules have been processed, and needed device nodes are
    /// created.
    ///
    /// Returns: a new [UdevMonitor], or [Error], in case of an error
    pub fn new_from_netlink<N: Into<UdevMonitorNetlinkGroup> + fmt::Display + Copy>(
        udev: Arc<Udev>,
        name: N,
    ) -> Result<Self> {
        Self::new_from_netlink_fd(udev, name, -1)
    }

    fn set_nl_address(&mut self) -> Result<()> {
        // SAFETY: `sockaddr_nl` is a well-aligned struct, so zeroing its memory results in a valid
        // stack allocation.
        let mut snl = unsafe { mem::zeroed::<libc::sockaddr_nl>() };
        let mut snl_len = mem::size_of::<libc::sockaddr_nl>() as u32;

        // get the address the kernel has assigned us
        // it is usually, but not necessarily the PID
        //
        // SAFETY: parameters are initialized properly, and pointers reference valid memory.
        match unsafe {
            libc::getsockname(
                self.sock,
                &mut snl as *mut libc::sockaddr_nl as *mut _,
                &mut snl_len as *mut _,
            )
        } {
            i if i >= 0 => {
                let pid = snl.nl_pid;
                log::debug!("UDEV monitor SNL PID: {pid}");
                self.set_snl(UdevSocket::Netlink(snl));
                Ok(())
            }
            _ => {
                let errno = io::Error::last_os_error();
                Err(Error::UdevMonitor(format!(
                    "unable to set SNL address: {errno}"
                )))
            }
        }
    }

    /// Gets a reference to the [Udev] context.
    pub const fn udev(&self) -> &Arc<Udev> {
        &self.udev
    }

    /// Gets the socket file descriptor.
    pub const fn sock(&self) -> i32 {
        self.sock
    }

    /// Sets the socket file descriptor.
    pub fn set_sock(&mut self, val: i32) {
        self.sock = val;
    }

    /// Builder function that sets the socket file descriptor.
    pub fn with_sock(mut self, val: i32) -> Self {
        self.set_sock(val);
        self
    }

    /// Gets a reference to the SNL [UdevSocket].
    pub const fn snl(&self) -> &UdevSocket {
        &self.snl
    }

    /// Sets the SNL [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn set_snl(&mut self, val: UdevSocket) {
        if matches!(val, UdevSocket::Netlink(_)) {
            self.snl = val;
        }
    }

    /// Builder function that sets the SNL [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn with_snl(mut self, val: UdevSocket) -> Self {
        self.set_snl(val);
        self
    }

    /// Gets the SNL [UdevMonitorNetlinkGroup].
    pub const fn snl_group(&self) -> UdevMonitorNetlinkGroup {
        self.snl_group
    }

    /// Sets the SNL [UdevMonitorNetlinkGroup].
    pub fn set_snl_group<G: Into<UdevMonitorNetlinkGroup>>(&mut self, val: G) {
        self.snl_group = val.into();
    }

    /// Builder function that sets the SNL [UdevMonitorNetlinkGroup].
    pub fn with_snl_group<G: Into<UdevMonitorNetlinkGroup>>(mut self, val: G) -> Self {
        self.set_snl_group(val);
        self
    }

    /// Gets a reference to the SNL trusted sender [UdevSocket].
    pub const fn snl_trusted_sender(&self) -> &UdevSocket {
        &self.snl_trusted_sender
    }

    /// Sets the SNL trusted sender [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn set_snl_trusted_sender(&mut self, val: UdevSocket) {
        if matches!(val, UdevSocket::Netlink(_)) {
            self.snl = val;
        }
    }

    /// Builder function that sets the SNL trusted sender [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn with_snl_trusted_sender(mut self, val: UdevSocket) -> Self {
        self.set_snl_trusted_sender(val);
        self
    }

    /// Gets a reference to the SNL destination [UdevSocket].
    pub const fn snl_destination(&self) -> &UdevSocket {
        &self.snl_destination
    }

    /// Sets the SNL destination [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn set_snl_destination(&mut self, val: UdevSocket) {
        if matches!(val, UdevSocket::Netlink(_)) {
            self.snl_destination = val;
        }
    }

    /// Builder function that sets the SNL destination [UdevSocket].
    ///
    /// **NOTE**: the SNL socket is only set for [UdevSocket::Netlink] sockets.
    pub fn with_snl_destination(mut self, val: UdevSocket) -> Self {
        self.set_snl_destination(val);
        self
    }

    /// Gets the SNL destination [UdevMonitorNetlinkGroup].
    pub const fn snl_destination_group(&self) -> UdevMonitorNetlinkGroup {
        self.snl_destination_group
    }

    /// Sets the SNL destination [UdevMonitorNetlinkGroup].
    pub fn set_snl_destination_group<G: Into<UdevMonitorNetlinkGroup>>(&mut self, val: G) {
        self.snl_destination_group = val.into();
    }

    /// Builder function that sets the SNL destination [UdevMonitorNetlinkGroup].
    pub fn with_snl_destination_group<G: Into<UdevMonitorNetlinkGroup>>(mut self, val: G) -> Self {
        self.set_snl_destination_group(val);
        self
    }

    /// Gets the socket address length.
    pub const fn addrlen(&self) -> usize {
        self.addrlen
    }

    /// Gets a reference to the filter subsystem [UdevList].
    pub const fn filter_subsystem_list(&self) -> &UdevList {
        &self.filter_subsystem_list
    }

    /// Gets a mutable reference to the filter subsystem [UdevList].
    pub fn filter_subsystem_list_mut(&mut self) -> &mut UdevList {
        &mut self.filter_subsystem_list
    }

    /// Sets the filter subsystem [UdevList].
    pub fn set_filter_subsystem_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.filter_subsystem_list.set_list(list);
    }

    /// Builder function that sets the filter subsystem [UdevList].
    pub fn with_filter_subsystem_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_filter_subsystem_list(list);
        self
    }

    /// Gets a reference to the filter tag [UdevList].
    pub const fn filter_tag_list(&self) -> &UdevList {
        &self.filter_tag_list
    }

    /// Gets a mutable reference to the filter tag [UdevList].
    pub fn filter_tag_list_mut(&mut self) -> &mut UdevList {
        &mut self.filter_tag_list
    }

    /// Sets the filter tag [UdevList].
    pub fn set_filter_tag_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.filter_tag_list.set_list(list);
    }

    /// Builder function that sets the filter tag [UdevList].
    pub fn with_filter_tag_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_filter_tag_list(list);
        self
    }

    /// Gets whether the [UdevMonitor] is bound to a socket.
    pub const fn bound(&self) -> bool {
        self.bound
    }

    fn set_bound(&mut self, val: bool) {
        self.bound = val;
    }

    fn with_bound(mut self, val: bool) -> Self {
        self.set_bound(val);
        self
    }

    /// Gets whether the [UdevDevice] passes the [UdevMonitor] filters.
    pub fn passes_filter(&self, device: &mut UdevDevice) -> bool {
        if self.filter_subsystem_list.is_empty() {
            self.filter_tag_list().has_tag(device)
        } else {
            for list_entry in self.filter_subsystem_list.iter() {
                if list_entry.name() == device.get_subsystem() {
                    let (devtype, ddevtype) = (list_entry.value(), device.devtype());

                    if !ddevtype.is_empty() && (devtype.is_empty() || devtype == ddevtype) {
                        return self.filter_tag_list().has_tag(device);
                    }
                }
            }
            false
        }
    }

    /// Receive data from the udev monitor socket, allocate a new udev
    /// device, fill in the received data, and return the device.
    ///
    /// Only socket connections with uid=0 are accepted.
    ///
    /// The monitor socket is by default set to NONBLOCK. A variant of poll() on
    /// the file descriptor returned by udev_monitor_get_fd() should to be used to
    /// wake up when new devices arrive, or alternatively the file descriptor
    /// switched into blocking mode.
    ///
    /// The initial refcount is 1, and needs to be decremented to
    /// release the resources of the udev device.
    // FIXME: break this into smaller functions
    pub fn receive_device(&mut self) -> Result<UdevDevice> {
        // avoid infinite loop, only retry up to a given limit of queued devices
        // TODO: should this limit be higher? lower?
        // the original implementation retries indefinitely, as long as there are queued devices
        for _ in 0..1024 {
            let mut buf = [0u8; 8192];
            let mut iov = libc::iovec {
                iov_base: buf.as_mut_ptr() as *mut _,
                iov_len: 8192,
            };

            let mut cred_msg = [0u8; mem::size_of::<libc::ucred>()];

            // SAFETY: `libc::sockaddr_nl` has a known-size, and is well-aligned.
            // `snl` will also be initialized below by the syscall before being used.
            let mut snl: libc::sockaddr_nl = unsafe { mem::zeroed() };
            snl.nl_family = libc::AF_NETLINK as u16;

            // SAFETY: `libc::msghdr` has a known-size, and is well-aligned.
            // `smsg` is properly initialized below before further use.
            let mut smsg: libc::msghdr = unsafe { mem::zeroed() };

            smsg.msg_iov = &mut iov as *mut libc::iovec as *mut _;
            smsg.msg_iovlen = 1;
            smsg.msg_control = cred_msg.as_mut_ptr() as *mut _;
            smsg.msg_controllen = mem::size_of::<libc::ucred>();
            smsg.msg_name = &mut snl as *mut libc::sockaddr_nl as *mut _;
            smsg.msg_namelen = mem::size_of::<libc::sockaddr_nl>() as u32;

            // SAFETY: all parameters are properly initialized, and point to valid memory.
            let buflen = unsafe { libc::recvmsg(self.sock, &mut smsg as *mut _, 0) };

            let trusted_pid = self.snl_trusted_sender.pid().unwrap_or(0);

            if buflen < 0 {
                let errno = io::Error::last_os_error();
                let err_msg = format!("unable to receive message: {errno}");

                log::error!("{err_msg}");

                Err(Error::UdevMonitor(err_msg))
            } else if buflen < 32 || smsg.msg_flags & libc::MSG_TRUNC != 0 {
                let err_msg = format!("invalid message length: {buflen}");

                log::error!("{err_msg}");

                Err(Error::UdevMonitor(err_msg))
            } else if snl.nl_groups == 0 && (trusted_pid == 0 || snl.nl_pid != trusted_pid) {
                // unicast message, check if we trust the sender
                let err_msg = "unicast netlink message ignored".to_owned();

                log::debug!("{err_msg}");

                Err(Error::UdevMonitor(err_msg))
            } else if snl.nl_groups == UdevMonitorNetlinkGroup::Kernel as u32 && snl.nl_pid > 0 {
                let pid = snl.nl_pid;
                let err_msg = format!("multicast kernel netlink message from PID {pid} ignored");

                log::debug!("{err_msg}");

                Err(Error::UdevMonitor(err_msg))
            } else {
                Ok(())
            }?;

            let libc::ucred {
                pid: _,
                uid,
                gid: _,
            } = parse_cmsg(cred_msg[..smsg.msg_controllen].as_ref())?;

            if uid != 0 {
                let err_msg = format!("sender uid={uid}, message ignored");

                log::debug!("{err_msg}");

                Err(Error::UdevMonitor(err_msg))
            } else {
                Ok(())
            }?;

            let (bufpos, is_initialized) = if let Ok(nlh) =
                UdevMonitorNetlinkHeader::try_from(buf.as_ref())
            {
                (nlh.properties_off as usize, true)
            } else {
                // kernel message header
                let bufpos = buf
                    .iter()
                    .position(|&b| b == b'\0')
                    .map(|b| b + 1)
                    .unwrap_or(0);

                if bufpos < b"a@/d".len() || bufpos >= buflen as usize {
                    let err_msg = format!("invalid message length :: buffer length: {buflen}, header length: {bufpos}, expected header: 4");

                    log::debug!("{err_msg}");

                    Err(Error::UdevMonitor(err_msg))
                } else if buf[..2].as_ref() != b"@/".as_ref() {
                    let err_msg = "unrecognized message header".to_owned();

                    log::debug!("{err_msg}");

                    Err(Error::UdevMonitor(err_msg))
                } else {
                    Ok((bufpos, false))
                }?
            };

            let mut udev_device =
                UdevDevice::new_from_nulstr(Arc::clone(&self.udev), buf[bufpos..].as_ref())
                    .map_err(|e| {
                        let err_msg = format!("could not create device: {e}");
                        log::debug!("{err_msg}");
                        Error::UdevMonitor(err_msg)
                    })?;

            if is_initialized {
                udev_device.set_is_initialized(true);
            }

            // skip device, if it does not pass the current filter
            if !self.passes_filter(&mut udev_device) {
                // if somthing is queued, get next device
                let mut pfd = [libc::pollfd {
                    fd: self.sock,
                    events: libc::POLLIN,
                    revents: 0,
                }];
                let pfd_len = pfd.len() as u64;

                // SAFETY: call to `poll` is safe because `pollfd` is properly initialized, and the
                // resulting mutable pointer references valid memory.
                if unsafe { libc::poll(pfd.as_mut_ptr(), pfd_len, 0) } > 0 {
                    // retry with the next device
                    Ok(())
                } else {
                    Err(Error::UdevMonitor(
                        "device did not pass filter, no queued devices".into(),
                    ))
                }?;
            } else {
                return Ok(udev_device);
            }
        }

        Err(Error::UdevMonitor("receive device retries exceeded".into()))
    }

    /// Sends an [UdevDevice] from one [UdevMonitor] to another.
    // FIXME: break this into smaller functions
    pub fn send_device(
        &mut self,
        mut destination: Option<&mut Self>,
        device: &mut UdevDevice,
    ) -> Result<isize> {
        let mut nlh = UdevMonitorNetlinkHeader::new();

        let mut iov = [
            libc::iovec {
                iov_base: &mut nlh as *mut UdevMonitorNetlinkHeader as *mut _,
                iov_len: mem::size_of::<UdevMonitorNetlinkHeader>(),
            },
            libc::iovec {
                iov_base: core::ptr::null_mut(),
                iov_len: 0,
            },
        ];

        let mut smsg = libc::msghdr {
            msg_iov: iov.as_mut_ptr() as *mut _,
            msg_iovlen: iov.len(),
            msg_control: core::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
            msg_name: core::ptr::null_mut(),
            msg_namelen: 0,
        };

        if device.get_properties_monitor_buf().len() < 32 {
            Err(Error::UdevMonitor(
                "device buffer is too small to contain a valid device".into(),
            ))
        } else {
            let mut buf = device.get_properties_monitor_buf().to_owned();
            let blen = buf.len();

            // fill in versioned header
            nlh.set_filter_subsystem_hash(util::string_hash32(device.get_subsystem()));

            if !device.devtype().is_empty() {
                nlh.set_filter_devtype_hash(util::string_hash32(device.devtype()));
            }

            // add tag bloom filter
            let mut tag_bloom_bits = 0u64;
            device
                .tags_list()
                .iter()
                .for_each(|list_entry| tag_bloom_bits |= util::string_bloom64(list_entry.name()));

            if tag_bloom_bits > 0 {
                nlh.set_filter_tag_bloom_hi((tag_bloom_bits >> 32) as u32);
                nlh.set_filter_tag_bloom_lo(tag_bloom_bits as u32);
            }

            // add properties list
            nlh.properties_off = iov[0].iov_len as u32;
            nlh.properties_len = blen as u32;

            iov[1].iov_base = buf.as_mut_ptr() as *mut _;
            iov[1].iov_len = blen;

            // Use custom address for target, or the default one.
            //
            // If we send to a multicast group, we will get
            // ECONNREFUSED, which is expected.
            if let Some(dest) = destination.as_mut() {
                smsg.msg_name = &mut dest.snl as *mut UdevSocket as *mut _;
            } else {
                smsg.msg_name = &mut self.snl_destination as *mut UdevSocket as *mut _;
            }

            smsg.msg_namelen = mem::size_of::<libc::sockaddr_nl>() as u32;
            // SAFETY: call to `sendmsg` is safe because the parameters are properly initialized
            // and the pointers reference valid memory.
            let count = unsafe { libc::sendmsg(self.sock, &mut smsg as *mut _, 0) };

            let mon_pid = if let Some(dest) = destination.as_ref() {
                dest.snl.pid()?
            } else {
                self.snl_destination.pid()?
            };

            if count < 0
                && destination.is_none()
                && io::Error::last_os_error().raw_os_error() == Some(libc::ECONNREFUSED)
            {
                log::debug!("passed device to netlink monitor: PID({mon_pid})");
                Ok(0)
            } else if count < 0 {
                let errno = io::Error::last_os_error();
                Err(Error::UdevMonitor(format!("sending device error: {errno}")))
            } else {
                log::debug!(
                    "monitor: passed {count} byte device to netlink monitor: PID({mon_pid})"
                );
                Ok(count)
            }
        }
    }

    /// Adds an [UdevEntry] into the filter subsystem list.
    ///
    /// From `libudev` documentation:
    ///
    /// Parameters:
    ///
    /// - `subsystem`: the subsystem value to match the incoming devices against
    ///   - must be non-empty
    /// - `devtype`: the devtype value to match the incoming devices against
    ///
    /// ```no_build,no_run
    /// This filter is efficiently executed inside the kernel, and libudev subscribers
    /// will usually not be woken up for devices which do not match.
    ///
    /// The filter must be installed before the monitor is switched to listening mode.
    /// ```
    ///
    /// Returns `Ok` on success, `Err` otherwise.
    pub fn filter_add_match_subsystem_devtype(
        &mut self,
        subsystem: &str,
        devtype: &str,
    ) -> Result<&UdevEntry> {
        if subsystem.is_empty() {
            Err(Error::UdevMonitor("empty subsystem filter".into()))
        } else {
            self.filter_subsystem_list
                .add_entry(subsystem, devtype)
                .ok_or(Error::UdevMonitor(
                    "unable to add entry to filter subsystem list".into(),
                ))
        }
    }

    /// Adds an [UdevEntry] into the filter tag list.
    ///
    /// From `libudev` documentation:
    ///
    /// - `tag`: the name of a tag
    ///   - must be non-empty
    ///
    /// ```no_build,no_run
    /// This filter is efficiently executed inside the kernel, and libudev subscribers
    /// will usually not be woken up for devices which do not match.
    ///
    /// The filter must be installed before the monitor is switched to listening mode.
    /// ```
    ///
    /// Returns `Ok` on success, `Err` otherwise.
    pub fn filter_add_match_tag(&mut self, tag: &str) -> Result<&UdevEntry> {
        if tag.is_empty() {
            Err(Error::UdevMonitor("empty tag filter".into()))
        } else {
            self.filter_tag_list
                .add_entry(tag, "")
                .ok_or(Error::UdevMonitor(
                    "unable to add entry to filter tag list".into(),
                ))
        }
    }

    /// Removes all filters from monitor.
    ///
    /// Returns `Ok` on success, `Err` otherwise.
    pub fn filter_remove(&mut self) -> Result<()> {
        let mut filter = libc::sock_fprog {
            len: 0,
            filter: std::ptr::null_mut(),
        };

        self.filter_subsystem_list.clear();

        let ret = unsafe {
            libc::setsockopt(
                self.sock,
                libc::SOL_SOCKET,
                libc::SO_ATTACH_FILTER,
                &mut filter as *mut libc::sock_fprog as *mut _,
                mem::size_of::<libc::sock_fprog>() as u32,
            )
        };

        if ret != 0 {
            let errno = io::Error::last_os_error();
            Err(Error::UdevMonitor(format!(
                "unable to remove kernel `SO_ATTACH_FILTER`: {ret}, errno: {errno}"
            )))
        } else {
            Ok(())
        }
    }
}

fn parse_cmsg(msg_control: &[u8]) -> Result<libc::ucred> {
    let controllen = msg_control.len();
    let msg_control_len = mem::size_of::<libc::cmsghdr>() + mem::size_of::<libc::ucred>();

    if controllen >= msg_control_len {
        let int_len = mem::size_of::<libc::c_int>();
        let null_int = [0u8; 4];

        // skip to the `cmsg_type` index of the `cmsghdr`
        let mut idx = int_len * 3;

        let cmsg_type = libc::c_int::from_ne_bytes(
            msg_control[idx..idx + int_len]
                .as_ref()
                .try_into()
                .unwrap_or(null_int),
        );

        idx += int_len;

        if cmsg_type != libc::SCM_CREDENTIALS {
            let err_msg = "no sender credentials received, message ignored".to_owned();

            log::debug!("{err_msg}");

            Err(Error::UdevMonitor(err_msg))
        } else {
            let pid = libc::pid_t::from_ne_bytes(
                msg_control[idx..idx + int_len]
                    .as_ref()
                    .try_into()
                    .unwrap_or(null_int),
            );
            idx += int_len;

            let uid = libc::uid_t::from_ne_bytes(
                msg_control[idx..idx + int_len]
                    .as_ref()
                    .try_into()
                    .unwrap_or(null_int),
            );
            idx += int_len;

            let gid = libc::gid_t::from_ne_bytes(
                msg_control[idx..idx + int_len]
                    .as_ref()
                    .try_into()
                    .unwrap_or(null_int),
            );

            Ok(libc::ucred { pid, uid, gid })
        }
    } else {
        Err(Error::UdevMonitor(format!(
            "msg_controllen ({controllen}) is too small for a cmsghdr"
        )))
    }
}

/// Represents the netlink group for the [UdevMonitor].
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum UdevMonitorNetlinkGroup {
    #[default]
    None,
    Kernel,
    Udev,
}

impl From<&str> for UdevMonitorNetlinkGroup {
    fn from(val: &str) -> Self {
        match val.to_lowercase().as_str() {
            "kernel" => Self::Kernel,
            "udev" => Self::Udev,
            _ => Self::None,
        }
    }
}

impl From<u32> for UdevMonitorNetlinkGroup {
    fn from(val: u32) -> Self {
        match val {
            1 => Self::Kernel,
            2 => Self::Udev,
            _ => Self::None,
        }
    }
}

impl From<&UdevMonitorNetlinkGroup> for &'static str {
    fn from(val: &UdevMonitorNetlinkGroup) -> Self {
        match val {
            UdevMonitorNetlinkGroup::None => "none",
            UdevMonitorNetlinkGroup::Kernel => "kernel",
            UdevMonitorNetlinkGroup::Udev => "udev",
        }
    }
}

/// Represents a UDEV Netlink header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UdevMonitorNetlinkHeader {
    prefix: [u8; 8],
    magic: u32,
    header_size: u32,
    properties_off: u32,
    properties_len: u32,
    filter_subsystem_hash: u32,
    filter_devtype_hash: u32,
    filter_tag_bloom_hi: u32,
    filter_tag_bloom_lo: u32,
}

impl UdevMonitorNetlinkHeader {
    /// Creates a new [UdevMonitorNetlinkHeader].
    pub const fn new() -> Self {
        Self {
            prefix: [b'l', b'i', b'b', b'u', b'd', b'e', b'v', 0],
            magic: UDEV_MONITOR_MAGIC.to_be(),
            header_size: mem::size_of::<Self>() as u32,
            properties_off: 0,
            properties_len: 0,
            filter_subsystem_hash: 0,
            filter_devtype_hash: 0,
            filter_tag_bloom_hi: 0,
            filter_tag_bloom_lo: 0,
        }
    }

    /// Gets a string representation of the [UdevMonitorNetlinkHeader] prefix.
    pub fn prefix(&self) -> &str {
        std::str::from_utf8(self.prefix.as_ref()).unwrap_or("")
    }

    /// Gets the magic bytes.
    pub const fn magic(&self) -> u32 {
        self.magic
    }

    /// Gets the total size of the [UdevMonitorNetlinkHeader].
    pub const fn header_size(&self) -> u32 {
        self.header_size
    }

    /// Gets the properties buffer offset.
    pub const fn properties_off(&self) -> u32 {
        self.properties_off
    }

    /// Sets the properties buffer offset.
    pub fn set_properties_off(&mut self, val: u32) {
        self.properties_off = val;
    }

    /// Builder function that sets the properties buffer offset.
    pub fn with_properties_off(mut self, val: u32) -> Self {
        self.set_properties_off(val);
        self
    }

    /// Gets the properties buffer length.
    pub const fn properties_len(&self) -> u32 {
        self.properties_len
    }

    /// Sets the properties buffer legnth.
    pub fn set_properties_len(&mut self, val: u32) {
        self.properties_len = val;
    }

    /// Builder function that sets the properties buffer length.
    pub fn with_properties_len(mut self, val: u32) -> Self {
        self.set_properties_len(val);
        self
    }

    /// Gets the filter subsystem hash.
    pub const fn filter_subsystem_hash(&self) -> u32 {
        self.filter_subsystem_hash
    }

    /// Sets the filter subsystem hash.
    pub fn set_filter_subsystem_hash(&mut self, val: u32) {
        self.filter_subsystem_hash = val;
    }

    /// Builder function that sets the filter subsystem hash.
    pub fn with_filter_subsystem_hash(mut self, val: u32) -> Self {
        self.set_filter_subsystem_hash(val);
        self
    }

    /// Gets the filter devtype hash.
    pub const fn filter_devtype_hash(&self) -> u32 {
        self.filter_devtype_hash
    }

    /// Sets the filter devtype hash.
    pub fn set_filter_devtype_hash(&mut self, val: u32) {
        self.filter_devtype_hash = val;
    }

    /// Builder function that sets the filter devtype hash.
    pub fn with_filter_devtype_hash(mut self, val: u32) -> Self {
        self.set_filter_devtype_hash(val);
        self
    }

    /// Gets the filter tag bloom hash high bits.
    pub const fn filter_tag_bloom_hi(&self) -> u32 {
        self.filter_tag_bloom_hi
    }

    /// Sets the filter tag bloom hash high bits.
    pub fn set_filter_tag_bloom_hi(&mut self, val: u32) {
        self.filter_tag_bloom_hi = val;
    }

    /// Builder function that sets the filter tag bloom hash high bits.
    pub fn with_filter_tag_bloom_hi(mut self, val: u32) -> Self {
        self.set_filter_tag_bloom_hi(val);
        self
    }

    /// Gets the filter tag bloom hash low bits.
    pub const fn filter_tag_bloom_lo(&self) -> u32 {
        self.filter_tag_bloom_lo
    }

    /// Sets the filter tag bloom hash low bits.
    pub fn set_filter_tag_bloom_lo(&mut self, val: u32) {
        self.filter_tag_bloom_lo = val;
    }

    /// Builder function that sets the filter tag bloom hash low bits.
    pub fn with_filter_tag_bloom_lo(mut self, val: u32) -> Self {
        self.set_filter_tag_bloom_lo(val);
        self
    }
}

impl TryFrom<&[u8]> for UdevMonitorNetlinkHeader {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        let len = val.len();
        let hdr_len = mem::size_of::<Self>();
        if len < hdr_len {
            Err(Error::UdevMonitor(format!(
                "invalid buffer length: {len}, expected at least: {hdr_len}"
            )))
        } else if &val[..8] != b"libudev\0".as_ref() {
            Err(Error::UdevMonitor(
                "invalid monitor netlink prefix, expected: 'libudev'".into(),
            ))
        } else {
            let mut idx = 0;

            let prefix: [u8; 8] = val[idx..idx + 8].try_into()?;
            idx += prefix.len();

            let magic = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let header_size = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let properties_off = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let properties_len = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let filter_subsystem_hash = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let filter_devtype_hash = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let filter_tag_bloom_hi = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);
            idx += mem::size_of::<u32>();

            let filter_tag_bloom_lo = u32::from_ne_bytes(val[idx..idx + 4].try_into()?);

            Ok(Self {
                prefix,
                magic,
                header_size,
                properties_off,
                properties_len,
                filter_subsystem_hash,
                filter_devtype_hash,
                filter_tag_bloom_hi,
                filter_tag_bloom_lo,
            })
        }
    }
}
