use std::fmt;

/// Convenience alias for the `udev` library `Result` type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the `udev` library.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    InvalidLen(usize),
    Udev(String),
    UdevDevice(String),
    UdevHwdb(String),
    UdevMonitor(String),
    UdevEnumerate(String),
    UdevQueue(String),
    UdevUtil(String),
    Io(String),
    StdIo {
        kind: std::io::ErrorKind,
        err: String,
    },
}

impl Error {
    /// Convenience function to create a I/O error.
    pub fn std_io<S: Into<String>>(kind: std::io::ErrorKind, err: S) -> Self {
        Self::StdIo {
            kind,
            err: err.into(),
        }
    }

    /// Gets the [ErrorKind](std::io::ErrorKind).
    pub fn kind(&self) -> std::io::ErrorKind {
        match self {
            Self::StdIo { kind, .. } => *kind,
            _ => std::io::ErrorKind::Other,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(format!("{err}"))
    }
}

impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::StdIo { kind, err } => Self::new(kind, err),
            err => Self::new(err.kind(), format!("{err}")),
        }
    }
}

impl From<Error> for std::io::ErrorKind {
    fn from(err: Error) -> Self {
        err.kind()
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

impl From<std::ffi::NulError> for Error {
    fn from(err: std::ffi::NulError) -> Self {
        Self::Io(format!("invalid FFI C-String: {err}"))
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
            Self::UdevEnumerate(err) => write!(f, "udev enumerate: {err}"),
            Self::UdevQueue(err) => write!(f, "udev queue: {err}"),
            Self::UdevUtil(err) => write!(f, "udev util: {err}"),
            Self::Io(err) => write!(f, "I/O: {err}"),
            Self::StdIo { kind, err } => write!(f, "I/O: kind: {kind}, error: {err}"),
        }
    }
}

impl std::error::Error for Error {}
