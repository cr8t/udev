//! Access to currently active events.
//!
//! The udev daemon processes events asynchronously. All events which do not have
//! interdependencies run in parallel. This exports the current state of the
//! event processing queue, and the current event sequence numbers from the kernel
//! and the udev daemon.
//!
//! From `libudev-queue` documentation.

use std::io::{self, Write};
use std::os::fd::FromRawFd;
use std::{ffi, fs, sync::Arc};

use crate::UDEV_ROOT_RUN;
use crate::{Error, Result, Udev, UdevEntryList, UdevList};

/// Represents the current event queue in the udev daemon.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UdevQueue {
    udev: Arc<Udev>,
    queue_list: UdevList,
    fd: i32,
}

impl UdevQueue {
    /// Creates a new [UdevQueue].
    pub fn new(udev: Arc<Udev>) -> Self {
        let udev_arc = Arc::clone(&udev);
        Self {
            udev,
            queue_list: UdevList::new(udev_arc),
            fd: -1,
        }
    }

    /// Creates a new [UdevQueue] from the provided parameters.
    pub fn create<Q: Into<UdevEntryList>>(udev: Arc<Udev>, queue_list: Q, fd: i32) -> Self {
        let udev_arc = Arc::clone(&udev);
        Self {
            udev,
            queue_list: UdevList::create(udev_arc, queue_list.into()),
            fd,
        }
    }

    /// Gets a reference to the [Udev] context.
    pub const fn udev(&self) -> &Arc<Udev> {
        &self.udev
    }

    /// Gets a reference to the queue list [UdevList].
    pub const fn queue_list(&self) -> &UdevList {
        &self.queue_list
    }

    /// Gets a mutable reference to the queue list [UdevList].
    pub fn queue_list_mut(&mut self) -> &mut UdevList {
        &mut self.queue_list
    }

    /// Sets the queue list [UdevEntryList].
    pub fn set_queue_list<Q: Into<UdevEntryList>>(&mut self, queue_list: Q) {
        self.queue_list.set_list(queue_list);
    }

    /// Builder function that sets the queue list [UdevEntryList].
    pub fn with_queue_list<Q: Into<UdevEntryList>>(mut self, queue_list: Q) -> Self {
        self.set_queue_list(queue_list);
        self
    }

    /// Gets the length of the [UdevQueue].
    pub fn len(&self) -> usize {
        self.queue_list.len()
    }

    /// Gets whether the [UdevQueue] is empty.
    pub fn is_empty(&self) -> bool {
        self.queue_list.is_empty()
    }

    /// Gets the [UdevQueue] file descriptor.
    pub const fn fd(&self) -> i32 {
        self.fd
    }

    /// Sets the [UdevQueue] file descriptor.
    pub fn set_fd(&mut self, val: i32) {
        self.fd = val;
    }

    /// Builder function that sets the [UdevQueue] file descriptor.
    pub fn with_fd(mut self, val: i32) -> Self {
        self.set_fd(val);
        self
    }

    /// Gets a file descriptor to watch for a queue to become empty.
    pub fn get_fd(&mut self) -> Result<i32> {
        if self.fd < 0 {
            // SAFETY: the argument is valid, and the return value is checked before use.
            let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
            if fd < 0 {
                let errno = io::Error::last_os_error();
                let err_msg =
                    format!("unable to init inotify monitor, error: {fd}, errno: {errno}");
                log::error!("{err_msg}");
                Err(Error::UdevQueue(err_msg))
            } else {
                let udev_path = ffi::CString::new(format!("{UDEV_ROOT_RUN}/udev"))?;
                // SAFETY: arguments are valid, and pointers reference valid memory.
                let r = unsafe {
                    libc::inotify_add_watch(fd, udev_path.as_ptr() as *const _, libc::IN_DELETE)
                };
                if r < 0 {
                    let errno = io::Error::last_os_error();
                    let err_msg =
                        format!("unable to add inotify watch event, error: {r}, errno: {errno}");
                    log::error!("{err_msg}");
                    // SAFETY: argument is valid
                    unsafe { libc::close(fd) };
                    Err(Error::UdevQueue(err_msg))
                } else {
                    self.fd = fd;
                    Ok(fd)
                }
            }
        } else {
            Ok(self.fd)
        }
    }

    /// Clears the watched file descriptor for queue changes.
    ///
    /// # Safety
    ///
    /// Users must ensure that every [UdevQueue] has a unique file descriptor, if the descriptor is
    /// non-negative.
    ///
    /// Returns: `Ok(())` on success, `Err(Error)` otherwise.
    pub fn flush(&mut self) -> Result<()> {
        let fd = self.fd;
        if fd < 0 {
            let err = libc::EINVAL;
            Err(Error::UdevQueue(format!(
                "invalid file descriptor, fd: {fd}, error: {err}"
            )))
        } else {
            // SAFETY: argument is valid, and only one valid mutable reference to this UdevQueue
            // can be held without further `unsafe` code.
            //
            // Users must ensure that every UdevQueue has a unique file descriptor.
            let mut file = unsafe { fs::File::from_raw_fd(fd) };
            file.flush().map_err(|err| {
                let errno = io::Error::last_os_error();
                let err_msg =
                    format!("unable to flush queue file descriptor, error: {err}, errno: {errno}");
                log::error!("{err_msg}");
                Error::UdevQueue(err_msg)
            })
        }
    }

    /// Checks if [Udev] is active on the system.
    pub fn udev_is_active(&self) -> bool {
        fs::OpenOptions::new()
            .read(true)
            .open(format!("{UDEV_ROOT_RUN}/udev/control"))
            .is_ok()
    }

    /// Gets whether [UdevQueue] is currently processing any events.
    pub fn queue_is_empty(&self) -> bool {
        fs::OpenOptions::new()
            .read(true)
            .open(format!("{UDEV_ROOT_RUN}/udev/queue"))
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UdevEntry;

    #[test]
    fn test_udev_queue() {
        let udev = Arc::new(Udev::new());
        let mut null_queue = UdevQueue::new(Arc::clone(&udev));

        let exp_list = [UdevEntry::new().with_name("test_list_entry")];
        let exp_queue = UdevQueue::create(Arc::clone(&udev), exp_list.clone(), -1);

        assert!(null_queue.queue_list().is_empty());

        for (entry, exp_entry) in exp_queue.queue_list().iter().zip(exp_list.iter()) {
            assert_eq!(entry, exp_entry);
        }

        null_queue.set_queue_list(exp_list);

        assert_eq!(null_queue, exp_queue);
    }
}
