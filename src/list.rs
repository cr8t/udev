use std::collections::{
    linked_list::{Iter, IterMut},
    LinkedList,
};
use std::sync::Arc;

use crate::{Udev, UdevDevice};

/// Convenience alias for a [LinkedList] of [UdevEntry].
pub type UdevEntryList = LinkedList<UdevEntry>;

///
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UdevList {
    udev: Arc<Udev>,
    list: UdevEntryList,
    entries_cur: usize,
    entries_max: usize,
    unique: bool,
}

impl UdevList {
    /// Creates a new [UdevList].
    pub const fn new(udev: Arc<Udev>) -> Self {
        Self {
            udev,
            list: LinkedList::new(),
            entries_cur: 0,
            entries_max: 0,
            unique: false,
        }
    }

    /// Creates a new [UdevList] from the provided parameters.
    pub const fn create(udev: Arc<Udev>, list: UdevEntryList) -> Self {
        Self {
            udev,
            list,
            entries_cur: 0,
            entries_max: 0,
            unique: false,
        }
    }

    /// Gets an [`Iterator`] over [UdevEntry] items.
    pub fn iter(&self) -> Iter<UdevEntry> {
        self.list.iter()
    }

    /// Gets an [`Iterator`] over [UdevEntry] items.
    pub fn iter_mut(&mut self) -> IterMut<UdevEntry> {
        self.list.iter_mut()
    }

    /// Gets a reference to the [UdevEntryList].
    pub fn list(&self) -> &UdevEntryList {
        &self.list
    }

    /// Gets a mutable reference to the [UdevEntryList].
    pub fn list_mut(&mut self) -> &mut UdevEntryList {
        &mut self.list
    }

    /// Sets the [UdevEntryList].
    pub fn set_list<L: Into<UdevEntryList>>(&mut self, list: L) {
        self.list = list.into();
    }

    /// Builder function that sets the [UdevEntryList].
    pub fn with_list<L: Into<UdevEntryList>>(mut self, list: L) -> Self {
        self.set_list(list);
        self
    }

    /// Gets the length of the [UdevEntry] list.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Gets whether the [UdevEntryList] is empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Clears the list of all entries.
    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Gets an optional reference to the first [UdevEntry] in the [UdevEntryList].
    pub fn entry(&self) -> Option<&UdevEntry> {
        self.list.front()
    }

    /// Gets an optional mutable reference to the first [UdevEntry] in the [UdevEntryList].
    pub fn entry_mut(&mut self) -> Option<&mut UdevEntry> {
        self.list.front_mut()
    }

    /// Gets an optional reference to an [UdevEntry] with a matching `name`.
    pub fn entry_by_name(&self, name: &str) -> Option<&UdevEntry> {
        self.list.iter().find(|e| e.name() == name)
    }

    /// Gets an optional mutable reference to an [UdevEntry] with a matching `name`.
    pub fn entry_by_name_mut(&mut self, name: &str) -> Option<&mut UdevEntry> {
        self.list.iter_mut().find(|e| e.name() == name)
    }

    /// Gets the next [UdevEntry] in the list.
    pub fn next_entry(&self) -> Option<&UdevEntry> {
        self.list.iter().nth(self.entries_cur)
    }

    /// Gets the next [UdevEntry] in the list.
    pub fn next_entry_mut(&mut self) -> Option<&mut UdevEntry> {
        self.list.iter_mut().nth(self.entries_cur)
    }

    /// Adds an entry to the list.
    ///
    /// If an [UdevEntry] with the same `name` exists, the `value` will be updated.
    ///
    /// If `value` is empty, the entry value with be empty.
    pub fn add_entry(&mut self, name: &str, value: &str) -> Option<&UdevEntry> {
        if self.unique() {
            if self.entry_by_name(name).is_some() {
                self.entry_by_name_mut(name).unwrap().set_value(value);
            } else {
                self.list
                    .push_back(UdevEntry::new().with_name(name).with_value(value));
            }
            self.entry_by_name(name)
        } else {
            self.list
                .push_back(UdevEntry::new().with_name(name).with_value(value));
            self.list.back()
        }
    }

    /// Removes an [UdevEntry] if an entry exists with a matching `name`.
    pub fn remove_entry(&mut self, name: &str) {
        if let Some(pos) = self.list.iter().position(|e| e.name() == name) {
            let mut ext = self.list.split_off(pos);

            if ext.len() > 1 {
                ext.pop_front();
                self.list.append(&mut ext);
            }
        }
    }

    /// Gets the current [UdevEntry].
    pub const fn entries_cur(&self) -> usize {
        self.entries_cur
    }

    /// Gets the maximum number of [UdevEntry] items.
    pub const fn entries_max(&self) -> usize {
        self.entries_max
    }

    /// Gets whether the [UdevList] is unique.
    pub const fn unique(&self) -> bool {
        self.unique
    }

    /// Gets whether the [UdevDevice] matches an [UdevEntry] in the list.
    pub fn has_tag(&self, device: &UdevDevice) -> bool {
        if self.is_empty() {
            true
        } else {
            self.iter()
                .filter(|e| device.tags_list().entry_by_name(e.name()).is_some())
                .count()
                != 0
        }
    }
}

/// UDEV list entry.
///
/// An entry contains contains a name, and optionally a value.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UdevEntry {
    name: String,
    value: String,
    num: i32,
}

impl UdevEntry {
    /// Creates a new [UdevEntry].
    pub const fn new() -> Self {
        Self {
            name: String::new(),
            value: String::new(),
            num: 0,
        }
    }

    /// Gets the [UdevEntry] name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Sets the [UdevEntry] name.
    pub fn set_name<N: Into<String>>(&mut self, name: N) {
        self.name = name.into();
    }

    /// Builder function that sets the [UdevEntry] name.
    pub fn with_name<N: Into<String>>(mut self, name: N) -> Self {
        self.set_name(name);
        self
    }

    /// Gets the [UdevEntry] value.
    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    /// Sets the [UdevEntry] value.
    pub fn set_value<N: Into<String>>(&mut self, value: N) {
        self.value = value.into();
    }

    /// Builder function that sets the [UdevEntry] value.
    pub fn with_value<N: Into<String>>(mut self, value: N) -> Self {
        self.set_value(value);
        self
    }

    /// Gets the [UdevEntry] number.
    pub const fn num(&self) -> i32 {
        self.num
    }

    /// Sets the [UdevEntry] number.
    pub fn set_num(&mut self, num: i32) {
        self.num = num;
    }

    /// Builder function that sets the [UdevEntry] number.
    pub fn with_num(mut self, num: i32) -> Self {
        self.set_num(num);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udev_entry() {
        let mut null_entry = UdevEntry::new();

        let exp_name = "test_name";
        let exp_value = "test_value";
        let exp_num = 42;

        let exp_entry = UdevEntry::new()
            .with_name(exp_name)
            .with_value(exp_value)
            .with_num(exp_num);

        assert_eq!(null_entry.name(), "");
        assert_eq!(null_entry.value(), "");
        assert_eq!(null_entry.num(), 0);

        assert_eq!(exp_entry.name(), exp_name);
        assert_eq!(exp_entry.value(), exp_value);
        assert_eq!(exp_entry.num(), exp_num);

        null_entry.set_name(exp_name);
        assert_eq!(null_entry.name(), exp_name);

        null_entry.set_value(exp_value);
        assert_eq!(null_entry.value(), exp_value);

        null_entry.set_num(exp_num);
        assert_eq!(null_entry.num(), exp_num);

        assert_eq!(null_entry, exp_entry);
    }
}
