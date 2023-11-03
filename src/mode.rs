/// Bitmask for [Mode].
pub const MODE_MASK: u32 = 0b1111_1111_1111;

/// Linux file-permission mode.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mode(u32);
bitflags! {
    impl Mode: u32 {
        const SET_UID = 1 << 11;
        const SET_GID = 1 << 10;
        const SAVE_TXT = 1 << 9;
        const READ_USER = 1 << 8;
        const WRITE_USER = 1 << 7;
        const EXEC_USER = 1 << 6;
        const READ_GROUP = 1 << 5;
        const WRITE_GROUP = 1 << 4;
        const EXEC_GROUP = 1 << 3;
        const READ_OTHER = 1 << 2;
        const WRITE_OTHER = 1 << 1;
        const EXEC_OTHER = 1 << 0;
        const NONE = 0;
    }
}

impl Mode {
    /// Creates a new [Mode].
    pub const fn new() -> Self {
        Self(0)
    }

    /// Creates a new [Mode] from the provided parameter.
    pub const fn create(val: u32) -> Self {
        Self(val & MODE_MASK)
    }
}

impl From<u32> for Mode {
    fn from(val: u32) -> Self {
        Self::create(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode() {
        assert_eq!(Mode::new(), Mode::default());
    }
}
