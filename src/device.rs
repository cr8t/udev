use std::io::{self, BufRead, Read};
use std::os::linux::fs::MetadataExt;
use std::sync::Arc;
use std::{cmp, env, ffi, fmt, fs, mem, time};

use super::{Error, Mode, Result, Udev, UdevEntry, UdevEntryList, UdevList};
use crate::util;

/// Maximum number of ENVP entries
pub const ENVP_LEN: usize = 128;

/// Limits the number of characters for a UEVENT file.
///
/// **NOTE** 4 KiB limit based on default Linux filesize.
pub const UEVENT_FILE_LIMIT: usize = 0x1000;

/// Represents one kernel `sys` device.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct UdevDevice {
    udev: Arc<Udev>,
    parent_device: Option<Arc<Self>>,
    syspath: String,
    devpath: String,
    sysname: String,
    sysnum: String,
    devnode: String,
    devnode_mode: Mode,
    devnode_uid: u32,
    devnode_gid: u32,
    subsystem: String,
    devtype: String,
    driver: String,
    action: String,
    devpath_old: String,
    id_filename: String,
    envp: [String; ENVP_LEN],
    monitor_buf: String,
    devlinks_list: UdevList,
    properties_list: UdevList,
    sysattr_value_list: UdevList,
    sysattr_list: UdevList,
    tags_list: UdevList,
    seqnum: u64,
    usec_initialized: u64,
    devlink_priority: i32,
    devnum: u64,
    ifindex: i32,
    watch_handle: i32,
    maj: i32,
    min: i32,
    devlinks_uptodate: bool,
    envp_uptodate: bool,
    tags_uptodate: bool,
    info_loaded: bool,
    db_loaded: bool,
    uevent_loaded: bool,
    is_initialized: bool,
    sysattr_list_read: bool,
    db_persist: bool,
}

impl UdevDevice {
    /// Creates a new [UdevDevice].
    pub fn new(udev: Arc<Udev>) -> Self {
        let devlinks_list = UdevList::new(Arc::clone(&udev));
        let properties_list = UdevList::new(Arc::clone(&udev));
        let sysattr_value_list = UdevList::new(Arc::clone(&udev));
        let sysattr_list = UdevList::new(Arc::clone(&udev));
        let tags_list = UdevList::new(Arc::clone(&udev));

        Self {
            udev,
            parent_device: None,
            syspath: String::new(),
            devpath: String::new(),
            sysname: String::new(),
            sysnum: String::new(),
            devnode: String::new(),
            devnode_mode: Mode::new(),
            devnode_gid: 0,
            devnode_uid: 0,
            subsystem: String::new(),
            devtype: String::new(),
            driver: String::new(),
            action: String::new(),
            devpath_old: String::new(),
            id_filename: String::new(),
            envp: [""; ENVP_LEN].map(String::from),
            monitor_buf: String::new(),
            devlinks_list,
            properties_list,
            sysattr_value_list,
            sysattr_list,
            tags_list,
            seqnum: 0,
            usec_initialized: 0,
            devlink_priority: 0,
            devnum: 0,
            ifindex: 0,
            watch_handle: 0,
            maj: 0,
            min: 0,
            devlinks_uptodate: false,
            envp_uptodate: false,
            tags_uptodate: false,
            info_loaded: false,
            db_loaded: false,
            uevent_loaded: false,
            is_initialized: false,
            sysattr_list_read: false,
            db_persist: false,
        }
    }

    /// Gets a reference to the [Udev] context.
    pub fn udev(&self) -> &Udev {
        self.udev.as_ref()
    }

    /// Gets a cloned reference to the [Udev] context.
    pub fn udev_cloned(&self) -> Arc<Udev> {
        Arc::clone(&self.udev)
    }

    /// Creates a new [UdevDevice].
    pub fn new_from_nulstr(udev: Arc<Udev>, buf: &[u8]) -> Result<Self> {
        let mut device = Self::new(udev);

        device.set_info_loaded(true);

        for key in buf.split(|&b| b == 0) {
            if key.is_empty() {
                break;
            }

            device.add_property_from_string(std::str::from_utf8(key).unwrap_or(""));
        }

        device.add_property_from_string_parse_finish()?;

        Ok(device)
    }

    /// Creates new [UdevDevice], and fills in information from the sys
    /// device and the udev database entry.
    ///
    /// The `syspath` is the absolute path to the device, including the sys mount point.
    ///
    /// The initial refcount is 1, and needs to be decremented to release the resources of the udev device.
    ///
    /// Returns: a new [UdevDevice], or `Error`, if it does not exist
    pub fn new_from_syspath(udev: Arc<Udev>, syspath: &str) -> Result<Self> {
        if syspath.is_empty() {
            Err(Error::UdevDevice("empty syspath".into()))
        } else if !syspath.starts_with("/sys") {
            Err(Error::UdevDevice(format!("not in sys: {syspath}")))
        } else if let Some(path) = syspath.strip_prefix("/sys/") {
            let fullpath = if path.starts_with("/devices/") {
                let uevent_path = format!("{syspath}/uevent");
                let _ = fs::File::open(uevent_path.as_str()).map_err(|err| {
                    Error::UdevDevice(format!("unable to open syspath uevent file: {err}"))
                })?;
                uevent_path
            } else {
                syspath.into()
            };

            let dev = Self::new(udev).with_syspath(fullpath);
            log::trace!("device {dev} has devpath: {}", dev.devpath());

            Ok(dev)
        } else {
            Err(Error::UdevDevice("empty syspath subdir".into()))
        }
    }

    /// Creates new [UdevDevice].
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Fills in information from the `sys` device and the udev database entry.
    ///
    /// The device is looked-up by its major/minor number and type. Character and block device
    /// numbers are not unique across the two types.
    /// ```
    ///
    /// Parameters:
    ///
    /// - `udev`: [Udev] library context
    /// - `type`: `char` or `block` device
    /// - `devnum`: device major/minor number
    ///
    /// Returns: a new [UdevDevice], or `Err`, if it does not exist
    pub fn new_from_devnum(udev: Arc<Udev>, devtype: &str, devnum: libc::dev_t) -> Result<Self> {
        let type_str = match devtype {
            t if t.starts_with('b') => Ok("block"),
            t if t.starts_with('c') => Ok("char"),
            _ => Err(Error::UdevDevice(format!("invalid device type: {devtype}"))),
        }?;

        // use /sys/dev/{block,char}/<maj>:<min> link
        let path = format!(
            "/sys/dev/{type_str}/{}:{}",
            util::major(devnum),
            util::minor(devnum)
        );
        Self::new_from_syspath(udev, &path)
    }

    /// Creates a new [UdevDevice] from the subsystem and sysname.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Fills in information from the sys device and the udev database entry.
    ///
    /// The device is looked up by the subsystem and name string of the device, like "mem" / "zero", or "block" / "sda".
    /// ```
    ///
    /// Parameters:
    ///
    /// - `udev`: [Udev] library context
    /// - `subsystem`: the subsystem of the device
    /// - `sysname`: the name of the device
    ///
    /// Returns: a new [UdevDevice], or `Err`, if it does not exist
    pub fn new_from_subsystem_sysname(
        udev: Arc<Udev>,
        subsystem: &str,
        sysname: &str,
    ) -> Result<Self> {
        let path = if subsystem == "subsystem" {
            let sub_path = format!("/sys/subsystem/{sysname}");
            let bus_path = format!("/sys/bus/{sysname}");
            let class_path = format!("/sys/class/{sysname}");

            if fs::metadata(sub_path.as_str()).is_ok() {
                Ok(sub_path)
            } else if fs::metadata(bus_path.as_str()).is_ok() {
                Ok(bus_path)
            } else if fs::metadata(class_path.as_str()).is_ok() {
                Ok(class_path)
            } else {
                Err(Error::UdevDevice(format!(
                    "no subsystem device found for: {sysname}"
                )))
            }
        } else if subsystem == "module" {
            let path = format!("/sys/module/{sysname}");
            if fs::metadata(path.as_str()).is_ok() {
                Ok(path)
            } else {
                Err(Error::UdevDevice(format!(
                    "no module device found for: {sysname}"
                )))
            }
        } else if subsystem == "drivers" {
            if let Some(driver) = sysname.split(':').nth(2) {
                let sub_path = format!("/sys/subsystem/{sysname}/drivers/{driver}");
                let bus_path = format!("/sys/bus/{sysname}/drivers/{driver}");

                if fs::metadata(sub_path.as_str()).is_ok() {
                    Ok(sub_path)
                } else if fs::metadata(bus_path.as_str()).is_ok() {
                    Ok(bus_path)
                } else {
                    Err(Error::UdevDevice(format!(
                        "no driver device found for: {sysname}"
                    )))
                }
            } else {
                Err(Error::UdevDevice(format!(
                    "invalid driver subsystem: {sysname}"
                )))
            }
        } else {
            let sub_path = format!("/sys/subsystem/{subsystem}/devices/{sysname}");
            let bus_path = format!("/sys/bus/{subsystem}/devices/{sysname}");
            let class_path = format!("/sys/class/{subsystem}/{sysname}");

            if fs::metadata(sub_path.as_str()).is_ok() {
                Ok(sub_path)
            } else if fs::metadata(bus_path.as_str()).is_ok() {
                Ok(bus_path)
            } else if fs::metadata(class_path.as_str()).is_ok() {
                Ok(class_path)
            } else {
                Err(Error::UdevDevice(format!("no device found for: {sysname}")))
            }
        }?;

        Self::new_from_syspath(udev, &path)
    }

