use std::fs;

use crate::{murmur_hash2, Error, Result, Udev};

pub(crate) const LINE_SIZE: usize = 16384;

mod device_nodes;

pub use device_nodes::*;

impl Udev {
    pub(crate) fn get_sys_core_link_value(slink: &str, syspath: &str) -> Result<String> {
        let path = format!("{syspath}/{slink}");
        let link = fs::read_link(path)?;

        // get the basename of the symlinked target
        Ok(link
            .components()
            .next_back()
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

/// Gets the major part of the device number.
pub fn major(dev: libc::dev_t) -> u16 {
    (((dev >> 31 >> 1) & 0xfffff000) | ((dev >> 8) & 0x00000fff)) as u16
}

/// Gets the minor part of the device number.
pub fn minor(dev: libc::dev_t) -> u16 {
    (((dev >> 12) & 0xffffff00) | (dev & 0x000000ff)) as u16
}

/// Encodes provided string, removing potentially unsafe characters.
///
/// From the `libudev` documentation:
///
/// ```no_build,no_run
/// Encode all potentially unsafe characters of a string to the
/// corresponding 2 char hex value prefixed by '\x'.
/// ```
///
/// Returns: `Ok(String)` on success, `Err(Error)` otherwise
pub fn encode_string(arg: &str) -> Result<String> {
    encode_devnode_name(arg)
}
