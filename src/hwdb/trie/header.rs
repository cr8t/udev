use std::mem;

use crate::{Error, Result};

use super::{HWDB_SIG, HWDB_SIG_STR};

/// On-disk trie objects
#[repr(C, packed(8))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrieHeader {
    signature: [u8; 8],
    tool_version: u64,
    file_size: u64,
    header_size: u64,
    node_size: u64,
    child_entry_size: u64,
    value_entry_size: u64,
    nodes_root_off: u64,
    nodes_len: u64,
    strings_len: u64,
}

impl TrieHeader {
    /// Creates a new [TrieHeader].
    pub const fn new() -> Self {
        Self {
            signature: HWDB_SIG,
            tool_version: 0,
            file_size: 0,
            header_size: 0,
            node_size: 0,
            child_entry_size: 0,
            value_entry_size: 0,
            nodes_root_off: 0,
            nodes_len: 0,
            strings_len: 0,
        }
    }

    /// Gets the [TrieHeader] object signature.
    pub fn signature(&self) -> &str {
        std::str::from_utf8(self.signature.as_ref()).unwrap_or("")
    }

    /// Version of the tool which created the file.
    pub const fn tool_version(&self) -> u64 {
        self.tool_version
    }

    /// Sets the version of the tool which created the file.
    pub fn set_tool_version(&mut self, val: u64) {
        self.tool_version = val;
    }

    /// Builder function that sets the version of the tool which created the file.
    pub fn with_tool_version(mut self, val: u64) -> Self {
        self.set_tool_version(val);
        self
    }

    /// Gets the file size of the database file.
    pub const fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Sets the file size of the database file.
    pub fn set_file_size(&mut self, val: u64) {
        self.file_size = val;
    }

    /// Builder function that sets the file size of the database file.
    pub fn with_file_size(mut self, val: u64) -> Self {
        self.set_file_size(val);
        self
    }

    /// Gets the header size of the [TrieHeader].
    pub const fn header_size(&self) -> u64 {
        self.header_size
    }

    /// Sets the header size of the [TrieHeader].
    pub fn set_header_size(&mut self, val: u64) {
        self.header_size = val;
    }

    /// Builder function that sets the header size of the [TrieHeader].
    pub fn with_header_size(mut self, val: u64) -> Self {
        self.set_header_size(val);
        self
    }

    /// Gets the node size of the database file.
    pub const fn node_size(&self) -> u64 {
        self.node_size
    }

    /// Sets the node size of the database file.
    pub fn set_node_size(&mut self, val: u64) {
        self.node_size = val;
    }

    /// Builder function that sets the node size of the database file.
    pub fn with_node_size(mut self, val: u64) -> Self {
        self.set_node_size(val);
        self
    }

    /// Gets the child entry size of the database file.
    pub const fn child_entry_size(&self) -> u64 {
        self.child_entry_size
    }

    /// Sets the child entry size of the database file.
    pub fn set_child_entry_size(&mut self, val: u64) {
        self.child_entry_size = val;
    }

    /// Builder function that sets child entry size of the database file.
    pub fn with_child_entry_size(mut self, val: u64) -> Self {
        self.set_child_entry_size(val);
        self
    }

    /// Gets the value entry size of the database file.
    pub const fn value_entry_size(&self) -> u64 {
        self.value_entry_size
    }

    /// Sets the value entry size of the database file.
    pub fn set_value_entry_size(&mut self, val: u64) {
        self.value_entry_size = val;
    }

    /// Builder function that sets value entry size of the database file.
    pub fn with_value_entry_size(mut self, val: u64) -> Self {
        self.set_value_entry_size(val);
        self
    }

    /// Gets the offset of the root trie node.
    pub const fn nodes_root_off(&self) -> u64 {
        self.nodes_root_off
    }

    /// Sets the offset of the root trie node.
    pub fn set_nodes_root_off(&mut self, val: u64) {
        self.nodes_root_off = val;
    }

    /// Builder function that sets the offset of the root trie node.
    pub fn with_nodes_root_off(mut self, val: u64) -> Self {
        self.set_nodes_root_off(val);
        self
    }

    /// Gets the size of the nodes section.
    pub const fn nodes_len(&self) -> u64 {
        self.nodes_len
    }

    /// Sets the size of the nodes section.
    pub fn set_nodes_len(&mut self, val: u64) {
        self.nodes_len = val;
    }

    /// Builder function that sets the size of the nodes section.
    pub fn with_nodes_len(mut self, val: u64) -> Self {
        self.set_nodes_len(val);
        self
    }

    /// Gets the size of the strings section.
    pub const fn strings_len(&self) -> u64 {
        self.strings_len
    }

    /// Sets the size of the strings section.
    pub fn set_strings_len(&mut self, val: u64) {
        self.strings_len = val;
    }

    /// Builder function that sets the size of the strings section.
    pub fn with_strings_len(mut self, val: u64) -> Self {
        self.set_strings_len(val);
        self
    }
}

impl TryFrom<&[u8]> for TrieHeader {
    type Error = Error;

    fn try_from(val: &[u8]) -> Result<Self> {
        if val.len() < mem::size_of::<Self>() {
            Err(Error::InvalidLen(val.len()))
        } else {
            let mut idx = 0usize;

            let signature: [u8; 8] = val[idx..idx + 8].try_into()?;

            if signature != HWDB_SIG {
                let sig_str = std::str::from_utf8(signature.as_ref()).unwrap_or("");
                Err(Error::UdevHwdb(format!(
                    "invalid HWDB signature, have: {sig_str}, expected: {HWDB_SIG_STR}"
                )))
            } else {
                idx += signature.len();

                let tool_version = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let file_size = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let header_size = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let node_size = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let child_entry_size = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let value_entry_size = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let nodes_root_off = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let nodes_len = u64::from_le_bytes(val[idx..idx + 8].try_into()?);
                idx += mem::size_of::<u64>();

                let strings_len = u64::from_le_bytes(val[idx..idx + 8].try_into()?);

                Ok(Self {
                    signature,
                    tool_version,
                    file_size,
                    header_size,
                    node_size,
                    child_entry_size,
                    value_entry_size,
                    nodes_root_off,
                    nodes_len,
                    strings_len,
                })
            }
        }
    }
}
