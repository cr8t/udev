//! Pure Rust library for interacting with the [udev](https://www.kernel.org/doc/ols/2003/ols2003-pages-249-257.pdf) userspace `devfs`.
//!
//! Uses the [`libc`](https://crates.io/crates/libc) and [`nix`](https://crates.io/crates/nix) crate to make syscalls to Linux.

use std::sync::Arc;

#[macro_use]
extern crate bitflags;

mod context;
mod device;
mod enumerate;
mod error;
mod file;
mod hwdb;
mod list;
mod log;
mod mode;
mod monitor;
mod murmur_hash;
mod queue;
mod socket;
mod util;

pub use context::*;
pub use device::*;
pub use enumerate::*;
pub use error::*;
pub use file::*;
pub use hwdb::*;
pub use list::*;
pub use log::*;
pub use mode::*;
pub use monitor::*;
pub use murmur_hash::*;
pub use queue::*;
pub use socket::*;
pub use util::*;

/// Creates a new [Udev] context.
pub fn udev_new() -> Arc<Udev> {
    Arc::new(Udev::new())
}

/// Gets the [LogPriority] for the [Udev] context.
pub fn udev_get_log_priority(udev: &Udev) -> LogPriority {
    udev.log_priority()
}

/// Sets the [LogPriority] for the [Udev] context.
pub fn udev_set_log_priority(udev: &mut Udev, val: LogPriority) {
    udev.set_log_priority(val);
}

/// Gets a reference to the next entry in a [UdevList].
///
/// Breaks with the original `libudev` API by requiring a reference to the list, instead of a list
/// entry. This is because the C version uses a linked-list composed of pointers, we don't.
pub fn udev_list_entry_get_next(list: &UdevList) -> Option<&UdevEntry> {
    list.next_entry()
}

/// Gets a mutable reference to the next entry in a [UdevList].
///
/// Breaks with the original `libudev` API by requiring a reference to the list, instead of a list
/// entry. This is because the C version uses a linked-list composed of pointers, we don't.
pub fn udev_list_entry_get_next_mut(list: &mut UdevList) -> Option<&mut UdevEntry> {
    list.next_entry_mut()
}

/// Gets the name of the [UdevEntry].
pub fn udev_list_entry_get_name(entry: &UdevEntry) -> &str {
    entry.name()
}

/// Gets the value of the [UdevEntry].
pub fn udev_list_entry_get_value(entry: &UdevEntry) -> &str {
    entry.value()
}

/// Helper function that iterates over every [UdevEntry] in the list, applying the function to each
/// entry.
///
/// Breaks with the original `libudev` API by requiring a reference to the list, instead of a list
/// entry. This is because the C version uses a linked-list composed of pointers, we don't.
pub fn udev_list_entry_foreach(list: &UdevList, f: fn(&UdevEntry) -> Result<()>) -> Result<()> {
    for entry in list.iter() {
        f(entry)?;
    }
    Ok(())
}

/// Helper function that iterates over every [UdevEntry] in the list, applying the function to each
/// entry.
///
/// Breaks with the original `libudev` API by requiring a reference to the list, instead of a list
/// entry. This is because the C version uses a linked-list composed of pointers, we don't.
pub fn udev_list_entry_foreach_mut(
    list: &mut UdevList,
    f: fn(&mut UdevEntry) -> Result<()>,
) -> Result<()> {
    for entry in list.iter_mut() {
        f(entry)?;
    }
    Ok(())
}

/// Creates a new [UdevDevice] from the provided [Udev] context.
pub fn udev_device_new(udev: Arc<Udev>) -> UdevDevice {
    UdevDevice::new(udev)
}

/// Gets a reference to the [Udev] context from an [UdevDevice].
pub fn udev_device_get_udev(device: &UdevDevice) -> &Udev {
    device.udev()
}

/// Gets a cloned reference to the [Udev] context from an [UdevDevice].
pub fn udev_device_get_udev_cloned(device: &UdevDevice) -> Arc<Udev> {
    device.udev_cloned()
}

