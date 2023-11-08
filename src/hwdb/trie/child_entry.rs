use std::{cmp, mem};

use crate::{Error, Result};

/// Trie child entry in the hardware database.
///
/// Array of child entries that directly follows the node record.
#[repr(C, packed(8))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TrieChildEntry {
    c: u8,
    _padding: [u8; 7],
    child_off: u64,
}

impl TrieChildEntry {
    /// Creates a new [TrieChildEntry].
    pub const fn new() -> Self {
        Self {
            c: 0,
            _padding: [0u8; 7],
            child_off: 0,
        }
    }

    /// Gets the index of the child node.
    pub const fn c(&self) -> u8 {
        self.c
    }

    /// Sets the index of the child node.
    pub fn set_c(&mut self, val: u8) {
        self.c = val;
    }

    /// Builder function that sets the index of the child node.
    pub fn with_c(mut self, val: u8) -> Self {
        self.set_c(val);
        self
    }

    /// Gets the offset of the child node.
    pub const fn child_off(&self) -> u64 {
        self.child_off
    }

    /// Sets the offset of the child node.
    pub fn set_child_off(&mut self, val: u64) {
        self.child_off = val;
    }

    /// Builder function that sets the offset of the child node.
    pub fn with_child_off(mut self, val: u64) -> Self {
        self.set_child_off(val);
        self
    }
}

impl TryFrom<&[u8]> for TrieChildEntry {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        if val.len() < mem::size_of::<Self>() {
            Err(Error::InvalidLen(val.len()))
        } else {
            let mut idx = 0usize;

            let c = val[idx];
            let _padding = [0u8; 7];

            // skip `c` index + padding
            idx += 8;

            let child_off = u64::from_le_bytes(val[idx..idx + 8].try_into()?);

            Ok(Self {
                c,
                _padding,
                child_off,
            })
        }
    }
}

impl Ord for TrieChildEntry {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.c.cmp(&other.c)
    }
}

impl PartialOrd for TrieChildEntry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.c.cmp(&other.c))
    }
}
