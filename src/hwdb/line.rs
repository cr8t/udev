use super::trie_string;
use crate::{Error, Result, TrieEntry, UdevHwdb, UdevList};
use heapless::Vec;

/// Maximum length for a file line.
pub const LINE_MAX: usize = 4096;

/// Line buffer for parsing HWDB on-disk file format.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineBuf {
    bytes: Vec<u8, LINE_MAX>,
}

impl LineBuf {
    /// Creates a new [LineBuf].
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Gets the line buffer as a string reference.
    pub fn get(&self) -> &str {
        std::str::from_utf8(self.bytes.as_ref()).unwrap_or("")
    }

    /// Adds `val` string to the [LineBuf].
    ///
    /// `val` must not cause the total length of the [LineBuf] to exceed [LINE_MAX].
    pub fn add(&mut self, val: &str) -> Result<()> {
        self.bytes.extend_from_slice(val.as_bytes()).map_err(|_| {
            Error::UdevHwdb(format!(
                "byte string exceeds the max line length: {LINE_MAX}"
            ))
        })
    }

    /// Adds `val` character to the [LineBuf].
    ///
    /// `val` must not cause the total length of the [LineBuf] to exceed [LINE_MAX].
    pub fn add_char<C: Into<u8>>(&mut self, val: C) -> Result<()> {
        self.bytes.push(val.into()).map_err(|_| {
            Error::UdevHwdb(format!("character exceeds the max line length: {LINE_MAX}"))
        })
    }

    /// Removes `count` characters from the [LineBuf].
    ///
    /// **NOTE**: clears the buffer if count is larger than the [LineBuf] length.
    pub fn remove(&mut self, count: usize) {
        if count >= self.bytes.len() {
            self.bytes.clear();
        } else {
            self.bytes.truncate(self.bytes.len() - count);
        }
    }

    /// Removes a single character from the [LineBuf].
    pub fn remove_char(&mut self) {
        self.remove(1);
    }

    /// Searches the [LineBuf] for a matching property.
    ///
    /// If a property is found, it is added to [`list`](UdevList).
    pub fn trie_fnmatch(
        &mut self,
        list: &mut UdevList,
        hwdb_buf: &[u8],
        entry: &TrieEntry,
        p: usize,
        search: &str,
    ) -> Result<()> {
        let prefix_off = entry.node().prefix_off() as usize;
        let prefix = trie_string(hwdb_buf, prefix_off);
        let prefix_len = prefix.len();

        log::trace!(
            "Entering fnmatch, prefix: {prefix}, glob: {}, search: {search}",
            self.get()
        );

        let (start, end) = (
            p,
            prefix
                .as_bytes()
                .iter()
                .skip(p)
                .position(|c| *c == b'\0')
                .unwrap_or(prefix_len),
        );

        // the logic of add only if within bounds but always remove is odd but the linebuf_add return is not checked in https://github.com/cr8t/udev/issues/25#L187
        // so behavior should match
        if (start..=end).contains(&prefix_len) {
            self.add(&prefix[start..end])?;
        }

        for child in entry.children().iter() {
            let child_off = child.child_off() as usize;
            if child_off < hwdb_buf.len() {
                self.add_char(child.c())?;
                self.trie_fnmatch(
                    list,
                    hwdb_buf,
                    &TrieEntry::try_from(&hwdb_buf[child_off..])?,
                    0,
                    search,
                )?;
                self.remove_char();
            }
        }

        if glob::Pattern::new(self.get())?.matches(search) {
            log::trace!("Found matching entry, entry: {entry:?}, search: {search}");
            for value in entry.values().iter() {
                UdevHwdb::_add_property(
                    list,
                    trie_string(hwdb_buf, value.key_off() as usize),
                    trie_string(hwdb_buf, value.value_off() as usize),
                )?;
            }
        }

        self.remove(end.saturating_sub(start));

        Ok(())
    }
}