/// Creates new [UdevDevice], and fills in information from the sys
/// device and the udev database entry.
///
/// The `syspath` is the absolute path to the device, including the sys mount point.
///
/// The initial refcount is 1, and needs to be decremented to release the resources of the udev device.
///
/// Returns: a new [UdevDevice], or `Error`, if it does not exist
pub fn udev_device_new_from_syspath(udev: Arc<Udev>, syspath: &str) -> Result<UdevDevice> {
    UdevDevice::new_from_syspath(udev, syspath)
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
pub fn udev_device_new_from_devnum(
    udev: Arc<Udev>,
    devtype: &str,
    devnum: libc::dev_t,
) -> Result<UdevDevice> {
    UdevDevice::new_from_devnum(udev, devtype, devnum)
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
pub fn udev_device_new_from_subsystem_sysname(
    udev: Arc<Udev>,
    subsystem: &str,
    sysname: &str,
) -> Result<UdevDevice> {
    UdevDevice::new_from_subsystem_sysname(udev, subsystem, sysname)
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
pub fn udev_device_new_from_device_id(udev: Arc<Udev>, id: &str) -> Result<UdevDevice> {
    UdevDevice::new_from_device_id(udev, id)
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
pub fn udev_device_new_from_environment(udev: Arc<Udev>) -> Result<UdevDevice> {
    UdevDevice::new_from_environment(udev)
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
pub fn udev_device_get_parent(dev: &mut UdevDevice) -> Result<Arc<UdevDevice>> {
    dev.get_parent()
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
pub fn udev_device_get_parent_with_subsystem_devtype(
    dev: &mut UdevDevice,
    subsystem: &str,
    devtype: &str,
) -> Result<Arc<UdevDevice>> {
    dev.get_parent_with_subsystem_devtype(subsystem, devtype)
}

/// Reads [UdevDevice] information from the persistent database file.
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise
pub fn udev_device_read_db(dev: &mut UdevDevice) -> Result<()> {
    dev.read_db()
}

/// Gets the [UdevDevice] device path.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Retrieve the kernel devpath value of the udev device. The path
/// does not contain the sys mount point, and starts with a '/'.
/// ```
pub fn udev_device_get_devpath(dev: &UdevDevice) -> &str {
    dev.devpath()
}

/// Gets the [UdevDevice] `syspath`.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Retrieve the sys path of the udev device. The path is an
/// absolute path and starts with the sys mount point.
/// ```
pub fn udev_device_get_syspath(dev: &UdevDevice) -> &str {
    dev.syspath()
}

/// Gets the [UdevDevice] `sysname`.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Get the kernel device name in /sys.
/// ```
pub fn udev_device_get_sysname(dev: &UdevDevice) -> &str {
    dev.sysname()
}

/// Gets the [UdevDevice] `sysnum`.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Get the instance number of the device.
/// ```
pub fn udev_device_get_sysnum(dev: &UdevDevice) -> &str {
    dev.sysnum()
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
pub fn udev_device_get_devnode(dev: &mut UdevDevice) -> &str {
    dev.get_devnode()
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
pub fn udev_device_get_is_initialized(dev: &mut UdevDevice) -> bool {
    dev.get_is_initialized()
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
pub fn udev_device_get_devlinks_list_entry(dev: &mut UdevDevice) -> Option<&UdevEntry> {
    dev.get_devlinks_list_entry()
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
pub fn udev_device_get_tags_list_entry(dev: &mut UdevDevice) -> Option<&UdevEntry> {
    dev.get_tags_list_entry()
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
pub fn udev_device_get_current_tags_list_entry(dev: &mut UdevDevice) -> Option<&UdevEntry> {
    dev.get_current_tags_list_entry()
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
pub fn udev_device_get_sysattr_list_entry(dev: &mut UdevDevice) -> Option<&UdevEntry> {
    dev.get_sysattr_list_entry()
}

/// Gets the value of a given property.
pub fn udev_device_get_property_value<'d>(dev: &'d UdevDevice, key: &str) -> Option<&'d str> {
    dev.get_property_value(key)
}

/// Gets the kernel driver name.
///
/// Returns: the kernel driver name, or `None`  if none is attached.
pub fn udev_device_get_driver(dev: &mut UdevDevice) -> Option<&str> {
    dev.get_driver()
}

/// Gets the device major/minor number.
pub fn udev_device_get_devnum(dev: &mut UdevDevice) -> u64 {
    dev.get_devnum()
}

/// Gets the device action.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// This is only valid if the device was received through a monitor. Devices read from
/// sys do not have an action string. Usual actions are: add, remove, change, online,
/// offline.
/// ```
///
/// Returns the kernel action value, or `None` if there is no action value available.
pub fn udev_device_get_action(dev: &UdevDevice) -> &str {
    dev.action()
}

/// Gets the device event sequence number.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// This is only valid if the device was received through a monitor. Devices read from
/// sys do not have a sequence number.
/// ```
///
/// Returns the kernel event sequence number, or zero if none is available.
pub const fn udev_device_get_seqnum(dev: &UdevDevice) -> u64 {
    dev.seqnum()
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
pub fn udev_device_get_usec_since_initialized(dev: &mut UdevDevice) -> u64 {
    dev.get_usec_since_initialized()
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
pub fn udev_device_get_sysattr_value(dev: &mut UdevDevice, sysattr: &str) -> Option<String> {
    dev.get_sysattr_value(sysattr)
}

/// Gets whether the [UdevDevice] has the provided `tag` associated.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Check if a given device has a certain tag associated.
/// ```
pub fn udev_device_has_tag(dev: &mut UdevDevice, tag: &str) -> bool {
    dev.has_tag(tag)
}

/// Gets whether the [UdevDevice] has the provided current `tag` associated.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Check if a given device has a certain tag associated.
/// ```
///
/// TODO: `eudev` does not database does not support current tags, implement in this library.
pub fn udev_device_has_current_tag(dev: &mut UdevDevice, tag: &str) -> bool {
    dev.has_tag(tag)
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
/// ```no_build,no_run
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
/// ```
///
/// Returns: a new [UdevMonitor], or [Error], in case of an error
pub fn udev_monitor_new_from_netlink(udev: Arc<Udev>, name: &str) -> Result<Arc<UdevMonitor>> {
    Ok(Arc::new(UdevMonitor::new_from_netlink(udev, name)?))
}

/// Gets the [Udev] context of the [UdevMonitor].
pub fn udev_monitor_get_udev(monitor: &UdevMonitor) -> &Arc<Udev> {
    monitor.udev()
}

/// Binds the [UdevMonitor] socket to the event source.
pub fn udev_monitor_enable_receiving(monitor: &mut UdevMonitor) -> Result<()> {
    monitor.enable_receiving()
}

/// Sets the size of the kernel socket buffer.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Set the size of the kernel socket buffer. This call needs the
/// appropriate privileges to succeed.
/// ```
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_monitor_set_receive_buffer_size(monitor: &mut UdevMonitor, size: usize) -> Result<()> {
    monitor.set_receive_buffer_size(size)
}

/// Gets the [UdevMonitor] socket file descriptor.
pub fn udev_monitor_get_fd(monitor: &UdevMonitor) -> i32 {
    monitor.sock()
}

/// Receives data from the [UdevMonitor] socket.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Receive data from the udev monitor socket, allocate a new udev
/// device, fill in the received data, and return the device.
///
/// Only socket connections with uid=0 are accepted.
///
/// The monitor socket is by default set to NONBLOCK. A variant of poll() on
/// the file descriptor returned by udev_monitor_get_fd() should to be used to
/// wake up when new devices arrive, or alternatively the file descriptor
/// switched into blocking mode.
/// ```
///
/// Returns: `Ok(UdevDevice)` on success, `Err(Error)` otherwise.
pub fn udev_monitor_receive_device(monitor: &mut UdevMonitor) -> Result<UdevDevice> {
    monitor.receive_device()
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
pub fn udev_monitor_filter_add_match_subsystem_devtype<'m>(
    monitor: &'m mut UdevMonitor,
    subsystem: &str,
    devtype: &str,
) -> Result<&'m UdevEntry> {
    monitor.filter_add_match_subsystem_devtype(subsystem, devtype)
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
pub fn udev_monitor_filter_add_match_tag<'m>(
    monitor: &'m mut UdevMonitor,
    tag: &str,
) -> Result<&'m UdevEntry> {
    monitor.filter_add_match_tag(tag)
}

/// Updates the monitor socket filter.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Update the installed socket filter. This is only needed,
/// if the filter was removed or changed.
/// ```
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_monitor_filter_update(monitor: &mut UdevMonitor) -> Result<()> {
    monitor.filter_update()
}

/// Removes all filters from the [UdevMonitor].
///
/// Returns `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_monitor_filter_remove(monitor: &mut UdevMonitor) -> Result<()> {
    monitor.filter_remove()
}

/// Creates a new [UdevEnumerate].
pub fn udev_enumerate_new(udev: Arc<Udev>) -> Arc<UdevEnumerate> {
    Arc::new(UdevEnumerate::new(udev))
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
pub fn udev_enumerate_add_match_subsystem<'e>(
    enumerate: &'e mut UdevEnumerate,
    subsystem: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_match_subsystem(subsystem)
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
pub fn udev_enumerate_add_nomatch_subsystem<'e>(
    enumerate: &'e mut UdevEnumerate,
    subsystem: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_nomatch_subsystem(subsystem)
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
pub fn udev_enumerate_add_match_sysattr<'e>(
    enumerate: &'e mut UdevEnumerate,
    sysattr: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_match_sysattr(sysattr)
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
pub fn udev_enumerate_add_nomatch_sysattr<'e>(
    enumerate: &'e mut UdevEnumerate,
    sysattr: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_nomatch_sysattr(sysattr)
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
pub fn udev_enumerate_add_match_property<'e>(
    enumerate: &'e mut UdevEnumerate,
    property: &str,
    value: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_match_property(property, value)
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
pub fn udev_enumerate_add_match_sysname<'e>(
    enumerate: &'e mut UdevEnumerate,
    sysname: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_match_sysname(sysname)
}

/// Adds an entry to the match tag [UdevEntry] list.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Match only devices with a certain tag.
/// ```
///
/// Returns `Ok(UdevEntry)` on success, `Err(Error)` otherwise.
pub fn udev_enumerate_add_match_tag<'e>(
    enumerate: &'e mut UdevEnumerate,
    tag: &str,
) -> Result<&'e UdevEntry> {
    enumerate.add_match_tag(tag)
}

/// Sets the parent [UdevDevice] on a given device tree.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Return the devices on the subtree of one given device. The parent
/// itself is included in the list.
///
/// A reference for the device is held until the udev_enumerate context
/// is cleaned up.
/// ```
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_enumerate_add_match_parent(
    enumerate: &mut UdevEnumerate,
    dev: Arc<UdevDevice>,
) -> Result<()> {
    enumerate.set_parent(dev);
    Ok(())
}

/// Sets whether the match lists are initialized.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Match only devices which udev has set up already. This makes
/// sure, that the device node permissions and context are properly set
/// and that network devices are fully renamed.
///
/// Usually, devices which are found in the kernel but not already
/// handled by udev, have still pending events. Services should subscribe
/// to monitor events and wait for these devices to become ready, instead
/// of using uninitialized devices.
///
/// For now, this will not affect devices which do not have a device node
/// and are not network interfaces.
/// ```
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_enumerate_add_match_is_initialized(
    enumerate: &mut UdevEnumerate,
    val: bool,
) -> Result<()> {
    enumerate.set_match_is_initialized(val);
    Ok(())
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
pub fn udev_enumerate_add_syspath(enumerate: &mut UdevEnumerate, syspath: &str) -> Result<()> {
    enumerate.add_syspath(syspath)
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
pub fn udev_enumerate_scan_devices(enumerate: &mut UdevEnumerate) -> Result<()> {
    enumerate.scan_devices()
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
pub fn udev_enumerate_scan_subsystems(enumerate: &mut UdevEnumerate) -> Result<()> {
    enumerate.scan_subsystems()
}

/// Creates a new [UdevQueue].
pub fn udev_queue_new(udev: Arc<Udev>) -> Arc<UdevQueue> {
    Arc::new(UdevQueue::new(udev))
}

/// Gets a reference to the [Udev] context.
pub fn udev_queue_get_udev(queue: &UdevQueue) -> &Arc<Udev> {
    queue.udev()
}

/// Checks if [Udev] is active on the system.
pub fn udev_queue_get_udev_is_active(queue: &UdevQueue) -> bool {
    queue.udev_is_active()
}

/// Gets whether [UdevQueue] is currently processing any events.
pub fn udev_queue_get_queue_is_empty(queue: &UdevQueue) -> bool {
    queue.queue_is_empty()
}

/// Gets a file descriptor to watch for a queue to become empty.
pub fn udev_queue_get_fd(queue: &mut UdevQueue) -> Result<i32> {
    queue.get_fd()
}

/// Clears the watched file descriptor for queue changes.
///
/// # Safety
///
/// Users must ensure that every [UdevQueue] has a unique file descriptor, if the descriptor is
/// non-negative.
///
/// Returns: `Ok(())` on success, `Err(Error)` otherwise.
pub fn udev_queue_flush(queue: &mut UdevQueue) -> Result<()> {
    queue.flush()
}

/// Creates a new [UdevHwdb].
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Provides access to the static hardware properties database; the database to
/// use can be overriden by setting the UDEV_HWDB_BIN environment
/// variable to its file name.
/// ```
pub fn udev_hwdb_new(udev: Arc<Udev>) -> Result<Arc<UdevHwdb>> {
    Ok(Arc::new(UdevHwdb::new(udev)?))
}

/// Looks up a matching device in the hardware database.
///
/// Parameters:
///
/// - `modalias`: modalias string
/// - `flags`: (unused), preserved for easier mapping to `libudev` C API
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// The lookup key is a `modalias` string, whose formats are defined for the Linux kernel modules.
/// Examples are: pci:v00008086d00001C2D*, usb:v04F2pB221*. The first entry
/// of a list of retrieved properties is returned.
/// ```
///
/// Returns: an optional reference to an [UdevEntry].
pub fn udev_hwdb_get_properties_list_entry<'h>(
    hwdb: &'h mut UdevHwdb,
    modalias: &str,
    flags: u32,
) -> Option<&'h UdevEntry> {
    hwdb.get_properties_list_entry(modalias, flags)
}

/// Looks up a matching device modalias in the hardware database and returns the list of properties.
pub fn udev_hwdb_query<'h>(hwdb: &'h mut UdevHwdb, modalias: &str) -> Option<&'h UdevList> {
    // populate list if modalias is present and return
    if hwdb.get_properties_list_entry(modalias, 0).is_some() {
        Some(hwdb.properties_list())
    } else {
        None
    }
}

/// Looks up a specific matching property name (key) for device modalias
///
/// ```no_run
/// use std::sync::Arc;
/// use udevrs::{Udev, UdevHwdb};
/// let udev = Arc::new(Udev::new());
///
/// let query = udevrs::udev_hwdb_query_one(&mut UdevHwdb::new(udev).unwrap(), "usb:v1D6Bp0001", "ID_VENDOR_FROM_DATABASE");
/// assert_eq!(query, Some("Linux Foundation".to_string()));
/// ```
pub fn udev_hwdb_query_one(hwdb: &mut UdevHwdb, modalias: &str, name: &str) -> Option<String> {
    udev_hwdb_query(hwdb, modalias).and_then(|list| {
        list.iter()
            .find(|e| e.name() == name)
            .map(|e| e.value().to_owned())
    })
}

/// Encodes provided string, removing potentially unsafe characters.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Encode all potentially unsafe characters of a string to the
/// corresponding 2 char hex value prefixed by '\x'.
/// ```
///
/// Returns: `Ok(String)` on success, `Err(Error)` otherwise
pub fn udev_util_encode_string(arg: &str) -> Result<String> {
    util::encode_string(arg)
}
