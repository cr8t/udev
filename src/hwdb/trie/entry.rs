use std::mem;

use crate::{hwdb, Error, Result};

use super::{TrieChildEntry, TrieNode, TrieValueEntry};

/// Represents the full Trie entry in the HWDB.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct TrieEntry {
    node: TrieNode,
    children: Vec<TrieChildEntry>,
    values: Vec<TrieValueEntry>,
}

impl TrieEntry {
    /// Creates a new [TrieEntry].
    pub const fn new() -> Self {
        Self {
            node: TrieNode::new(),
            children: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Gets a reference to the [TrieNode].
    pub const fn node(&self) -> &TrieNode {
        &self.node
    }

    /// Gets a reference to the list of [TrieChildEntry].
    pub fn children(&self) -> &[TrieChildEntry] {
        self.children.as_ref()
    }

    /// Gets a reference to the list of [TrieValueEntry].
    pub fn values(&self) -> &[TrieValueEntry] {
        self.values.as_ref()
    }

    /// Gets the total length of the [TrieEntry].
    pub fn len(&self) -> usize {
        let children_len = self.children.len().saturating_mul(hwdb::child_entry_size());
        let values_len = self.values.len().saturating_mul(hwdb::value_entry_size());

        mem::size_of::<TrieNode>()
            .saturating_add(children_len)
            .saturating_add(values_len)
    }

    /// Gets whether the [TrieValueEntry] has no children and values.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty() && self.values.is_empty()
    }

    /// Looks up a child node in the HWDB buffer.
    ///
    /// Parameters:
    ///
    /// - `hwdb_buf`: in-memory buffer of the entire HWDB.
    /// - `c`: Child index to search the list of [TrieChildEntry].
    ///
    /// Returns [Some(TrieNode)](TrieNode) on success, [`None`] otherwise.
    pub fn lookup_child(&self, hwdb_buf: &[u8], c: u8) -> Option<Self> {
        let search = TrieChildEntry::new().with_c(c);
        let buf_len = hwdb_buf.len();

        // assuming children are sorted (done in initialisation), perform a binary search instead like C hwdb
        let child = self.children.binary_search_by(|child| child.cmp(&search))
            .ok()
            .and_then(|idx| self.children.get(idx))?;

        let child_off = child.child_off() as usize;

        // if the child offset is in range, attempt to construct a `TrieNode` at that offset
        if (0..buf_len).contains(&child_off) {
            Self::try_from(&hwdb_buf[child_off..]).ok()
        } else {
            None
        }
    }
}

impl Default for TrieEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<&[u8]> for TrieEntry {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        let node = TrieNode::try_from(val)?;

        let mut idx = hwdb::node_size();

        let val_end = val.len();
        let child_len = hwdb::child_entry_size();
        let child_count = node.children_count() as usize;
        let child_end = idx.saturating_add(child_count.saturating_mul(child_len));

        let mut children: Vec<TrieChildEntry> = Vec::with_capacity(child_count);

        if (idx..val_end).contains(&child_end) && child_count > 0 {
            for c in val[idx..].chunks_exact(child_len).take(child_count) {
                children.push(c.try_into()?);
                idx = idx.saturating_add(child_len);
            }
        }

        children.sort();

        let value_len = hwdb::value_entry_size();
        let value_count = node.values_count() as usize;
        let value_end = idx.saturating_add(value_count.saturating_mul(value_len));

        let mut values: Vec<TrieValueEntry> = Vec::with_capacity(value_count);

        if (idx..val_end).contains(&value_end) && value_count > 0 {
            for c in val[idx..].chunks_exact(value_len).take(value_count) {
                values.push(c.try_into()?);
            }
        }

        Ok(Self {
            node,
            children,
            values,
        })
    }
}
