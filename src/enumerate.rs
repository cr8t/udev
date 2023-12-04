//! Lookup and sort `sys` devices.
//!
//! Lookup devices in the `sys` filesystem, filter devices by properties,
//! and return a sorted list of devices.
//!
//! From [`libudev-enumerate`](https://github.com/eudev-project/eudev/blob/master/src/libudev/libudev-enumerate.c) documentation.

use std::{fs, sync::Arc};

use crate::util;
use crate::UDEV_ROOT_RUN;
use crate::{Error, Result, Udev, UdevDevice, UdevEntry, UdevEntryList, UdevList};

const LOG_PREFIX: &str = "udev enumerate:";

/// Represents the file path in the `sys` filesystem.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Syspath {
    syspath: String,
}

impl Syspath {
    /// Creates a new [Syspath].
    pub const fn new() -> Self {
        Self {
            syspath: String::new(),
        }
    }

    /// Gets a reference to the syspath string.
    pub fn syspath(&self) -> &str {
        self.syspath.as_str()
    }

    /// Sets the syspath.
    pub fn set_syspath<S: Into<String>>(&mut self, syspath: S) {
        self.syspath = syspath.into();
    }

    /// Builder function that sets the syspath.
    pub fn with_syspath<S: Into<String>>(mut self, syspath: S) -> Self {
        self.set_syspath(syspath);
        self
    }
}

/// Represents one device lookup/sort context..
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UdevEnumerate {
    udev: Arc<Udev>,
    sysattr_match_list: UdevList,
    sysattr_nomatch_list: UdevList,
    subsystem_match_list: UdevList,
    subsystem_nomatch_list: UdevList,
    sysname_match_list: UdevList,
    properties_match_list: UdevList,
    tags_match_list: UdevList,
    devices_list: UdevList,
    parent: Option<Arc<UdevDevice>>,
    devices: Vec<Syspath>,
    devices_cur: usize,
    devices_max: usize,
    devices_uptodate: bool,
    match_is_initialized: bool,
}

impl UdevEnumerate {
    /// Creates a new [UdevEnumerate].
    pub fn new(udev: Arc<Udev>) -> Self {
        let sysattr_match_list = UdevList::new(Arc::clone(&udev));
        let sysattr_nomatch_list = UdevList::new(Arc::clone(&udev));
        let subsystem_match_list = UdevList::new(Arc::clone(&udev));
        let subsystem_nomatch_list = UdevList::new(Arc::clone(&udev));
        let sysname_match_list = UdevList::new(Arc::clone(&udev));
        let properties_match_list = UdevList::new(Arc::clone(&udev));
        let tags_match_list = UdevList::new(Arc::clone(&udev));
        let devices_list = UdevList::new(Arc::clone(&udev));

        Self {
            udev,
            sysattr_match_list,
            sysattr_nomatch_list,
            subsystem_match_list,
            subsystem_nomatch_list,
            sysname_match_list,
            properties_match_list,
            tags_match_list,
            devices_list,
            parent: None,
            devices: Vec::new(),
            devices_cur: 0,
            devices_max: 0,
            devices_uptodate: false,
            match_is_initialized: false,
        }
    }

    /// Gets a reference to the [Udev] object.
    pub fn udev(&self) -> &Arc<Udev> {
        &self.udev
    }

    /// Gets a reference to the sysattr match list [UdevList].
    pub const fn sysattr_match_list(&self) -> &UdevList {
        &self.sysattr_match_list
    }

    /// Gets a mutable reference to the sysattr match list [UdevList].
    pub fn sysattr_match_list_mut(&mut self) -> &mut UdevList {
        &mut self.sysattr_match_list
    }

