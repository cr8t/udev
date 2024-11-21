mod child_entry;
mod entry;
mod header;
mod node;
mod value_entry;

use crate::{Error, Result};
pub use child_entry::*;
pub use entry::*;
pub use header::*;
pub use node::*;
pub use value_entry::*;

/// Hardware database signature.
pub const HWDB_SIG: [u8; 8] = [b'K', b'S', b'L', b'P', b'H', b'H', b'R', b'H'];
/// Hardware database signature (string representation).
pub const HWDB_SIG_STR: &str = "KSLPHHRH";

/// Parses a string from the HWDB buffer.
pub fn trie_string(hwdb_buf: &[u8], offset: usize) -> Result<&str> {
    let buf_len = hwdb_buf.len();
    if (0..buf_len).contains(&offset) {
        let str_end = hwdb_buf[offset..]
            .iter()
            .position(|c| c == &b'\0' || c == &b'\n')
            .map(|end| offset + end)
            .unwrap_or(buf_len);

        std::str::from_utf8(&hwdb_buf[offset..str_end])
            .map_err(|_| Error::UdevHwdb("failed to parse utf-8 trie_string".to_string()))
    } else {
        Err(Error::UdevHwdb("invalid trie_string offset".to_string()))
    }
}
