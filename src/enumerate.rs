//! Lookup and sort `sys` devices.
//!
//! Lookup devices in the `sys` filesystem, filter devices by properties,
//! and return a sorted list of devices.
//!
//! From [`libudev-enumerate`](https://github.com/systemd/systemd/blob/869c1cf88fdb17681ec2cc274d04622f6f21e95c/src/libudev/libudev-enumerate.c) documentation.

use std::sync::Arc;

use crate::{Udev, UdevDevice, UdevEntryList, UdevList};

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