    /// Sets the sysattr match list [UdevList].
    pub fn set_sysattr_match_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.sysattr_match_list.set_list(list);
    }

    /// Builder function that sets the sysattr match list [UdevList].
    pub fn with_sysattr_match_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_sysattr_match_list(list);
        self
    }

    /// Adds an entry to the match sysattr [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices with a given /sys device attribute.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_match_sysattr(&mut self, sysattr: &str) -> Result<&UdevEntry> {
        if sysattr.is_empty() {
            Err(Error::UdevEnumerate("sysattr is null".into()))
        } else {
            self.sysattr_match_list
                .add_entry(sysattr, "")
                .ok_or(Error::UdevEnumerate(
                    "unable to add match sysattr entry".into(),
                ))
        }
    }

    /// Gets a reference to the sysattr nomatch list [UdevList].
    pub const fn sysattr_nomatch_list(&self) -> &UdevList {
        &self.sysattr_nomatch_list
    }

    /// Gets a mutable reference to the sysattr nomatch list [UdevList].
    pub fn sysattr_nomatch_list_mut(&mut self) -> &mut UdevList {
        &mut self.sysattr_nomatch_list
    }

    /// Sets the sysattr nomatch list [UdevList].
    pub fn set_sysattr_nomatch_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.sysattr_nomatch_list.set_list(list);
    }

    /// Builder function that sets the sysattr nomatch list [UdevList].
    pub fn with_sysattr_nomatch_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_sysattr_nomatch_list(list);
        self
    }

    /// Adds an entry to the no-match sysattr [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices without a given /sys device attribute.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_nomatch_sysattr(&mut self, sysattr: &str) -> Result<&UdevEntry> {
        if sysattr.is_empty() {
            Err(Error::UdevEnumerate("sysattr is null".into()))
        } else {
            self.sysattr_nomatch_list
                .add_entry(sysattr, "")
                .ok_or(Error::UdevEnumerate(
                    "unable to add no-match sysattr entry".into(),
                ))
        }
    }

    /// Gets a reference to the subsystem match list [UdevList].
    pub const fn subsystem_match_list(&self) -> &UdevList {
        &self.subsystem_match_list
    }

    /// Gets a mutable reference to the subsystem match list [UdevList].
    pub fn subsystem_match_list_mut(&mut self) -> &mut UdevList {
        &mut self.subsystem_match_list
    }

    /// Sets the subsystem match list [UdevList].
    pub fn set_subsystem_match_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.subsystem_match_list.set_list(list);
    }

    /// Builder function that sets the subsystem match list [UdevList].
    pub fn with_subsystem_match_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_subsystem_match_list(list);
        self
    }

    /// Adds an entry to the match subsystem [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices belonging to a certain kernel subsystem.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_match_subsystem(&mut self, subsystem: &str) -> Result<&UdevEntry> {
        if subsystem.is_empty() {
            Err(Error::UdevEnumerate("subsystem is null".into()))
        } else {
            self.subsystem_match_list
                .add_entry(subsystem, "")
                .ok_or(Error::UdevEnumerate(
                    "unable to add match subsystem entry".into(),
                ))
        }
    }

    /// Gets a reference to the subsystem nomatch list [UdevList].
    pub const fn subsystem_nomatch_list(&self) -> &UdevList {
        &self.subsystem_nomatch_list
    }

    /// Gets a mutable reference to the subsystem nomatch list [UdevList].
    pub fn subsystem_nomatch_list_mut(&mut self) -> &mut UdevList {
        &mut self.subsystem_nomatch_list
    }

    /// Sets the subsystem nomatch list [UdevList].
    pub fn set_subsystem_nomatch_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.subsystem_nomatch_list.set_list(list);
    }

    /// Builder function that sets the subsystem nomatch list [UdevList].
    pub fn with_subsystem_nomatch_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_subsystem_nomatch_list(list);
        self
    }

    /// Adds an entry to the no-match subsystem [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices not belonging to a certain kernel subsystem.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_nomatch_subsystem(&mut self, subsystem: &str) -> Result<&UdevEntry> {
        if subsystem.is_empty() {
            Err(Error::UdevEnumerate("subsystem is null".into()))
        } else {
            self.subsystem_nomatch_list
                .add_entry(subsystem, "")
                .ok_or(Error::UdevEnumerate(
                    "unable to add no-match subsystem entry".into(),
                ))
        }
    }

    /// Gets a reference to the sysname match list [UdevList].
    pub const fn sysname_match_list(&self) -> &UdevList {
        &self.sysname_match_list
    }

    /// Gets a mutable reference to the sysname match list [UdevList].
    pub fn sysname_match_list_mut(&mut self) -> &mut UdevList {
        &mut self.sysname_match_list
    }

    /// Sets the sysname match list [UdevList].
    pub fn set_sysname_match_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.sysname_match_list.set_list(list);
    }

    /// Builder function that sets the sysname match list [UdevList].
    pub fn with_sysname_match_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_sysname_match_list(list);
        self
    }

    /// Adds an entry to the match sysname [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices with a given /sys device name.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_match_sysname(&mut self, sysname: &str) -> Result<&UdevEntry> {
        if sysname.is_empty() {
            Err(Error::UdevEnumerate("sysname is null".into()))
        } else {
            self.sysname_match_list
                .add_entry(sysname, "")
                .ok_or(Error::UdevEnumerate(
                    "unable to add match sysname entry".into(),
                ))
        }
    }

    /// Gets a reference to the properties match list [UdevList].
    pub const fn properties_match_list(&self) -> &UdevList {
        &self.properties_match_list
    }

    /// Gets a mutable reference to the properties match list [UdevList].
    pub fn properties_match_list_mut(&mut self) -> &mut UdevList {
        &mut self.properties_match_list
    }

    /// Sets the properties match list [UdevList].
    pub fn set_properties_match_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.properties_match_list.set_list(list);
    }

    /// Builder function that sets the properties match list [UdevList].
    pub fn with_properties_match_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_properties_match_list(list);
        self
    }

    /// Adds an entry to the match properties [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices with a certain property.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_match_property(&mut self, property: &str, value: &str) -> Result<&UdevEntry> {
        if property.is_empty() {
            Err(Error::UdevEnumerate("property is null".into()))
        } else {
            self.properties_match_list
                .add_entry(property, value)
                .ok_or(Error::UdevEnumerate(
                    "unable to add match property entry".into(),
                ))
        }
    }

    /// Gets a reference to the tags match list [UdevList].
    pub const fn tags_match_list(&self) -> &UdevList {
        &self.tags_match_list
    }

    /// Gets a mutable reference to the tags match list [UdevList].
    pub fn tags_match_list_mut(&mut self) -> &mut UdevList {
        &mut self.tags_match_list
    }

    /// Sets the tags match list [UdevList].
    pub fn set_tags_match_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.tags_match_list.set_list(list);
    }

    /// Builder function that sets the tags match list [UdevList].
    pub fn with_tags_match_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_tags_match_list(list);
        self
    }

    /// Adds an entry to the match tags [UdevEntry] list.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Match only devices with a certain tag.
    /// ```
    ///
    /// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
    pub fn add_match_tag(&mut self, tag: &str) -> Result<&UdevEntry> {
        if tag.is_empty() {
            Err(Error::UdevEnumerate("tag is null".into()))
        } else {
            self.tags_match_list
                .add_entry(tag, "")
                .ok_or(Error::UdevEnumerate("unable to add match tag entry".into()))
        }
    }

    /// Gets a reference to the devices list [UdevList].
    pub const fn devices_list(&self) -> &UdevList {
        &self.devices_list
    }

    /// Gets a mutable reference to the devices list [UdevList].
    pub fn devices_list_mut(&mut self) -> &mut UdevList {
        &mut self.devices_list
    }

    /// Sets the devices list [UdevList].
    pub fn set_devices_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.devices_list.set_list(list);
    }

    /// Builder function that sets the devices list [UdevList].
    pub fn with_devices_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_devices_list(list);
        self
    }

    /// Gets an optional reference to the parent [UdevDevice].
    pub fn parent(&self) -> Option<&Arc<UdevDevice>> {
        self.parent.as_ref()
    }

    /// Sets the parent [UdevDevice].
    pub fn set_parent(&mut self, parent: Arc<UdevDevice>) {
        self.parent.replace(parent);
    }

    /// Builder function that sets the parent [UdevDevice].
    pub fn with_parent(mut self, parent: Arc<UdevDevice>) -> Self {
        self.set_parent(parent);
        self
    }

    /// Gets a reference to the [Syspath] devices list.
    pub fn devices(&self) -> &[Syspath] {
        self.devices.as_ref()
    }

    /// Sets the [Syspath] devices list.
    pub fn set_devices<S: IntoIterator<Item = Syspath>>(&mut self, devices: S) {
        self.devices = devices.into_iter().collect();
    }

    /// Builder function that sets the [Syspath] devices list.
    pub fn with_devices<S: IntoIterator<Item = Syspath>>(mut self, devices: S) -> Self {
        self.set_devices(devices);
        self
    }

    /// Gets the current index into the [Syspath] devices list.
    pub const fn devices_cur(&self) -> usize {
        self.devices_cur
    }

    /// Sets the current index into the [Syspath] devices list.
    ///
    /// **NOTE**: `cur` must be in-bounds to set the current index, otherwise this function is a
    /// no-op.
    pub fn set_devices_cur(&mut self, cur: usize) {
        if cur < self.devices.len() {
            self.devices_cur = cur;
        }
    }

    /// Builder function that sets the current index into the [Syspath] devices list.
    ///
    /// **NOTE**: `cur` must be in-bounds to set the current index, otherwise this function is a
    /// no-op.
    pub fn with_devices_cur(mut self, cur: usize) -> Self {
        self.set_devices_cur(cur);
        self
    }

    /// Gets the maximum number of [Syspath] devices.
    pub const fn devices_max(&self) -> usize {
        self.devices_max
    }

    /// Sets the maximum number of [Syspath] devices.
    ///
    /// **NOTE** if `max` is greater than the current [Syspath] devices list capacity,
    /// additional space will be reserved to avoid frequent reallocations.
    pub fn set_devices_max(&mut self, max: usize) {
        self.devices_max = max;
        let cap = self.devices.capacity();
        self.devices.reserve(self.devices_max.saturating_sub(cap));
    }

    /// Builder function that sets the maximum number of [Syspath] devices.
    ///
    /// **NOTE** if `max` is greater than the current [Syspath] devices list capacity,
    /// additional space will be reserved to avoid frequent reallocations.
    pub fn with_devices_max(mut self, max: usize) -> Self {
        self.set_devices_max(max);
        self
    }

    /// Gets whether the [Syspath] devices list is up-to-date.
    pub const fn devices_uptodate(&self) -> bool {
        self.devices_uptodate
    }

    /// Sets whether the [Syspath] devices list is up-to-date.
    pub fn set_devices_uptodate(&mut self, val: bool) {
        self.devices_uptodate = val;
    }

    /// Builder function that sets whether the [Syspath] devices list is up-to-date.
    pub fn with_devices_uptodate(mut self, val: bool) -> Self {
        self.set_devices_uptodate(val);
        self
    }

    /// Gets whether the match lists are initialized.
    pub const fn match_is_initialized(&self) -> bool {
        self.match_is_initialized
    }

    /// Sets whether the match lists are initialized.
    pub fn set_match_is_initialized(&mut self, val: bool) {
        self.match_is_initialized = val;
    }

    /// Builder function that sets whether the match lists are initialized.
    pub fn with_match_is_initialized(mut self, val: bool) -> Self {
        self.set_match_is_initialized(val);
        self
    }

    /// Adds a devices to the list of devices.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Add a device to the list of devices, to retrieve it back sorted in dependency order.
    /// ```
    ///
    /// Returns: `Ok(())` on success, `Err(Error)` otherwise.
    pub fn add_syspath(&mut self, syspath: &str) -> Result<()> {
        if syspath.is_empty() {
            Err(Error::UdevEnumerate("empty syspath".into()))
        } else {
            let dev = UdevDevice::new_from_syspath(Arc::clone(&self.udev), syspath)?;
            self.syspath_add(dev.syspath())
        }
    }

    fn syspath_add(&mut self, syspath: &str) -> Result<()> {
        if self.devices_cur >= self.devices_max {
            self.devices_max = self.devices_max.saturating_add(1024);
            self.devices.reserve(1024);
        }

        self.devices.push(Syspath::new().with_syspath(syspath));

        self.devices_cur = self.devices_cur.saturating_add(1);
        self.devices_uptodate = false;

        Ok(())
    }

    /// Scan `/sys` for devices which match the given filters.
    ///
    /// From the `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Scan /sys for all devices which match the given filters. No matches
    /// will return all currently available devices.
    /// ```
    ///
    /// Returns: `Ok(())` on success, `Err(Error)` otherwise.
    pub fn scan_devices(&mut self) -> Result<()> {
        if self.tags_match_list.entry().is_some() {
            self.scan_devices_tags()
        } else if self.parent.is_some() {
            self.scan_devices_children()
        } else {
            self.scan_devices_all()
        }
    }

    fn scan_devices_tags(&mut self) -> Result<()> {
        // scan only tagged devices.
        // use tags reverse-index, instead of searching all deivces in /sys
        let mut add_syspaths: Vec<String> = Vec::new();

        for list_entry in self.tags_match_list.iter() {
            let tag_name = list_entry.name();
            let path = format!("{UDEV_ROOT_RUN}/udev/tags/{tag_name}");

            for dir_entry in fs::read_dir(path.as_str())
                .map_err(|err| Error::UdevEnumerate(format!("unable to open tags path: {err}")))?
                .filter(|e| e.is_ok())
            {
                let d_name = dir_entry?
                    .file_name()
                    .into_string()
                    .unwrap_or(String::new());
                if d_name.is_empty() {
                    log::trace!("{LOG_PREFIX} empty/invalid entry");
                } else if d_name.starts_with('.') {
                    log::trace!("{LOG_PREFIX} private entry");
                } else {
                    let mut dev =
                        UdevDevice::new_from_device_id(Arc::clone(&self.udev), d_name.as_str())?;
                    let dev_syspath = dev.syspath().to_owned();

                    if !self.match_subsystem(dev.subsystem()) {
                        log::trace!("{LOG_PREFIX} no subsystem match");
                    } else if !self.match_sysname(dev.sysname()) {
                        log::trace!("{LOG_PREFIX} no sysname match");
                    } else if !self.match_parent(&dev) {
                        log::trace!("{LOG_PREFIX} no parent match");
                    } else if !self.match_property(&dev) {
                        log::trace!("{LOG_PREFIX} no property match");
                    } else if !self.match_sysattr(&mut dev) {
                        log::trace!("{LOG_PREFIX} no sys attribute match");
                    } else {
                        add_syspaths.push(dev_syspath);
                    }
                }
            }
        }

        for path in add_syspaths.iter() {
            self.syspath_add(path)?;
        }

        Ok(())
    }

    fn match_subsystem(&self, subsystem: &str) -> bool {
        !subsystem.is_empty()
            && !self
                .subsystem_nomatch_list
                .iter()
                .any(|f| f.name() == subsystem)
            && (self.subsystem_match_list.is_empty()
                || self
                    .subsystem_match_list
                    .iter()
                    .any(|f| f.name() == subsystem))
    }

    fn match_sysname(&self, sysname: &str) -> bool {
        !sysname.is_empty()
            && (self.sysname_match_list.is_empty()
                || self.sysname_match_list.iter().any(|f| f.name() == sysname))
    }

    fn match_parent(&self, dev: &UdevDevice) -> bool {
        match self.parent.as_ref() {
            Some(parent) => dev.devpath().starts_with(parent.devpath()),
            None => true,
        }
    }

    fn match_tag(&self, dev: &mut UdevDevice) -> bool {
        // no match always matches
        self.tags_match_list.is_empty() ||
            // loop over matches
            // if any tag is a mismatch, return false
            self.tags_match_list.iter().filter(|f| !dev.has_tag(f.name())).count() == 0
    }

    fn match_property(&self, dev: &UdevDevice) -> bool {
        if self.properties_match_list.is_empty() {
            true
        } else {
            let mut ret = false;
            for list_entry in self.properties_match_list.iter() {
                let match_key = list_entry.name();
                let match_value = list_entry.value();

                for property_entry in dev.properties_list().iter() {
                    let dev_key = property_entry.name();
                    let dev_value = property_entry.value();

                    if let Ok(key_pattern) = glob::Pattern::new(match_key) {
                        if !key_pattern.matches(dev_key) {
                            log::trace!(
                                "no key match found, entry key: {match_key}, device key {dev_key}"
                            );
                        } else if match_value.is_empty() && dev_value.is_empty() {
                            ret = true;
                            break;
                        } else if let Ok(val_pattern) = glob::Pattern::new(match_value) {
                            if val_pattern.matches(dev_value) {
                                ret = true;
                                break;
                            }
                        } else {
                            log::trace!("no value match found, entry value: {match_value}, device value: {dev_value}");
                        }
                    }
                }

                if ret {
                    break;
                }
            }

            ret
        }
    }

    fn match_sysattr(&self, dev: &mut UdevDevice) -> bool {
        !self
            .sysattr_nomatch_list
            .iter()
            .any(|f| dev.match_sysattr_value(f.name(), f.value()))
            && self
                .sysattr_match_list
                .iter()
                .filter(|f| !dev.match_sysattr_value(f.name(), f.value()))
                .count()
                == 0
            && self
                .sysattr_match_list
                .iter()
                .any(|f| dev.match_sysattr_value(f.name(), f.value()))
    }

    fn scan_devices_children(&mut self) -> Result<()> {
        Err(Error::UdevEnumerate("unimplemented".into()))
    }

    fn scan_devices_all(&mut self) -> Result<()> {
        Err(Error::UdevEnumerate("unimplemented".into()))
    }

    /// Scans `/sys` for all kernel subsystems.
    ///
    /// From `libudev` documentation:
    ///
    /// ```no_build,no_run
    /// Scan /sys for all kernel subsystems, including buses, classes, drivers.
    /// ```
    ///
    /// Returns: `Ok(())` on success, `Err(Error)` otherwise.
    pub fn scan_subsystems(&mut self) -> Result<()> {
        // all kernel modules
        if self.match_subsystem("module") {
            self.scan_dir_and_add_devices("module", "", "")?;
        }

        let subsysdir = if fs::metadata("/sys/subsystem").is_ok() {
            "subsystem"
        } else {
            "bus"
        };

        // all subsystems (only buses support coldplug)
        if self.match_subsystem("subsystem") {
            self.scan_dir_and_add_devices(subsysdir, "", "")?;
        }

        // all subsystem drivers
        if self.match_subsystem("drivers") {
            self.scan_dir(subsysdir, "drivers", "drivers")?;
        }

        Ok(())
    }

    fn scan_dir_and_add_devices(
        &mut self,
        basedir: &str,
        subdir1: &str,
        subdir2: &str,
    ) -> Result<()> {
        let path = if !subdir1.is_empty() && !subdir2.is_empty() {
            format!("/sys/{basedir}/{subdir1}/{subdir2}")
        } else if !subdir1.is_empty() {
            format!("/sys/{basedir}/{subdir1}")
        } else if !subdir2.is_empty() {
            format!("/sys/{basedir}/{subdir2}")
        } else {
            format!("/sys/{basedir}")
        };

        let mut add_syspaths: Vec<String> = Vec::new();

        for dir_entry in fs::read_dir(path.as_str())
            .map_err(|err| Error::UdevEnumerate(format!("unable to open {path} path: {err}")))?
        {
            let d_name = dir_entry?
                .file_name()
                .into_string()
                .unwrap_or(String::new());

            if d_name.is_empty() {
                log::trace!("{LOG_PREFIX} empty/invalid entry");
            } else if d_name.starts_with('.') {
                log::trace!("{LOG_PREFIX} private entry");
            } else if !self.match_sysname(d_name.as_str()) {
                log::trace!("{LOG_PREFIX} no /sys name match");
            } else {
                let syspath = format!("{path}/{d_name}");
                if let Ok(mut dev) =
                    UdevDevice::new_from_syspath(Arc::clone(&self.udev), syspath.as_str())
                {
                    if self.match_is_initialized {
                        // From `libudev` documentation:
                        //
                        // ```
                        // All devices with a device node or network interfaces
                        // possibly need udev to adjust the device node permission
                        // or context, or rename the interface before it can be
                        // reliably used from other processes.
                        //
                        // For now, we can only check these types of devices, we
                        // might not store a database, and have no way to find out
                        // for all other types of devices.
                        // ```
                        if dev.get_is_initialized()
                            && (util::major(dev.devnum()) > 0 || dev.get_ifindex() > 0)
                        {
                            break;
                        }
                    }
                    let dev_syspath = dev.syspath().to_owned();
                    if !self.match_parent(&dev) {
                        log::trace!("{LOG_PREFIX} no parent match");
                        break;
                    } else if !self.match_tag(&mut dev) {
                        log::trace!("{LOG_PREFIX} no tag match");
                        break;
                    } else if !self.match_property(&dev) {
                        log::trace!("{LOG_PREFIX} no property match");
                        break;
                    } else if !self.match_sysattr(&mut dev) {
                        log::trace!("{LOG_PREFIX} no /sys attribute match");
                        break;
                    } else {
                        add_syspaths.push(dev_syspath);
                    }
                }
            }
        }

        for syspath in add_syspaths.iter() {
            self.add_syspath(syspath)?;
        }

        Ok(())
    }

    fn scan_dir(&mut self, basedir: &str, subdir: &str, subsystem: &str) -> Result<()> {
        let path = format!("/sys/{basedir}");

        for dir_entry in fs::read_dir(path.as_str())
            .map_err(|err| Error::UdevEnumerate(format!("unable to open {path} path: {err}")))?
        {
            let d_name = dir_entry?
                .file_name()
                .into_string()
                .unwrap_or(String::new());

            if d_name.is_empty() {
                log::trace!("{LOG_PREFIX} empty/invalid entry");
            } else if d_name.starts_with('.') {
                log::trace!("{LOG_PREFIX} private entry");
            } else if !self.match_sysname(if subsystem.is_empty() {
                d_name.as_str()
            } else {
                subsystem
            }) {
                log::trace!("{LOG_PREFIX} no /sys subsystem name match");
            } else {
                self.scan_dir_and_add_devices(basedir, d_name.as_str(), subdir)?;
            }
        }

        Ok(())
    }
}

