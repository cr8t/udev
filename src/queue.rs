//! Access to currently active events.
//!
//! The udev daemon processes events asynchronously. All events which do not have
//! interdependencies run in parallel. This exports the current state of the
//! event processing queue, and the current event sequence numbers from the kernel
//! and the udev daemon.
//!
//! From `libudev-queue` documentation.

use std::sync::Arc;

use crate::{Udev, UdevEntryList, UdevList};

/// Represents the current event queue in the udev daemon.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UdevQueue {
    udev: Arc<Udev>,
    queue_list: UdevList,
}

impl UdevQueue {
    /// Creates a new [UdevQueue].
    pub fn new(udev: Arc<Udev>) -> Self {
        let udev_arc = Arc::clone(&udev);
        Self {
            udev,
            queue_list: UdevList::new(udev_arc),
        }
    }

    /// Creates a new [UdevQueue] from the provided parameter.
    pub fn create<Q: Into<UdevEntryList>>(udev: Arc<Udev>, queue_list: Q) -> Self {
        let udev_arc = Arc::clone(&udev);
        Self {
            udev,
            queue_list: UdevList::create(udev_arc, queue_list.into()),
        }
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
        let exp_queue = UdevQueue::create(Arc::clone(&udev), exp_list.clone());

        assert!(null_queue.queue_list().is_empty());

        for (entry, exp_entry) in exp_queue.queue_list().iter().zip(exp_list.iter()) {
            assert_eq!(entry, exp_entry);
        }

        null_queue.set_queue_list(exp_list);

        assert_eq!(null_queue, exp_queue);
    }
}
