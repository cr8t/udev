use std::fs;

use crate::{murmur_hash2, Error, Result, Udev};

impl Udev {
    pub(crate) fn get_sys_core_link_value(slink: &str, syspath: &str) -> Result<String> {
        let path = format!("{syspath}/{slink}");
        let link = fs::read_link(path)?;

        // get the basename of the symlinked target
        Ok(link
            .components()
            .last()
            .ok_or(Error::Io("empty sys core link value".into()))?
            .as_os_str()
            .to_str()
            .ok_or(Error::Io(
                "sys core link OS string contains non-Unicode bytes".into(),
            ))?
            .to_owned())
    }
}

/// Compute a MurMurHash over the provided string.
pub fn string_hash32(s: &str) -> u32 {
    murmur_hash2(s.as_bytes(), 0)
}

/// Gets a bunch of bit numbers out of the hash, and sets the bits into a bitfield.
pub fn string_bloom64(s: &str) -> u64 {
    let hash = string_hash32(s);

    (1u64 << (hash & 63))
        | (1u64 << ((hash >> 6) & 63))
        | (1u64 << ((hash >> 12) & 63))
        | (1u64 << ((hash >> 18) & 63))
}