    /// Create new [UdevDevice] from an ID string.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    ///
    /// Fill in information from the sys device and the udev database entry.
    ///
    /// The device is looked-up by a special string:
    ///
    ///   b8:2          - block device major:minor
    ///   c128:1        - char device major:minor
    ///   n3            - network device ifindex
    ///   +sound:card29 - kernel driver core subsystem:device name
    /// ```
    ///
    /// Parameters:
    ///
    /// - `udev`: udev library context
    /// - `id`: text string identifying a kernel device
    ///
    /// Returns: a new [UdevDevice], or `Err`, if it does not exist
    pub fn new_from_device_id(udev: Arc<Udev>, id: &str) -> Result<Self> {
        match id.chars().next() {
            Some('b') | Some('c') => {
                let devtype = &id[..1];

                let mut maj_min_iter = id[1..].split(':');
                let maj = maj_min_iter
                    .next()
                    .unwrap_or("")
                    .parse::<u32>()
                    .unwrap_or(0);
                let min = maj_min_iter
                    .next()
                    .unwrap_or("")
                    .parse::<u32>()
                    .unwrap_or(0);

                Self::new_from_devnum(udev, devtype, libc::makedev(maj, min))
            }
            Some('n') => {
                let ifindex = id[1..].parse::<i32>().map_err(|err| {
                    Error::UdevDevice(format!("invalid network device ID: {id}, error: {err}"))
                })?;
                // SAFETY: all arguments are valid, and properly initialized.
                let sock = unsafe { libc::socket(libc::PF_INET, libc::SOCK_DGRAM, 0) };
                if sock < 0 {
                    let errno = io::Error::last_os_error();
                    return Err(Error::UdevDevice(format!(
                        "unable to create a socket: {errno}"
                    )));
                }

                // SAFETY: zeroed memory initializes `libc::ifreq` to a valid state.
                let mut ifr: libc::ifreq = unsafe { mem::zeroed() };
                ifr.ifr_ifru.ifru_ifindex = ifindex;
                // SAFETY: all arguments are valid, pointers reference valid memory, and `SICGIFNAME` is a valid ioctl
                if unsafe { libc::ioctl(sock, libc::SIOCGIFNAME, &mut ifr) } != 0 {
                    let errno = io::Error::last_os_error();
                    // SAFETY: `sock` is a valid socket file descriptor returned by the kernel
                    unsafe { libc::close(sock) };
                    return Err(Error::UdevDevice(format!(
                        "invalid interface name: {errno}"
                    )));
                }
                // SAFETY: `sock` is a valid socket file descriptor returned by the kernel
                unsafe { libc::close(sock) };

                // SAFETY: `ifr_name` is a C-string returned by the kernel.
                // Unless something went horribly wrong, it will be a valid, nul-terminated C-string.
                let cifr_name = unsafe { ffi::CStr::from_ptr(ifr.ifr_name.as_ptr()) };
                let ifr_name = cifr_name.to_str().map_err(|err| {
                    Error::UdevDevice(format!("unable to convert ifr_name to Rust string: {err}"))
                })?;

                let mut dev = Self::new_from_subsystem_sysname(udev, "net", ifr_name)?;
                if dev.get_ifindex() == ifindex {
                    Ok(dev)
                } else {
                    Err(Error::UdevDevice(
                        "unable to create device from ID string".into(),
                    ))
                }
            }
            Some('+') => {
                let mut subsys_iter = id[1..].split(':');
                let subsys = subsys_iter.next().ok_or(Error::UdevDevice(format!(
                    "invalid subsystem from ID: {id}"
                )))?;
                let sysname = subsys_iter.next().ok_or(Error::UdevDevice(format!(
                    "invalid system name from ID: {id}"
                )))?;

                Self::new_from_subsystem_sysname(udev, subsys, sysname)
            }
            _ => Err(Error::UdevDevice(format!("invalid device ID: {id}"))),
        }
    }

    /// Create new udev device from the environment information.
    ///
    /// From the original `libudev` documnentation:
    ///
    /// ```no_build,no_run
    /// Fills in information from the current process environment.
    /// This only works reliable if the process is called from a udev rule.
    /// It is usually used for tools executed from IMPORT= rules.
    /// ```
    ///
    /// Parameters:
    ///
    /// - `udev`: [Udev] library context
    ///
    /// Returns: a new [UdevDevice], or `Err`, if it does not exist
    pub fn new_from_environment(udev: Arc<Udev>) -> Result<Self> {
        let mut dev = Self::new(udev);
        dev.set_info_loaded(true);

        for (key, value) in env::vars() {
            dev.add_property_from_string_parse(format!("{key}={value}").as_str())?;
        }

        dev.add_property_from_string_parse_finish()?;

        Ok(dev)
    }

    /// Creates a new [UdevDevice] from the next parent directory in the syspath.
    ///
    /// Returns an `Err` if no parent is found.
    pub fn new_from_parent(&self) -> Result<Self> {
        let path = self.syspath();
        let subdir = path
            .strip_prefix("/sys/")
            .ok_or(Error::UdevDevice(format!("invalid syspath: {path}")))?;

        let mut pos = subdir.len();
        let syslen = "/sys/".len();
        loop {
            // do a reverse search for the next parent directory on the syspath
            pos = subdir[..pos]
                .rfind('/')
                .ok_or(Error::UdevDevice("no syspath subdirectory".into()))?;
            if pos < 2 {
                break;
            }
            if let Ok(dev) =
                Self::new_from_syspath(self.udev_cloned(), &path[..syslen.saturating_add(pos)])
            {
                return Ok(dev);
            }
        }

        Err(Error::UdevDevice(format!(
            "no parent device found for syspath: {path}"
        )))
    }

    /// Gets the next parent [UdevDevice].
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Find the next parent device, and fill in information from the sys
    /// device and the udev database entry.
    ///
    /// @udev_device: the device to start searching from
    ///
    /// Returned device is not referenced. It is attached to the child
    /// device, and will be cleaned up when the child device is cleaned up.
    ///
    /// It is not necessarily just the upper level directory, empty or not
    /// recognized sys directories are ignored.
    ///
    /// It can be called as many times as needed, without caring about
    /// references.
    /// ```
    ///
    /// Returns: a new [UdevDevice], or `Err`, if it no parent exists.
    pub fn get_parent(&mut self) -> Result<Arc<Self>> {
        match self.parent_device.as_ref() {
            Some(dev) => Ok(Arc::clone(dev)),
            None => {
                let dev = Arc::new(self.new_from_parent()?);
                self.parent_device = Some(Arc::clone(&dev));
                Ok(dev)
            }
        }
    }

    /// Gets the next parent [UdevDevice] based on `subsystem` and `devtype`.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Find the next parent device, with a matching subsystem and devtype
    /// value, and fill in information from the sys device and the udev
    /// database entry.
    ///
    /// If devtype is #NULL, only subsystem is checked, and any devtype will
    /// match.
    ///
    /// Returned device is not referenced. It is attached to the child
    /// device, and will be cleaned up when the child device is cleaned up.
    ///
    /// It can be called as many times as needed, without caring about
    /// references.
    /// ```
    ///
    /// Parameters:
    ///
    /// - `udev_device`: udev device to start searching from
    /// - `subsystem`: the subsystem of the device
    /// - `devtype`: the type (DEVTYPE) of the device
    ///
    /// Returns: a new [UdevDevice], or `Err` if no matching parent exists.
    pub fn get_parent_with_subsystem_devtype(
        &mut self,
        subsystem: &str,
        devtype: &str,
    ) -> Result<Arc<Self>> {
        let mut ret = Err(Error::UdevDevice("no matching parent device found".into()));
        let mut parent_res = self.new_from_parent();

        while let Ok(mut parent) = parent_res {
            let parent_subsystem = parent.get_subsystem().to_owned();
            let parent_devtype = parent.get_devtype().to_owned();

            if !parent_subsystem.is_empty()
                && parent_subsystem.as_str() == subsystem
                && !parent_devtype.is_empty()
                && parent_devtype.as_str() == devtype
            {
                let dev = Arc::new(parent);
                self.parent_device = Some(Arc::clone(&dev));

                // not a real error, but breaks the while loop next iteration
                parent_res = Err(Error::UdevDevice("set parent device".into()));
                // set a successful return value
                ret = Ok(dev);
            } else {
                // try the next parent device
                parent_res = parent.new_from_parent();
            }
        }

        ret
    }

    /// Gets whether a parent [UdevDevice] is set.
    pub const fn parent_set(&self) -> bool {
        self.parent_device.is_some()
    }

    /// Gets the [UdevDevice] syspath.
    pub fn syspath(&self) -> &str {
        self.syspath.as_str()
    }

    /// Sets the [UdevDevice] syspath.
    pub fn set_syspath<P: Into<String>>(&mut self, syspath: P) {
        self.syspath = syspath.into();
    }

