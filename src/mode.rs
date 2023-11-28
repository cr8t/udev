use std::fmt;

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

    /// Gets whether bits in `oth` are set in `self`.
    pub fn is_set(&self, oth: &Self) -> bool {
        self.0 & oth.0 != 0
    }
}

impl From<u32> for Mode {
    fn from(val: u32) -> Self {
        Self::create(val)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;

        let (suid, sgid, svtxt) = (
            self.is_set(&Self::SET_UID),
            self.is_set(&Self::SET_GID),
            self.is_set(&Self::SAVE_TXT),
        );
        write!(f, r#""set_bits": "SUID{suid} SGID{sgid} SAVE_TXT{svtxt}","#)?;

        let (ur, uw, ux) = (
            self.is_set(&Self::READ_USER),
            self.is_set(&Self::WRITE_USER),
            self.is_set(&Self::EXEC_USER),
        );
        write!(f, r#""user": "R{ur}W{uw}X{ux}","#)?;

        let (gr, gw, gx) = (
            self.is_set(&Self::READ_GROUP),
            self.is_set(&Self::WRITE_GROUP),
            self.is_set(&Self::EXEC_GROUP),
        );
        write!(f, r#""group: "R{gr}W{gw}X{gx}","#)?;

        let (or, ow, ox) = (
            self.is_set(&Self::READ_OTHER),
            self.is_set(&Self::WRITE_OTHER),
            self.is_set(&Self::EXEC_OTHER),
        );
        write!(f, r#""other: "R{or}W{ow}X{ox}""#)?;

        write!(f, "}}")
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
