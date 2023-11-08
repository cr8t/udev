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
pub fn udev_list_entry_foreach_mut(list: &mut UdevList, f: fn(&mut UdevEntry) -> Result<()>) -> Result<()> {
    for entry in list.iter_mut() {
        f(entry)?;
    }
    Ok(())
}