    /// Builder function that sets the [UdevDevice] syspath.
    pub fn with_syspath<P: Into<String>>(mut self, syspath: P) -> Self {
        self.set_syspath(syspath);
        self
    }

    /// Gets the [UdevDevice] devpath.
    pub fn devpath(&self) -> &str {
        self.devpath.as_str()
    }

    /// Sets the [UdevDevice] devpath.
    pub fn set_devpath<P: Into<String>>(&mut self, devpath: P) {
        self.devpath = devpath.into();
    }

    /// Builder function that sets the [UdevDevice] devpath.
    pub fn with_devpath<P: Into<String>>(mut self, devpath: P) -> Self {
        self.set_devpath(devpath);
        self
    }

    /// Gets the [UdevDevice] sysname.
    pub fn sysname(&self) -> &str {
        self.sysname.as_str()
    }

    /// Sets the [UdevDevice] sysname.
    pub fn set_sysname<P: Into<String>>(&mut self, sysname: P) {
        self.sysname = sysname.into();
    }

    /// Builder function that sets the [UdevDevice] sysname.
    pub fn with_sysname<P: Into<String>>(mut self, sysname: P) -> Self {
        self.set_sysname(sysname);
        self
    }

    /// Gets the [UdevDevice] sysnum.
    pub fn sysnum(&self) -> &str {
        self.sysnum.as_str()
    }

    /// Sets the [UdevDevice] sysnum.
    pub fn set_sysnum<P: Into<String>>(&mut self, sysnum: P) {
        self.sysnum = sysnum.into();
    }

    /// Builder function that sets the [UdevDevice] sysnum.
    pub fn with_sysnum<P: Into<String>>(mut self, sysnum: P) -> Self {
        self.set_sysnum(sysnum);
        self
    }

    /// Gets the [UdevDevice] devnode.
    pub fn devnode(&self) -> &str {
        self.devnode.as_str()
    }

    /// Sets the [UdevDevice] devnode.
    pub fn set_devnode<P: Into<String>>(&mut self, devnode: P) {
        self.devnode = devnode.into();
    }

    /// Builder function that sets the [UdevDevice] devnode.
    pub fn with_devnode<P: Into<String>>(mut self, devnode: P) -> Self {
        self.set_devnode(devnode);
        self
    }

    /// Gets the [UdevDevice] `devnode`.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Retrieve the device node file name belonging to the udev device.
    /// The path is an absolute path, and starts with the device directory.
    /// ```
    ///
    /// Returns: the device node file name of the [UdevDevice], or an empty string if none exists.
    pub fn get_devnode(&mut self) -> &str {
        if self.devnode.is_empty() && !self.info_loaded {
            self.read_uevent_file().ok();
        }
        self.devnode.as_str()
    }

    /// Gets the [UdevDevice] devnode [Mode].
    pub const fn devnode_mode(&self) -> Mode {
        self.devnode_mode
    }

    /// Sets the [UdevDevice] devnode [Mode].
    pub fn set_devnode_mode(&mut self, devnode_mode: Mode) {
        self.devnode_mode = devnode_mode;
    }

    /// Builder function sets the [UdevDevice] devnode [Mode].
    pub fn with_devnode_mode(mut self, devnode_mode: Mode) -> Self {
        self.set_devnode_mode(devnode_mode);
        self
    }

    /// Gets the [UdevDevice] devnode GID.
    pub const fn devnode_gid(&self) -> u32 {
        self.devnode_gid
    }

    /// Sets the [UdevDevice] devnode GID.
    pub fn set_devnode_gid(&mut self, gid: u32) {
        self.devnode_gid = gid;
    }

    /// Builder function that sets the [UdevDevice] devnode GID.
    pub fn with_devnode_gid(mut self, gid: u32) -> Self {
        self.set_devnode_gid(gid);
        self
    }

    /// Gets the [UdevDevice] devnode UID.
    pub const fn devnode_uid(&self) -> u32 {
        self.devnode_uid
    }

    /// Sets the [UdevDevice] devnode UID.
    pub fn set_devnode_uid(&mut self, uid: u32) {
        self.devnode_uid = uid;
    }

    /// Builder function that sets the [UdevDevice] devnode UID.
    pub fn with_devnode_uid(mut self, uid: u32) -> Self {
        self.set_devnode_uid(uid);
        self
    }

    /// Gets the [UdevDevice] subsystem.
    pub fn subsystem(&self) -> &str {
        self.subsystem.as_str()
    }

    /// Sets the [UdevDevice] subsystem.
    pub fn set_subsystem<P: Into<String>>(&mut self, subsystem: P) {
        self.subsystem = subsystem.into();
    }

    /// Builder function that sets the [UdevDevice] subsystem.
    pub fn with_subsystem<P: Into<String>>(mut self, subsystem: P) -> Self {
        self.set_subsystem(subsystem);
        self
    }

    /// Retrieves the subsystem string of the `udev` device.
    ///
    /// The string does not contain any "`/`".
    ///
    /// Returns:
    ///
    /// - name of the subsystem of the `udev` device.
    /// - empty string if the subsystem can not be determined.
    pub fn get_subsystem(&mut self) -> &str {
        if self.subsystem.is_empty() {
            if let Ok(subsystem) = Udev::get_sys_core_link_value("subsystem", self.syspath()) {
                self.set_subsystem(subsystem);
            } else if self.devpath.starts_with("/module/")
                || self.devpath.rfind("/drivers/").is_some()
            {
                self.set_subsystem("module");
            } else if self.devpath.starts_with("/subsystem/")
                || self.devpath.starts_with("/class/")
                || self.devpath.starts_with("/bus/")
            {
                self.set_subsystem("subsystem");
            }
        }

        self.subsystem()
    }

    /// Gets the [UdevDevice] devtype.
    pub fn devtype(&self) -> &str {
        self.devtype.as_str()
    }

    /// Sets the [UdevDevice] devtype.
    pub fn set_devtype<P: Into<String>>(&mut self, devtype: P) {
        self.devtype = devtype.into();
    }

    /// Builder function that sets the [UdevDevice] devtype.
    pub fn with_devtype<P: Into<String>>(mut self, devtype: P) -> Self {
        self.set_devtype(devtype);
        self
    }

    /// Gets the devtypes string of the [UdevDevice].
    ///
    /// If the devtype is unset, attempts to read properties from the syspath file.
    pub fn get_devtype(&mut self) -> &str {
        if self.devtype.is_empty() {
            self.read_uevent_file().ok();
        }
        self.devtype.as_str()
    }

    /// Gets the [UdevDevice] driver.
    pub fn driver(&self) -> &str {
        self.driver.as_str()
    }

    /// Sets the [UdevDevice] driver.
    pub fn set_driver<P: Into<String>>(&mut self, driver: P) {
        self.driver = driver.into();
    }

    /// Builder function that sets the [UdevDevice] driver.
    pub fn with_driver<P: Into<String>>(mut self, driver: P) -> Self {
        self.set_driver(driver);
        self
    }

    /// Gets the kernel driver name.
    ///
    /// Returns: the kernel driver name, or `None`  if none is attached.
    pub fn get_driver(&mut self) -> Option<&str> {
        if self.driver.is_empty() {
            self.driver = Udev::get_sys_core_link_value("driver", self.syspath.as_str()).ok()?;
        }
        Some(self.driver.as_str())
    }

    /// Gets the [UdevDevice] action.
    pub fn action(&self) -> &str {
        self.action.as_str()
    }

    /// Sets the [UdevDevice] action.
    pub fn set_action<P: Into<String>>(&mut self, action: P) {
        self.action = action.into();
    }

    /// Builder function that sets the [UdevDevice] action.
    pub fn with_action<P: Into<String>>(mut self, action: P) -> Self {
        self.set_action(action);
        self
    }

    /// Gets the [UdevDevice] devpath_old.
    pub fn devpath_old(&self) -> &str {
        self.devpath_old.as_str()
    }

    /// Sets the [UdevDevice] devpath_old.
    pub fn set_devpath_old<P: Into<String>>(&mut self, devpath_old: P) {
        self.devpath_old = devpath_old.into();
    }

    /// Builder function that sets the [UdevDevice] devpath_old.
    pub fn with_devpath_old<P: Into<String>>(mut self, devpath_old: P) -> Self {
        self.set_devpath_old(devpath_old);
        self
    }

    /// Gets the [UdevDevice] id_filename.
    pub fn id_filename(&self) -> &str {
        self.id_filename.as_str()
    }

    /// Sets the [UdevDevice] id_filename.
    pub fn set_id_filename<P: Into<String>>(&mut self, id_filename: P) {
        self.id_filename = id_filename.into();
    }

    /// Builder function that sets the [UdevDevice] id_filename.
    pub fn with_id_filename<P: Into<String>>(mut self, id_filename: P) -> Self {
        self.set_id_filename(id_filename);
        self
    }

