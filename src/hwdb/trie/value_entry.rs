use std::mem;

use crate::{hwdb, Error, Result};

/// Trie value entry in the hardware database.
///
/// Array of value entries that directly follows the node record.
#[repr(C, packed(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrieValueEntry {
    key_off: u64,
    value_off: u64,
}

impl TrieValueEntry {
    /// Creates a new [TrieValueEntry].
    pub const fn new() -> Self {
        Self {
            key_off: 0,
            value_off: 0,
        }
    }

    /// Gets the length of the encoded [TrieValueEntry].
    pub fn len(&self) -> usize {
        hwdb::value_entry_size()
    }

    /// Gets whether the [TrieValueEntry] is empty.
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Gets key offset.
    pub const fn key_off(&self) -> u64 {
        self.key_off
    }

    /// Sets key offset.
    pub fn set_key_off(&mut self, val: u64) {
        self.key_off = val;
    }

    /// Builder function that sets the key offset.
    pub fn with_key_off(mut self, val: u64) -> Self {
        self.set_key_off(val);
        self
    }

    /// Gets value offset.
    pub const fn value_off(&self) -> u64 {
        self.value_off
    }

    /// Sets value offset.
    pub fn set_value_off(&mut self, val: u64) {
        self.value_off = val;
    }

    /// Builder function that sets the value offset.
    pub fn with_value_off(mut self, val: u64) -> Self {
        self.set_value_off(val);
        self
    }
}

impl TryFrom<&[u8]> for TrieValueEntry {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        if val.len() < hwdb::value_entry_size() {
            Err(Error::InvalidLen(val.len()))
        } else {
            // TODO: parse use get ranges and offsets
            let mut idx = 0usize;

            let key_off = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
            idx += mem::size_of::<u64>();

            let value_off = u64::from_le_bytes(val[idx..idx + 8].try_into()?);

            Ok(Self { key_off, value_off })
        }
    }
}
