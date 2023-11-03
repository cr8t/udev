use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    InvalidLen(usize),
    Udev(String),
    UdevDevice(String),
    UdevHwdb(String),
    UdevMonitor(String),
    Io(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(format!("{err}"))
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(err: std::array::TryFromSliceError) -> Self {
        Self::Io(format!("{err}"))
    }
}

impl From<glob::PatternError> for Error {
    fn from(err: glob::PatternError) -> Self {
        Self::Io(format!("invalid glob pattern: {err}"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLen(err) => write!(f, "udev invalid length: {err}"),
            Self::Udev(err) => write!(f, "udev: {err}"),
            Self::UdevDevice(err) => write!(f, "udev device: {err}"),
            Self::UdevHwdb(err) => write!(f, "udev hwdb: {err}"),
            Self::UdevMonitor(err) => write!(f, "udev monitor: {err}"),
            Self::Io(err) => write!(f, "I/O: {err}"),
        }
    }
}