    /// Gets the ID filename.
    ///
    /// If the filename is empty, attempts to set it based on subsystem information.
    ///
    /// Returns: the ID filename, or an empty string if none can be constructed.
    pub fn get_id_filename(&mut self) -> &str {
        let id_filename = if self.id_filename.is_empty() {
            if self.get_subsystem().is_empty() {
                String::new()
            } else if util::major(self.get_devnum()) > 0 {
                // From `libudev` documentation:
                //
                // use devtype: <type><major>:<minor>, e.g. b259:131072, c254:0
                let devtype = if self.get_subsystem() == "block" {
                    'b'
                } else {
                    'c'
                };
                let devnum = self.devnum();
                let major = util::major(devnum);
                let minor = util::minor(devnum);
                format!("{devtype}{major}:{minor}")
            } else if self.get_ifindex() > 0 {
                // From `libudev` documentation:
                //
                // use netdev ifindex: <type><ifindex>, e.g. n3
                format!("n{}", self.ifindex)
            } else if let Some(sysname) = self.devpath.clone().rsplit('/').next() {
                // From `libudev` documentation:
                //
                // use $subsys:$sysname, e.g. pci:0000:00:1f.2
                // sysname() has '!' translated, get it from devpath
                format!("+{}:{sysname}", self.get_subsystem())
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if id_filename.is_empty() {
            ""
        } else {
            self.id_filename = id_filename;
            self.id_filename()
        }
    }

    /// Gets a reference to the list of `envp` arguments.
    pub fn envp(&self) -> &[String] {
        let len = self.envp_len();
        self.envp[..len].as_ref()
    }

    /// Gets the length of non-empty `envp` arguments.
    pub fn envp_len(&self) -> usize {
        self.envp.iter().take_while(|e| !e.is_empty()).count()
    }

    /// Sets the list of `envp` arguments.
    pub fn set_envp<P: Into<String> + Clone>(&mut self, envp: &[P]) {
        let len = cmp::min(envp.len(), ENVP_LEN);
        self.envp[..len]
            .iter_mut()
            .zip(envp.iter().cloned())
            .for_each(|(dst, src)| *dst = src.into());
        self.envp[len..].iter_mut().for_each(|s| s.clear());
    }

    /// Builder function that sets the list of `envp` arguments.
    pub fn with_envp<P: Into<String> + Clone>(mut self, envp: &[P]) -> Self {
        self.set_envp(envp);
        self
    }

    /// Gets whether the `envp` list is empty.
    pub fn envp_is_empty(&self) -> bool {
        self.envp.iter().filter(|e| !e.is_empty()).count() == 0
    }

    /// Clears the `envp` arguments.
    pub fn clear_envp(&mut self) {
        self.envp.iter_mut().for_each(|s| s.clear());
    }

    /// Gets the [UdevDevice] monitor_buf.
    pub fn monitor_buf(&self) -> &str {
        self.monitor_buf.as_str()
    }

    /// Sets the [UdevDevice] monitor_buf.
    pub fn set_monitor_buf<P: Into<String>>(&mut self, monitor_buf: P) {
        self.monitor_buf = monitor_buf.into();
    }

    /// Builder function that sets the [UdevDevice] monitor_buf.
    pub fn with_monitor_buf<P: Into<String>>(mut self, monitor_buf: P) -> Self {
        self.set_monitor_buf(monitor_buf);
        self
    }

    /// Gets the environment properties monitor buffer.
    ///
    /// If the environment is not up to date, updates the monitor buffer.
    pub fn get_properties_monitor_buf(&mut self) -> &str {
        if !self.envp_uptodate {
            self.update_envp_monitor_buf();
        }
        self.monitor_buf()
    }

    /// Updates the `envp` and monitor buffer from the properties list.
    pub fn update_envp_monitor_buf(&mut self) {
        self.monitor_buf.clear();
        self.clear_envp();

        // add at most `ENVP_LEN` properties, skipping private entries
        for (i, list_entry) in self
            .properties_list
            .iter()
            .filter(|e| !e.name().starts_with('.'))
            .enumerate()
            .take(ENVP_LEN)
        {
            let key = list_entry.name();
            let value = list_entry.value();

            // add string to envp
            let envp_str = format!("{key}={value}");

            self.monitor_buf += envp_str.as_str();
            self.monitor_buf += "\0";

            self.envp[i] = envp_str;
        }

        self.set_envp_uptodate(true);
    }

    /// Gets a reference to the [UdevDevice] `devlinks_list` [UdevList].
    pub const fn devlinks_list(&self) -> &UdevList {
        &self.devlinks_list
    }

    /// Gets a mutable reference to the [UdevDevice] `devlinks_list` [UdevList].
    pub fn devlinks_list_mut(&mut self) -> &mut UdevList {
        &mut self.devlinks_list
    }

    /// Gets the next entry in the `devlinks` list.
    pub fn devlinks_list_entry(&self) -> Option<&UdevEntry> {
        self.devlinks_list.next_entry()
    }

    /// Gets the list of device links for the [UdevDevice].
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Retrieve the list of device links pointing to the device file of
    /// the udev device. The next list entry can be retrieved with
    /// udev_list_entry_get_next(), which returns #NULL if no more entries exist.
    ///
    /// The devlink path can be retrieved from the list entry by
    /// udev_list_entry_get_name(). The path is an absolute path, and starts with
    /// the device directory.
    /// ```
    ///
    /// Returns: the first entry of the device node link list
    pub fn get_devlinks_list_entry(&mut self) -> Option<&UdevEntry> {
        if !self.info_loaded {
            self.read_db().ok()?;
        }
        self.devlinks_list_entry()
    }

    /// Sets the [UdevDevice] `devlinks_list` [UdevList].
    pub fn set_devlinks_list<U: Into<UdevEntryList>>(&mut self, devlinks_list: U) {
        self.devlinks_list.set_list(devlinks_list);
    }

    /// Builder function sets the [UdevDevice] [UdevList].
    pub fn with_devlinks_list<U: Into<UdevEntryList>>(mut self, devlinks_list: U) -> Self {
        self.set_devlinks_list(devlinks_list);
        self
    }

    /// Adds an [UdevEntry] to the devlinks list.
    pub fn add_devlink(&mut self, devlink: &str) {
        self.set_devlinks_uptodate(false);
        self.devlinks_list.add_entry(devlink, "");
    }

    /// Gets a reference to the [UdevDevice] `properties_list` [UdevList].
    pub const fn properties_list(&self) -> &UdevList {
        &self.properties_list
    }

    /// Gets a mutable reference to the [UdevDevice] `properties_list` [UdevList].
    pub fn properties_list_mut(&mut self) -> &mut UdevList {
        &mut self.properties_list
    }

    /// Sets the [UdevDevice] `properties_list` [UdevList].
    pub fn set_properties_list<U: Into<UdevEntryList>>(&mut self, properties_list: U) {
        self.properties_list.set_list(properties_list);
    }

    /// Builder function sets the [UdevDevice] [UdevList].
    pub fn with_properties_list<U: Into<UdevEntryList>>(mut self, properties_list: U) -> Self {
        self.set_properties_list(properties_list);
        self
    }

    /// Gets the value of a given property.
    pub fn get_property_value(&self, key: &str) -> Option<&str> {
        self.properties_list.entry_by_name(key).map(|e| e.value())
    }

    /// Gets a reference to the [UdevDevice] `sysattr_value_list` [UdevList].
    pub const fn sysattr_value_list(&self) -> &UdevList {
        &self.sysattr_value_list
    }

    /// Gets a mutable reference to the [UdevDevice] `sysattr_value_list` [UdevList].
    pub fn sysattr_value_list_mut(&mut self) -> &mut UdevList {
        &mut self.sysattr_value_list
    }

    /// Sets the [UdevDevice] `sysattr_value_list` [UdevList].
    pub fn set_sysattr_value_list<U: Into<UdevEntryList>>(&mut self, sysattr_value_list: U) {
        self.sysattr_value_list.set_list(sysattr_value_list);
    }

    /// Builder function sets the [UdevDevice] [UdevList].
    pub fn with_sysattr_value_list<U: Into<UdevEntryList>>(
        mut self,
        sysattr_value_list: U,
    ) -> Self {
        self.set_sysattr_value_list(sysattr_value_list);
        self
    }

    /// Gets a reference to the [UdevDevice] `sysattr_list` [UdevList].
    pub const fn sysattr_list(&self) -> &UdevList {
        &self.sysattr_list
    }

    /// Gets a mutable reference to the [UdevDevice] `sysattr_list` [UdevList].
    pub fn sysattr_list_mut(&mut self) -> &mut UdevList {
        &mut self.sysattr_list
    }

    /// Sets the [UdevDevice] `sysattr_list` [UdevList].
    pub fn set_sysattr_list<U: Into<UdevEntryList>>(&mut self, sysattr_list: U) {
        self.sysattr_list.set_list(sysattr_list);
    }

    /// Builder function sets the [UdevDevice] [UdevList].
    pub fn with_sysattr_list<U: Into<UdevEntryList>>(mut self, sysattr_list: U) -> Self {
        self.set_sysattr_list(sysattr_list);
        self
    }

    /// Gets the first entry in the `sysattr` properties list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Retrieve the list of available sysattrs, with value being empty;
    /// This just return all available sysfs attributes for a particular
    /// device without reading their values.
    /// ```
    ///
    /// Returns: the first entry of the property list
    pub fn get_sysattr_list_entry(&mut self) -> Option<&UdevEntry> {
        let num = if !self.sysattr_list_read {
            self.get_sysattr_list_read().ok()?
        } else {
            0
        };
        if num > 0 {
            None
        } else {
            self.sysattr_list.entry()
        }
    }

    /// Gets the sys attribute file value.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// The retrieved value is cached in the device. Repeated calls will return the same
    /// value and not open the attribute again.
    /// ```
    ///
    /// Returns: the content of a sys attribute file, or `None` if there is no sys attribute value.
    pub fn get_sysattr_value(&mut self, sysattr: &str) -> Option<String> {
        match self.sysattr_value_list.entry_by_name(sysattr).cloned() {
            Some(entry) => Some(entry.value().to_owned()),
            None => {
                let path = format!("{}/{sysattr}", self.syspath);
                match fs::metadata(path.as_str()) {
                    Ok(metadata) => {
                        if metadata.is_symlink() {
                            if sysattr == "driver" || sysattr == "subsystem" || sysattr == "module"
                            {
                                let value =
                                    Udev::get_sys_core_link_value(sysattr, self.syspath.as_str())
                                        .ok()?;
                                Some(
                                    self.sysattr_value_list
                                        .add_entry(sysattr, value.as_str())?
                                        .value()
                                        .to_owned(),
                                )
                            } else {
                                None
                            }
                        } else if metadata.is_dir()
                            || metadata.len() == 0
                            || metadata.st_mode() & libc::S_IRUSR == 0
                        {
                            None
                        } else {
                            let mut file = fs::File::open(path.as_str()).ok()?;
                            let mut value = [0u8; 4096];
                            let read = file.read(&mut value).ok()?;
                            let value_str =
                                std::str::from_utf8(value[..read].as_ref()).unwrap_or("");
                            let entry = self.sysattr_value_list.add_entry(sysattr, value_str)?;

                            Some(entry.value().to_owned())
                        }
                    }
                    Err(_) => {
                        self.sysattr_value_list.add_entry(sysattr, "");
                        None
                    }
                }
            }
        }
    }

    /// Gets a reference to the [UdevDevice] `tags_list` [UdevList].
    pub const fn tags_list(&self) -> &UdevList {
        &self.tags_list
    }

    /// Gets a mutable reference to the [UdevDevice] `tags_list` [UdevList].
    pub fn tags_list_mut(&mut self) -> &mut UdevList {
        &mut self.tags_list
    }

    /// Sets the [UdevDevice] `tags_list` [UdevList].
    pub fn set_tags_list<U: Into<UdevEntryList>>(&mut self, tags_list: U) {
        self.tags_list.set_list(tags_list);
    }

    /// Builder function sets the [UdevDevice] [UdevList].
    pub fn with_tags_list<U: Into<UdevEntryList>>(mut self, tags_list: U) -> Self {
        self.set_tags_list(tags_list);
        self
    }

    /// Gets the first tags list entry in the [UdevDevice].
    pub fn tags_list_entry(&self) -> Option<&UdevEntry> {
        self.tags_list.entry()
    }

    /// Gets the first tags list entry in the [UdevDevice].
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Retrieve the list of tags attached to the udev device. The next
    /// list entry can be retrieved with udev_list_entry_get_next(),
    /// which returns `None` if no more entries exist. The tag string
    /// can be retrieved from the list entry by udev_list_entry_get_name().
    /// ```
    ///
    /// Returns: the first entry of the tag list
    pub fn get_tags_list_entry(&mut self) -> Option<&UdevEntry> {
        if !self.info_loaded {
            self.read_db().ok();
        }
        self.tags_list_entry()
    }

    /// Gets the current tags list entry in the [UdevDevice].
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Retrieve the list of tags attached to the udev device. The next
    /// list entry can be retrieved with udev_list_entry_get_next(),
    /// which returns `None` if no more entries exist. The tag string
    /// can be retrieved from the list entry by udev_list_entry_get_name().
    /// ```
    ///
    /// Returns: the current entry of the tag list
    pub fn get_current_tags_list_entry(&mut self) -> Option<&UdevEntry> {
        if !self.info_loaded {
            self.read_db().ok();
        }
        self.tags_list.next_entry()
    }

    /// Adds an [UdevEntry] to the tags list.
    pub fn add_tag(&mut self, tag: &str) -> Result<()> {
        Self::is_valid_tag(tag)?;

        self.set_tags_uptodate(false);
        self.tags_list.add_entry(tag, "");

        Ok(())
    }

    fn is_valid_tag(tag: &str) -> Result<()> {
        if tag.contains(':') || tag.contains(' ') {
            Err(Error::Udev("device: invalid tag".into()))
        } else {
            Ok(())
        }
    }

    /// Gets whether the [UdevDevice] has the provided `tag` associated.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Check if a given device has a certain tag associated.
    /// ```
    pub fn has_tag(&mut self, tag: &str) -> bool {
        if !self.info_loaded {
            self.read_db().ok();
        }
        self.tags_list.entry_by_name(tag).is_some()
    }

    /// Gets the [UdevDevice] seqnum.
    pub const fn seqnum(&self) -> u64 {
        self.seqnum
    }

    /// Sets the [UdevDevice] seqnum.
    pub fn set_seqnum(&mut self, seqnum: u64) {
        self.seqnum = seqnum;
    }

    /// Builder function sets the [UdevDevice] seqnum.
    pub fn with_seqnum(mut self, seqnum: u64) -> Self {
        self.set_seqnum(seqnum);
        self
    }

    /// Gets the [UdevDevice] usec_initialized.
    pub const fn usec_initialized(&self) -> u64 {
        self.usec_initialized
    }

    /// Sets the [UdevDevice] usec_initialized.
    pub fn set_usec_initialized(&mut self, usec_initialized: u64) {
        self.usec_initialized = usec_initialized;
    }

    /// Builder function sets the [UdevDevice] usec_initialized.
    pub fn with_usec_initialized(mut self, usec_initialized: u64) -> Self {
        self.set_usec_initialized(usec_initialized);
        self
    }

    /// Gets the number of microseconds since the [UdevDevice] was initialized.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Return the number of microseconds passed since udev set up the
    /// device for the first time.
    ///
    /// This is only implemented for devices with need to store properties
    /// in the udev database. All other devices return 0 here.
    /// ```
    ///
    /// Returns: the number of microseconds since the device was first seen.
    pub fn get_usec_since_initialized(&mut self) -> u64 {
        if !self.info_loaded {
            self.read_db().ok();
        }
        if self.usec_initialized == 0 {
            0
        } else {
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap_or(time::Duration::from_micros(self.usec_initialized))
                .as_micros()
                .saturating_sub(self.usec_initialized as u128) as u64
        }
    }

    /// Gets the [UdevDevice] devlink priority.
    pub const fn devlink_priority(&self) -> i32 {
        self.devlink_priority
    }

    /// Sets the [UdevDevice] devlink priority.
    pub fn set_devlink_priority(&mut self, devlink_priority: i32) {
        self.devlink_priority = devlink_priority;
    }

    /// Builder function sets the [UdevDevice] devlink priority.
    pub fn with_devlink_priority(mut self, devlink_priority: i32) -> Self {
        self.set_devlink_priority(devlink_priority);
        self
    }

    /// Gets the [UdevDevice] devnum.
    pub const fn devnum(&self) -> u64 {
        self.devnum
    }

    /// Sets the [UdevDevice] devnum.
    pub fn set_devnum(&mut self, devnum: u64) {
        self.devnum = devnum;
    }

    /// Builder function sets the [UdevDevice] devnum.
    pub fn with_devnum(mut self, devnum: u64) -> Self {
        self.set_devnum(devnum);
        self
    }

    /// Gets the device major/minor number.
    pub fn get_devnum(&mut self) -> u64 {
        if !self.info_loaded {
            self.read_uevent_file().ok();
        }
        self.devnum
    }

    /// Gets the [UdevDevice] ifindex.
    pub const fn ifindex(&self) -> i32 {
        self.ifindex
    }

    /// Sets the [UdevDevice] ifindex.
    pub fn set_ifindex(&mut self, ifindex: i32) {
        self.ifindex = ifindex;
    }

    /// Builder function sets the [UdevDevice] ifindex.
    pub fn with_ifindex(mut self, ifindex: i32) -> Self {
        self.set_ifindex(ifindex);
        self
    }

    /// Gets the [UdevDevice] ifindex.
    ///
    /// If the information file is not loaded, reads and parses the file from disk.
    pub fn get_ifindex(&mut self) -> i32 {
        if !self.info_loaded {
            self.read_uevent_file().ok();
        }
        self.ifindex
    }

    /// Gets the [UdevDevice] watch handle.
    pub const fn watch_handle(&self) -> i32 {
        self.watch_handle
    }

    /// Sets the [UdevDevice] watch handle.
    pub fn set_watch_handle(&mut self, watch_handle: i32) {
        self.watch_handle = watch_handle;
    }

    /// Builder function sets the [UdevDevice] watch handle.
    pub fn with_watch_handle(mut self, watch_handle: i32) -> Self {
        self.set_watch_handle(watch_handle);
        self
    }

    /// Gets the [UdevDevice] major number.
    pub const fn maj(&self) -> i32 {
        self.maj
    }

    /// Sets the [UdevDevice] major number.
    pub fn set_maj(&mut self, maj: i32) {
        self.maj = maj;
    }

    /// Builder function sets the [UdevDevice] major number.
    pub fn with_maj(mut self, maj: i32) -> Self {
        self.set_maj(maj);
        self
    }

    /// Gets the [UdevDevice] minor number.
    pub const fn min(&self) -> i32 {
        self.min
    }

    /// Sets the [UdevDevice] minor number.
    pub fn set_min(&mut self, min: i32) {
        self.min = min;
    }

    /// Builder function sets the [UdevDevice] minor number.
    pub fn with_min(mut self, min: i32) -> Self {
        self.set_min(min);
        self
    }

    /// Gets the [UdevDevice] devlinks up-to-date.
    pub const fn devlinks_uptodate(&self) -> bool {
        self.devlinks_uptodate
    }

    /// Sets the [UdevDevice] devlinks up-to-date.
    pub fn set_devlinks_uptodate(&mut self, devlinks_uptodate: bool) {
        self.devlinks_uptodate = devlinks_uptodate;
    }

    /// Builder function sets the [UdevDevice] devlinks up-to-date.
    pub fn with_devlinks_uptodate(mut self, devlinks_uptodate: bool) -> Self {
        self.set_devlinks_uptodate(devlinks_uptodate);
        self
    }

    /// Gets the [UdevDevice] envp up-to-date.
    pub const fn envp_uptodate(&self) -> bool {
        self.envp_uptodate
    }

    /// Sets the [UdevDevice] envp up-to-date.
    pub fn set_envp_uptodate(&mut self, envp_uptodate: bool) {
        self.envp_uptodate = envp_uptodate;
    }

    /// Builder function sets the [UdevDevice] envp up-to-date.
    pub fn with_envp_uptodate(mut self, envp_uptodate: bool) -> Self {
        self.set_envp_uptodate(envp_uptodate);
        self
    }

    /// Gets the [UdevDevice] tags up-to-date.
    pub const fn tags_uptodate(&self) -> bool {
        self.tags_uptodate
    }

    /// Sets the [UdevDevice] tags up-to-date.
    pub fn set_tags_uptodate(&mut self, tags_uptodate: bool) {
        self.tags_uptodate = tags_uptodate;
    }

    /// Builder function sets the [UdevDevice] tags up-to-date.
    pub fn with_tags_uptodate(mut self, tags_uptodate: bool) -> Self {
        self.set_tags_uptodate(tags_uptodate);
        self
    }

    /// Gets the [UdevDevice] info loaded.
    pub const fn info_loaded(&self) -> bool {
        self.info_loaded
    }

    /// Sets the [UdevDevice] info loaded.
    pub fn set_info_loaded(&mut self, info_loaded: bool) {
        self.info_loaded = info_loaded;
    }

    /// Builder function sets the [UdevDevice] info loaded.
    pub fn with_info_loaded(mut self, info_loaded: bool) -> Self {
        self.set_info_loaded(info_loaded);
        self
    }

    /// Gets the [UdevDevice] db loaded.
    pub const fn db_loaded(&self) -> bool {
        self.db_loaded
    }

    /// Sets the [UdevDevice] db loaded.
    pub fn set_db_loaded(&mut self, db_loaded: bool) {
        self.db_loaded = db_loaded;
    }

    /// Builder function sets the [UdevDevice] db loaded.
    pub fn with_db_loaded(mut self, db_loaded: bool) -> Self {
        self.set_db_loaded(db_loaded);
        self
    }

    /// Gets the [UdevDevice] uevent loaded.
    pub const fn uevent_loaded(&self) -> bool {
        self.uevent_loaded
    }

    /// Sets the [UdevDevice] uevent loaded.
    pub fn set_uevent_loaded(&mut self, uevent_loaded: bool) {
        self.uevent_loaded = uevent_loaded;
    }

    /// Builder function sets the [UdevDevice] uevent loaded.
    pub fn with_uevent_loaded(mut self, uevent_loaded: bool) -> Self {
        self.set_uevent_loaded(uevent_loaded);
        self
    }

    /// Gets the [UdevDevice] is initialized.
    pub const fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Sets the [UdevDevice] is initialized.
    pub fn set_is_initialized(&mut self, is_initialized: bool) {
        self.is_initialized = is_initialized;
    }

    /// Builder function sets the [UdevDevice] is initialized.
    pub fn with_is_initialized(mut self, is_initialized: bool) -> Self {
        self.set_is_initialized(is_initialized);
        self
    }

    /// Gets whether the [UdevDevice] is initialized.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Check if udev has already handled the device and has set up
    /// device node permissions and context, or has renamed a network
    /// device.
    ///
    /// This is only implemented for devices with a device node
    /// or network interfaces. All other devices return 1 here.
    /// ```
    pub fn get_is_initialized(&mut self) -> bool {
        if !self.info_loaded {
            self.read_db().ok();
        }
        self.is_initialized
    }

    /// Gets the [UdevDevice] sysattr list read.
    pub const fn sysattr_list_read(&self) -> bool {
        self.sysattr_list_read
    }

    /// Sets the [UdevDevice] sysattr list read.
    pub fn set_sysattr_list_read(&mut self, sysattr_list_read: bool) {
        self.sysattr_list_read = sysattr_list_read;
    }

    /// Builder function sets the [UdevDevice] sysattr list read.
    pub fn with_sysattr_list_read(mut self, sysattr_list_read: bool) -> Self {
        self.set_sysattr_list_read(sysattr_list_read);
        self
    }

    pub fn get_sysattr_list_read(&mut self) -> Result<usize> {
        let mut num = 0usize;
        if self.sysattr_list_read {
            Ok(num)
        } else {
            let syspath = self.syspath().to_owned();

            for dir_entry in fs::read_dir(syspath.as_str())
                .map_err(|err| Error::UdevDevice(format!("unable to open device syspath: {err}")))?
            {
                let entry = match dir_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let file_type = entry.file_type().map_err(|err| {
                    Error::UdevDevice(format!("unable to get syspath entry file type: {err}"))
                })?;

                if !file_type.is_symlink() && !file_type.is_file() {
                    continue;
                }

                let name = match entry.file_name().into_string() {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                let path = format!("{syspath}/{name}");
                let metadata = match fs::metadata(path.as_str()) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if metadata.st_mode() & libc::S_IRUSR != 0 {
                    continue;
                }

                self.sysattr_list.add_entry(name.as_str(), "");
                num = num.saturating_add(1);
            }

            self.sysattr_list_read = true;

            Ok(num)
        }
    }

    /// Gets the [UdevDevice] database persist.
    pub const fn db_persist(&self) -> bool {
        self.db_persist
    }

    /// Sets the [UdevDevice] database persist.
    pub fn set_db_persist(&mut self, db_persist: bool) {
        self.db_persist = db_persist;
    }

    /// Builder function sets the [UdevDevice] database persist.
    pub fn with_db_persist(mut self, db_persist: bool) -> Self {
        self.set_db_persist(db_persist);
        self
    }

    /// Reads [UdevDevice] information from the persistent database file.
    ///
    /// Returns: `Ok(())` on success, `Err(Error)` otherwise
    pub fn read_db(&mut self) -> Result<()> {
        let id = self.get_id_filename().to_owned();
        if self.db_loaded() {
            Ok(())
        } else if id.is_empty() {
            Err(Error::UdevDevice("unable to retrieve ID filename".into()))
        } else {
            let filename = format!("/udev/data/{id}");
            let file = fs::File::open(filename.as_str()).map_err(|err| {
                Error::UdevDevice(format!("unable to open DB file: {filename}, error: {err}"))
            })?;

            // devices with a database entry are initialized
            self.is_initialized = true;

            let mut line = String::with_capacity(util::LINE_SIZE);
            let mut reader = io::BufReader::new(file);

            while let Ok(len) = reader.read_line(&mut line) {
                if len < 4 {
                    break;
                }
                let val = &line[2..];
                match val.chars().next() {
                    Some('S') => {
                        self.add_devlink(format!("/dev/{val}").as_str());
                        break;
                    }
                    Some('L') => {
                        self.set_devlink_priority(val.parse::<i32>().unwrap_or(0));
                        break;
                    }
                    Some('E') => {
                        if self.add_property_from_string(val).is_some() {
                            if let Some(entry) = self.properties_list.list_mut().back_mut() {
                                entry.set_num(1);
                            }
                        }
                        break;
                    }
                    Some('G') => {
                        self.add_tag(val)?;
                        break;
                    }
                    Some('W') => {
                        self.set_watch_handle(val.parse::<i32>().unwrap_or(0));
                        break;
                    }
                    Some('I') => {
                        self.set_usec_initialized(val.parse::<u64>().unwrap_or(0));
                        break;
                    }
                    _ => (),
                }
            }

            log::trace!("device {self} filled with DB file data");

            Ok(())
        }
    }

    /// Reads properties from the `uevent` file.
    pub fn read_uevent_file(&mut self) -> Result<()> {
        if !self.uevent_loaded {
            Ok(())
        } else {
            let filename = format!("{}/uevent", self.syspath());
            let f = fs::OpenOptions::new().read(true).open(filename)?;

            self.uevent_loaded = true;

            let mut reader = io::BufReader::new(f.take(UEVENT_FILE_LIMIT as u64));
            let mut line = String::new();

            let (mut maj, mut min) = (0u32, 0u32);

            while let Ok(read) = reader.read_line(&mut line) {
                let tline = line.trim_end_matches('\n');

                if let Some(devtype) = tline.strip_prefix("DEVTYPE=") {
                    self.set_devtype(devtype);
                    continue;
                }
                if let Some(ifindex) = tline.strip_prefix("IFINDEX=") {
                    self.set_ifindex(ifindex.parse::<i32>().unwrap_or(0));
                    continue;
                }
                if let Some(devname) = tline.strip_prefix("DEVNAME=") {
                    self.set_devnode(devname);
                    continue;
                }

                if let Some(major) = tline.strip_prefix("MAJOR=") {
                    maj = major.parse::<u32>().unwrap_or(0);
                } else if let Some(minor) = tline.strip_prefix("MINOR=") {
                    min = minor.parse::<u32>().unwrap_or(0);
                } else if let Some(devmode) = tline.strip_prefix("DEVMODE=") {
                    self.set_devnode_mode(u32::from_str_radix(devmode, 8).unwrap_or(0).into());
                }

                self.add_property_from_string(tline);

                if read == 0 {
                    break;
                }
            }

            self.set_devnum(libc::makedev(maj, min));

            Ok(())
        }
    }

    /// Parses the `property` string, and adds an [UdevEntry] to the properties list.
    pub fn add_property_from_string(&mut self, property: &str) -> Option<&UdevEntry> {
        let mut pit = property.split('=').take(2);

        let name = pit.next().unwrap_or("");
        let value = pit.next().unwrap_or("");

        self.add_property_internal(name, value)
    }

    fn add_property_internal(&mut self, key: &str, value: &str) -> Option<&UdevEntry> {
        if key.is_empty() {
            None
        } else {
            self.set_envp_uptodate(false);
            if value.is_empty() {
                // remove the matching property if it already exists
                self.properties_list_mut().remove_entry(key);
                None
            } else {
                self.properties_list_mut().add_entry(key, value)
            }
        }
    }

    /// Parses property string, and if needed, updates internal values accordingly.
    ///
    /// From `libudev` documentation:
    ///
    /// [add_property_from_string_parse_finish()](Self::add_property_from_string_parse_finish) needs to be
    /// called after adding properties, and its return value checked.
    ///
    /// [set_info_loaded()](Self::set_info_loaded) needs to be set, to avoid trying
    /// to use a device without a `DEVPATH` set.
    pub fn add_property_from_string_parse(&mut self, property: &str) -> Result<()> {
        if let Some(path) = property.strip_prefix("DEVPATH=") {
            self.set_syspath(path);
        } else if let Some(path) = property.strip_prefix("SUBSYSTEM=") {
            self.set_subsystem(path);
        } else if let Some(devtype) = property.strip_prefix("DEVTYPE=") {
            self.set_devtype(devtype);
        } else if let Some(devname) = property.strip_prefix("DEVNAME=") {
            self.set_devnode(devname);
        } else if let Some(devlinks) = property.strip_prefix("DEVLINKS=") {
            for link in devlinks.split(' ') {
                if !link.is_empty() && !link.starts_with('\0') {
                    self.add_devlink(link);
                }
            }
        } else if let Some(tags) = property.strip_prefix("TAGS=") {
            for tag in tags.split(':') {
                if !tag.is_empty() && !tag.starts_with('\0') {
                    self.add_tag(tag)?;
                }
            }
        } else if let Some(usec_init) = property.strip_prefix("USEC_INITIALIZED=") {
            self.set_usec_initialized(usec_init.parse::<u64>().unwrap_or(0));
        } else if let Some(driver) = property.strip_prefix("DRIVER=") {
            self.set_driver(driver);
        } else if let Some(action) = property.strip_prefix("ACTION=") {
            self.set_action(action);
        } else if let Some(major) = property.strip_prefix("MAJOR=") {
            self.set_maj(major.parse::<i32>().unwrap_or(0));
        } else if let Some(minor) = property.strip_prefix("MINOR=") {
            self.set_min(minor.parse::<i32>().unwrap_or(0));
        } else if let Some(devpath_old) = property.strip_prefix("DEVPATH_OLD=") {
            self.set_devpath_old(devpath_old);
        } else if let Some(seqnum) = property.strip_prefix("SEQNUM=") {
            self.set_seqnum(seqnum.parse::<u64>().unwrap_or(0));
        } else if let Some(ifindex) = property.strip_prefix("IFINDEX=") {
            self.set_ifindex(ifindex.parse::<i32>().unwrap_or(0));
        } else if let Some(devmode) = property.strip_prefix("DEVMODE=") {
            self.set_devnode_mode(devmode.parse::<u32>().unwrap_or(0).into());
        } else if let Some(devuid) = property.strip_prefix("DEVUID=") {
            self.set_devnode_uid(devuid.parse::<u32>().unwrap_or(0));
        } else if let Some(devgid) = property.strip_prefix("DEVGID=") {
            self.set_devnode_gid(devgid.parse::<u32>().unwrap_or(0));
        } else {
            self.add_property_from_string(property);
        }

        Ok(())
    }

    /// Finishes adding property information after parsing configuration string.
    ///
    /// **NOTE** users should call this function after the final call to
    /// (add_property_from_string_parse)[Self::add_property_from_string_parse].
    pub fn add_property_from_string_parse_finish(&mut self) -> Result<()> {
        if self.maj() > 0 {
            self.set_devnum(libc::makedev(self.maj() as u32, self.min() as u32));
        }

        self.set_maj(0);
        self.set_min(0);

        if self.devpath().is_empty() || self.subsystem().is_empty() {
            log::debug!("device: empty devpath and/or subsystem");
        }

        Ok(())
    }
}

impl Default for UdevDevice {
    fn default() -> Self {
        Self::new(Arc::new(Udev::new()))
    }
}

impl fmt::Display for UdevDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        write!(f, r#""syspath": {},"#, self.syspath)?;
        write!(f, r#""devpath": {},"#, self.devpath)?;
        write!(f, r#""sysname": {},"#, self.sysname)?;
        write!(f, r#""sysnum": {},"#, self.sysnum)?;
        write!(f, r#""devnode": {},"#, self.devnode)?;
        write!(f, r#""devnode_mode": {},"#, self.devnode_mode)?;
        write!(f, r#""devnode_uid": {},"#, self.devnode_uid)?;
        write!(f, r#""devnode_gid": {},"#, self.devnode_gid)?;
        write!(f, r#""subsystem": {},"#, self.subsystem)?;
        write!(f, r#""devtype": {},"#, self.devtype)?;
        write!(f, r#""driver": {},"#, self.driver)?;
        write!(f, r#""action": {},"#, self.action)?;
        write!(f, r#""devpath_old": {},"#, self.devpath_old)?;
        write!(f, r#""id_filename": {},"#, self.id_filename)?;
        write!(f, r#""monitor_buf": {},"#, self.monitor_buf)?;
        write!(f, r#""seqnum": {},"#, self.seqnum)?;
        write!(f, r#""usec_initialized": {},"#, self.usec_initialized)?;
        write!(f, r#""devlink_priority": {},"#, self.devlink_priority)?;
        write!(f, r#""devnum": {},"#, self.devnum)?;
        write!(f, r#""ifindex": {},"#, self.ifindex)?;
        write!(f, r#""watch_handle": {},"#, self.watch_handle)?;
        write!(f, r#""maj": {},"#, self.maj)?;
        write!(f, r#""min": {}"#, self.min)?;
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UdevEntry;

    #[test]
    fn test_udev_device() {
        let udev = Arc::new(Udev::new());
        let mut null_dev = UdevDevice::new(Arc::clone(&udev));

        let exp_syspath = "test_syspath";
        let exp_devpath = "test_devpath";
        let exp_sysname = "test_sysname";
        let exp_sysnum = "test_sysnum";
        let exp_devnode = "test_devnode";
        let exp_devnode_mode = Mode::EXEC_OTHER;
        let exp_subsystem = "test_subsystem";
        let exp_devtype = "test_devtype";
        let exp_driver = "test_driver";
        let exp_action = "test_action";
        let exp_devpath_old = "test_devpath_old";
        let exp_id_filename = "test_id_filename";
        let exp_envp = ["test_envp"];
        let exp_monitor_buf = "test_monitor_buf";

        let exp_devlinks = [UdevEntry::new().with_name("test_devlinks")];
        let exp_properties = [UdevEntry::new().with_name("test_properties")];
        let exp_sysattr_value = [UdevEntry::new().with_name("test_sysattr_value")];
        let exp_sysattr = [UdevEntry::new().with_name("test_sysattr")];
        let exp_tags = [UdevEntry::new().with_name("test_tags")];

        let exp_seqnum = 1;
        let exp_usec_initialized = 2;
        let exp_devlink_priority = 3;
        let exp_devnum = 4;
        let exp_ifindex = 5;
        let exp_watch_handle = 6;
        let exp_maj = 7;
        let exp_min = 8;

        let exp_devlinks_uptodate = true;
        let exp_envp_uptodate = true;
        let exp_tags_uptodate = true;
        let exp_info_loaded = true;
        let exp_db_loaded = true;
        let exp_uevent_loaded = true;
        let exp_is_initialized = true;
        let exp_sysattr_list_read = true;
        let exp_db_persist = true;

        let exp_dev = UdevDevice::new(udev)
            .with_syspath(exp_syspath)
            .with_devpath(exp_devpath)
            .with_sysname(exp_sysname)
            .with_sysnum(exp_sysnum)
            .with_devnode(exp_devnode)
            .with_devnode_mode(exp_devnode_mode)
            .with_subsystem(exp_subsystem)
            .with_devtype(exp_devtype)
            .with_driver(exp_driver)
            .with_action(exp_action)
            .with_devpath_old(exp_devpath_old)
            .with_id_filename(exp_id_filename)
            .with_envp(&exp_envp)
            .with_monitor_buf(exp_monitor_buf)
            .with_devlinks_list(exp_devlinks.clone())
            .with_properties_list(exp_properties.clone())
            .with_sysattr_value_list(exp_sysattr_value.clone())
            .with_sysattr_list(exp_sysattr.clone())
            .with_tags_list(exp_tags.clone())
            .with_seqnum(exp_seqnum)
            .with_usec_initialized(exp_usec_initialized)
            .with_devlink_priority(exp_devlink_priority)
            .with_devnum(exp_devnum)
            .with_ifindex(exp_ifindex)
            .with_watch_handle(exp_watch_handle)
            .with_maj(exp_maj)
            .with_min(exp_min)
            .with_devlinks_uptodate(exp_devlinks_uptodate)
            .with_envp_uptodate(exp_envp_uptodate)
            .with_tags_uptodate(exp_tags_uptodate)
            .with_info_loaded(exp_info_loaded)
            .with_db_loaded(exp_db_loaded)
            .with_uevent_loaded(exp_uevent_loaded)
            .with_is_initialized(exp_is_initialized)
            .with_sysattr_list_read(exp_sysattr_list_read)
            .with_db_persist(exp_db_persist);

        assert_eq!(null_dev.syspath(), "");
        assert_eq!(null_dev.devpath(), "");
        assert_eq!(null_dev.sysname(), "");
        assert_eq!(null_dev.sysnum(), "");
        assert_eq!(null_dev.devnode(), "");
        assert_eq!(null_dev.devnode_mode(), Mode::NONE);
        assert_eq!(null_dev.subsystem(), "");
        assert_eq!(null_dev.devtype(), "");
        assert_eq!(null_dev.driver(), "");
        assert_eq!(null_dev.action(), "");
        assert_eq!(null_dev.devpath_old(), "");
        assert_eq!(null_dev.id_filename(), "");
        assert!(null_dev.envp_is_empty());
        assert_eq!(null_dev.monitor_buf(), "");
        assert!(null_dev.devlinks_list().is_empty());
        assert!(null_dev.properties_list().is_empty());
        assert!(null_dev.sysattr_value_list().is_empty());
        assert!(null_dev.sysattr_list().is_empty());
        assert!(null_dev.tags_list().is_empty());
        assert_eq!(null_dev.seqnum(), 0);
        assert_eq!(null_dev.usec_initialized(), 0);
        assert_eq!(null_dev.devlink_priority(), 0);
        assert_eq!(null_dev.devnum(), 0);
        assert_eq!(null_dev.ifindex(), 0);
        assert_eq!(null_dev.watch_handle(), 0);
        assert_eq!(null_dev.maj(), 0);
        assert_eq!(null_dev.min(), 0);
        assert!(!null_dev.devlinks_uptodate());
        assert!(!null_dev.envp_uptodate());
        assert!(!null_dev.tags_uptodate());
        assert!(!null_dev.info_loaded());
        assert!(!null_dev.db_loaded());
        assert!(!null_dev.uevent_loaded());
        assert!(!null_dev.is_initialized());
        assert!(!null_dev.sysattr_list_read());
        assert!(!null_dev.db_persist());

        assert_eq!(exp_dev.syspath(), exp_syspath);
        assert_eq!(exp_dev.devpath(), exp_devpath);
        assert_eq!(exp_dev.sysname(), exp_sysname);
        assert_eq!(exp_dev.sysnum(), exp_sysnum);
        assert_eq!(exp_dev.devnode(), exp_devnode);
        assert_eq!(exp_dev.devnode_mode(), exp_devnode_mode);
        assert_eq!(exp_dev.subsystem(), exp_subsystem);
        assert_eq!(exp_dev.devtype(), exp_devtype);
        assert_eq!(exp_dev.driver(), exp_driver);
        assert_eq!(exp_dev.action(), exp_action);
        assert_eq!(exp_dev.devpath_old(), exp_devpath_old);
        assert_eq!(exp_dev.id_filename(), exp_id_filename);
        assert_eq!(exp_dev.envp(), exp_envp);
        assert_eq!(exp_dev.monitor_buf(), exp_monitor_buf);

        for (item, exp_item) in exp_dev.devlinks_list().iter().zip(exp_devlinks.iter()) {
            assert_eq!(item, exp_item);
        }

        for (item, exp_item) in exp_dev.properties_list().iter().zip(exp_properties.iter()) {
            assert_eq!(item, exp_item);
        }

        for (item, exp_item) in exp_dev
            .sysattr_value_list()
            .iter()
            .zip(exp_sysattr_value.iter())
        {
            assert_eq!(item, exp_item);
        }

        for (item, exp_item) in exp_dev.sysattr_list().iter().zip(exp_sysattr.iter()) {
            assert_eq!(item, exp_item);
        }

        for (item, exp_item) in exp_dev.tags_list().iter().zip(exp_tags.iter()) {
            assert_eq!(item, exp_item);
        }

        assert_eq!(exp_dev.seqnum(), exp_seqnum);
        assert_eq!(exp_dev.usec_initialized(), exp_usec_initialized);
        assert_eq!(exp_dev.devlink_priority(), exp_devlink_priority);
        assert_eq!(exp_dev.devnum(), exp_devnum);
        assert_eq!(exp_dev.ifindex(), exp_ifindex);
        assert_eq!(exp_dev.watch_handle(), exp_watch_handle);
        assert_eq!(exp_dev.maj(), exp_maj);
        assert_eq!(exp_dev.min(), exp_min);
        assert_eq!(exp_dev.devlinks_uptodate(), exp_devlinks_uptodate);
        assert_eq!(exp_dev.envp_uptodate(), exp_envp_uptodate);
        assert_eq!(exp_dev.tags_uptodate(), exp_tags_uptodate);
        assert_eq!(exp_dev.info_loaded(), exp_info_loaded);
        assert_eq!(exp_dev.db_loaded(), exp_db_loaded);
        assert_eq!(exp_dev.uevent_loaded(), exp_uevent_loaded);
        assert_eq!(exp_dev.is_initialized(), exp_is_initialized);
        assert_eq!(exp_dev.sysattr_list_read(), exp_sysattr_list_read);
        assert_eq!(exp_dev.db_persist(), exp_db_persist);

        null_dev.set_syspath(exp_syspath);
        null_dev.set_devpath(exp_devpath);
        null_dev.set_sysname(exp_sysname);
        null_dev.set_sysnum(exp_sysnum);
        null_dev.set_devnode(exp_devnode);
        null_dev.set_devnode_mode(exp_devnode_mode);
        null_dev.set_subsystem(exp_subsystem);
        null_dev.set_devtype(exp_devtype);
        null_dev.set_driver(exp_driver);
        null_dev.set_action(exp_action);
        null_dev.set_devpath_old(exp_devpath_old);
        null_dev.set_id_filename(exp_id_filename);
        null_dev.set_envp(&exp_envp);
        null_dev.set_monitor_buf(exp_monitor_buf);
        null_dev.set_devlinks_list(exp_devlinks.clone());
        null_dev.set_properties_list(exp_properties.clone());
        null_dev.set_sysattr_value_list(exp_sysattr_value.clone());
        null_dev.set_sysattr_list(exp_sysattr.clone());
        null_dev.set_tags_list(exp_tags.clone());
        null_dev.set_seqnum(exp_seqnum);
        null_dev.set_usec_initialized(exp_usec_initialized);
        null_dev.set_devlink_priority(exp_devlink_priority);
        null_dev.set_devnum(exp_devnum);
        null_dev.set_ifindex(exp_ifindex);
        null_dev.set_watch_handle(exp_watch_handle);
        null_dev.set_maj(exp_maj);
        null_dev.set_min(exp_min);
        null_dev.set_devlinks_uptodate(exp_devlinks_uptodate);
        null_dev.set_envp_uptodate(exp_envp_uptodate);
        null_dev.set_tags_uptodate(exp_tags_uptodate);
        null_dev.set_info_loaded(exp_info_loaded);
        null_dev.set_db_loaded(exp_db_loaded);
        null_dev.set_uevent_loaded(exp_uevent_loaded);
        null_dev.set_is_initialized(exp_is_initialized);
        null_dev.set_sysattr_list_read(exp_sysattr_list_read);
        null_dev.set_db_persist(exp_db_persist);

        assert_eq!(null_dev, exp_dev);
    }
}