impl UdevDevice {
    pub(crate) fn match_sysattr_value(&mut self, sysattr: &str, match_val: &str) -> bool {
        match self.get_sysattr_value(sysattr) {
            Some(val) => {
                if match_val.is_empty() {
                    true
                } else if let Ok(pattern) = glob::Pattern::new(match_val) {
                    pattern.matches(val.as_str())
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UdevEntry;

    #[test]
    fn test_udev_enumerate() {
        let udev = Arc::new(Udev::new());

        let mut null_enum = UdevEnumerate::new(Arc::clone(&udev));

        assert_eq!(null_enum.udev(), &udev);
        assert!(null_enum.sysattr_match_list().is_empty());
        assert!(null_enum.sysattr_nomatch_list().is_empty());
        assert!(null_enum.subsystem_match_list().is_empty());
        assert!(null_enum.subsystem_nomatch_list().is_empty());
        assert!(null_enum.sysname_match_list().is_empty());
        assert!(null_enum.properties_match_list().is_empty());
        assert!(null_enum.tags_match_list().is_empty());
        assert!(null_enum.devices_list().is_empty());
        assert!(null_enum.parent().is_none());
        assert!(null_enum.devices().is_empty());
        assert_eq!(null_enum.devices_cur(), 0);
        assert_eq!(null_enum.devices_max(), 0);
        assert!(!null_enum.devices_uptodate());
        assert!(!null_enum.match_is_initialized());

        let exp_sysattr_match_list = [UdevEntry::new().with_name("test_sysattr_match_list")];
        let exp_sysattr_nomatch_list = [UdevEntry::new().with_name("test_sysattr_nomatch_list")];
        let exp_subsystem_match_list = [UdevEntry::new().with_name("test_subsystem_match_list")];
        let exp_subsystem_nomatch_list =
            [UdevEntry::new().with_name("test_subsystem_nomatch_list")];
        let exp_sysname_match_list = [UdevEntry::new().with_name("test_sysname_match_list")];
        let exp_properties_match_list = [UdevEntry::new().with_name("test_properties_match_list")];
        let exp_tags_match_list = [UdevEntry::new().with_name("test_tags_match_list")];
        let exp_devices_list = [UdevEntry::new().with_name("test_devices_list")];
        let exp_parent = Arc::new(UdevDevice::new(Arc::clone(&udev)));
        let exp_devices = [Syspath::new(), Syspath::new()];
        let exp_devices_cur = 1;
        let exp_devices_max = 4;
        let exp_devices_uptodate = true;
        let exp_match_is_initialized = true;

        let exp_enum = UdevEnumerate::new(Arc::clone(&udev))
            .with_sysattr_match_list(exp_sysattr_match_list.clone())
            .with_sysattr_nomatch_list(exp_sysattr_nomatch_list.clone())
            .with_subsystem_match_list(exp_subsystem_match_list.clone())
            .with_subsystem_nomatch_list(exp_subsystem_nomatch_list.clone())
            .with_sysname_match_list(exp_sysname_match_list.clone())
            .with_properties_match_list(exp_properties_match_list.clone())
            .with_tags_match_list(exp_tags_match_list.clone())
            .with_devices_list(exp_devices_list.clone())
            .with_parent(exp_parent.clone())
            .with_devices(exp_devices.clone())
            .with_devices_cur(exp_devices_cur)
            .with_devices_max(exp_devices_max)
            .with_devices_uptodate(exp_devices_uptodate)
            .with_match_is_initialized(exp_match_is_initialized);

        assert_eq!(exp_enum.udev(), &udev);

        for iter in [
            exp_enum
                .sysattr_match_list()
                .iter()
                .zip(exp_sysattr_match_list.iter()),
            exp_enum
                .sysattr_nomatch_list()
                .iter()
                .zip(exp_sysattr_nomatch_list.iter()),
            exp_enum
                .subsystem_match_list()
                .iter()
                .zip(exp_subsystem_match_list.iter()),
            exp_enum
                .subsystem_nomatch_list()
                .iter()
                .zip(exp_subsystem_nomatch_list.iter()),
            exp_enum
                .sysname_match_list()
                .iter()
                .zip(exp_sysname_match_list.iter()),
            exp_enum
                .properties_match_list()
                .iter()
                .zip(exp_properties_match_list.iter()),
            exp_enum
                .tags_match_list()
                .iter()
                .zip(exp_tags_match_list.iter()),
            exp_enum.devices_list().iter().zip(exp_devices_list.iter()),
        ] {
            for (entry, exp_entry) in iter {
                assert_eq!(entry, exp_entry);
            }
        }

        assert_eq!(exp_enum.parent(), Some(&exp_parent));
        assert_eq!(exp_enum.devices(), exp_devices.as_ref());
        assert_eq!(exp_enum.devices_cur(), exp_devices_cur);
        assert_eq!(exp_enum.devices_max(), exp_devices_max);
        assert_eq!(exp_enum.devices_uptodate(), exp_devices_uptodate);
        assert_eq!(exp_enum.match_is_initialized(), exp_match_is_initialized);

        null_enum.set_sysattr_match_list(exp_sysattr_match_list.clone());
        null_enum.set_sysattr_nomatch_list(exp_sysattr_nomatch_list.clone());
        null_enum.set_subsystem_match_list(exp_subsystem_match_list.clone());
        null_enum.set_subsystem_nomatch_list(exp_subsystem_nomatch_list.clone());
        null_enum.set_sysname_match_list(exp_sysname_match_list.clone());
        null_enum.set_properties_match_list(exp_properties_match_list.clone());
        null_enum.set_tags_match_list(exp_tags_match_list.clone());
        null_enum.set_devices_list(exp_devices_list.clone());
        null_enum.set_parent(exp_parent.clone());
        null_enum.set_devices(exp_devices.clone());
        null_enum.set_devices_cur(exp_devices_cur);
        null_enum.set_devices_max(exp_devices_max);
        null_enum.set_devices_uptodate(exp_devices_uptodate);
        null_enum.set_match_is_initialized(exp_match_is_initialized);

        assert_eq!(null_enum, exp_enum);
    }
}
