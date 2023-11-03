use std::fmt;

pub const LOG_EMERG: i32 = 0;
pub const LOG_ALERT: i32 = 1;
pub const LOG_CRIT: i32 = 2;
pub const LOG_ERR: i32 = 3;
pub const LOG_WARNING: i32 = 4;
pub const LOG_NOTICE: i32 = 5;
pub const LOG_INFO: i32 = 6;
pub const LOG_DEBUG: i32 = 7;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub enum LogPriority {
    /// System is untenable
    Emergency = LOG_EMERG,
    /// Action must be taken immediately
    Alert = LOG_ALERT,
    /// Critical conditions
    Critical = LOG_CRIT,
    /// Error conditions
    Error = LOG_ERR,
    /// Warning conditions
    Warning = LOG_WARNING,
    /// Normal but significant condition
    Notice = LOG_NOTICE,
    /// Informational
    #[default]
    Info = LOG_INFO,
    /// Debug-level messages
    Debug = LOG_DEBUG,
}

impl LogPriority {
    /// Creates a new [LogPriority].
    pub const fn new() -> Self {
        Self::Info
    }
}

impl From<i32> for LogPriority {
    fn from(val: i32) -> Self {
        match val {
            LOG_EMERG => Self::Emergency,
            LOG_ALERT => Self::Alert,
            LOG_CRIT => Self::Critical,
            LOG_ERR => Self::Error,
            LOG_WARNING => Self::Warning,
            LOG_NOTICE => Self::Notice,
            LOG_INFO => Self::Info,
            LOG_DEBUG => Self::Debug,
            _ => Self::Info,
        }
    }
}

impl From<&LogPriority> for i32 {
    fn from(val: &LogPriority) -> Self {
        (*val).into()
    }
}

impl From<LogPriority> for i32 {
    fn from(val: LogPriority) -> Self {
        val as i32
    }
}

impl From<&LogPriority> for &'static str {
    fn from(val: &LogPriority) -> Self {
        match val {
            LogPriority::Emergency => "EMERG",
            LogPriority::Alert => "ALERT",
            LogPriority::Critical => "CRITICAL",
            LogPriority::Error => "ERR",
            LogPriority::Warning => "WARNING",
            LogPriority::Notice => "NOTICE",
            LogPriority::Info => "INFO",
            LogPriority::Debug => "DEBUG",
        }
    }
}

impl From<LogPriority> for &'static str {
    fn from(val: LogPriority) -> Self {
        (&val).into()
    }
}

impl fmt::Display for LogPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", <&str>::from(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_priority() {
        assert_eq!(LogPriority::from(LOG_EMERG), LogPriority::Emergency);
        assert_eq!(LogPriority::from(LOG_ALERT), LogPriority::Alert);
        assert_eq!(LogPriority::from(LOG_CRIT), LogPriority::Critical);
        assert_eq!(LogPriority::from(LOG_ERR), LogPriority::Error);
        assert_eq!(LogPriority::from(LOG_WARNING), LogPriority::Warning);
        assert_eq!(LogPriority::from(LOG_NOTICE), LogPriority::Notice);
        assert_eq!(LogPriority::from(LOG_INFO), LogPriority::Info);
        assert_eq!(LogPriority::from(LOG_DEBUG), LogPriority::Debug);

        assert_eq!(i32::from(LogPriority::Emergency), LOG_EMERG);
        assert_eq!(i32::from(LogPriority::Alert), LOG_ALERT);
        assert_eq!(i32::from(LogPriority::Critical), LOG_CRIT);
        assert_eq!(i32::from(LogPriority::Error), LOG_ERR);
        assert_eq!(i32::from(LogPriority::Warning), LOG_WARNING);
        assert_eq!(i32::from(LogPriority::Notice), LOG_NOTICE);
        assert_eq!(i32::from(LogPriority::Info), LOG_INFO);
        assert_eq!(i32::from(LogPriority::Debug), LOG_DEBUG);

        assert_eq!(LogPriority::new(), LogPriority::default());
    }
}
