use std::mem;

use crate::{hwdb, Error, Result};

/// Trie node in the hardware database.
#[repr(C, packed(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrieNode {
    prefix_off: u64,
    children_count: u8,
    _padding: [u8; 7],
    values_count: u64,
}

impl TrieNode {
    /// Creates a new [TrieNode].
    pub const fn new() -> Self {
        Self {
            prefix_off: 0,
            children_count: 0,
            _padding: [0u8; 7],
            values_count: 0,
        }
    }

    /// Gets the length of the encoded [TrieNode].
    pub fn len(&self) -> usize {
        hwdb::node_size()
    }

    /// Gets whether the [TrieNode] is empty.
    pub const fn is_empty(&self) -> bool {
        self.children_count == 0 && self.values_count == 0
    }

    /// Gets the prefix of the lookup string, shared by all children.
    pub const fn prefix_off(&self) -> u64 {
        self.prefix_off
    }

    /// Sets the prefix of the lookup string, shared by all children.
    pub fn set_prefix_off(&mut self, val: u64) {
        self.prefix_off = val;
    }

    /// Builder function that sets the prefix of the lookup string, shared by all children.
    pub fn with_prefix_off(mut self, val: u64) -> Self {
        self.set_prefix_off(val);
        self
    }

    /// Gets the size of children entry array appended to the node.
    pub const fn children_count(&self) -> u8 {
        self.children_count
    }

    /// Sets the size of children entry array appended to the node.
    pub fn set_children_count(&mut self, val: u8) {
        self.children_count = val;
    }

    /// Builder function that sets the size of children entry array appended to the node.
    pub fn with_children_count(mut self, val: u8) -> Self {
        self.set_children_count(val);
        self
    }

    /// Gets the size of value entry array appended to the node.
    pub const fn values_count(&self) -> u64 {
        self.values_count
    }

    /// Sets the size of value entry array appended to the node.
    pub fn set_values_count(&mut self, val: u64) {
        self.values_count = val;
    }

    /// Builder function that sets the size of value entry array appended to the node.
    pub fn with_values_count(mut self, val: u64) -> Self {
        self.set_values_count(val);
        self
    }
}

impl TryFrom<&[u8]> for TrieNode {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        if val.len() < mem::size_of::<Self>() {
            Err(Error::InvalidLen(val.len()))
        } else {
            let mut idx = 0usize;

            let prefix_off = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
            idx += mem::size_of::<u64>();

            let children_count = val[idx];
            let _padding = [0u8; 7];

            // skip past the children count + padding
            idx += 8;

            let values_count = u64::from_le_bytes(
                val.get(idx..idx + 8)
                    .ok_or(Error::InvalidLen(val.len()))?
                    .try_into()?,
            );

            if values_count > 64 {
                Err(Error::InvalidLen(values_count as usize))
            } else {
                Ok(Self {
                    prefix_off,
                    children_count,
                    _padding,
                    values_count,
                })
            }
        }
    }
}
