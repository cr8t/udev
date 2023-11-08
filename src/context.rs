use std::{cmp, fmt, fs, io, sync::Arc};

use crate::{file_handle, name_to_handle_at, Error, LogPriority, Result, UdevEntryList, UdevList};

pub const RULES_PATH_LEN: usize = 4;

/// libudev context
///
/// The context contains the default values read from the udev config file,
/// and is passed to all library operations.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Udev {
    sys_path: String,
    dev_path: String,
    rules_path: [String; RULES_PATH_LEN],
    rules_path_ts: [u64; RULES_PATH_LEN],
    run_path: String,
    properties_list: Option<UdevList>,
    log_priority: LogPriority,
}

impl Udev {
    /// Creates a new [Udev].
    pub fn new() -> Self {
        Self {
            sys_path: String::new(),
            dev_path: String::new(),
            rules_path: [""; RULES_PATH_LEN].map(String::from),
            rules_path_ts: [0; RULES_PATH_LEN],
            run_path: String::new(),
            properties_list: None,
            log_priority: LogPriority::new(),
        }
    }

    /// Convenience function for `udev` log messages.
    pub fn log<M: fmt::Display>(&self, priority: LogPriority, msg: M) {
        if priority <= self.log_priority {
            match priority {
                LogPriority::Emergency
                | LogPriority::Alert
                | LogPriority::Critical
                | LogPriority::Error => log::error!("{priority}: {msg}"),
                LogPriority::Warning => log::warn!("{priority}: {msg}"),
                LogPriority::Notice | LogPriority::Info => log::info!("{priority}: {msg}"),
                LogPriority::Debug => log::debug!("{priority}: {msg}"),
            }
        }
    }

    /// Gets the system path.
    pub fn sys_path(&self) -> &str {
        self.sys_path.as_str()
    }

    /// Sets the system path.
    pub fn set_sys_path<P: Into<String>>(&mut self, path: P) {
        self.sys_path = path.into();
    }

    /// Builder function that sets the system path.
    pub fn with_sys_path<P: Into<String>>(mut self, path: P) -> Self {
        self.set_sys_path(path);
        self
    }

    /// Gets the device path.
    pub fn dev_path(&self) -> &str {
        self.dev_path.as_str()
    }

    /// Sets the device path.
    pub fn set_dev_path<P: Into<String>>(&mut self, path: P) {
        self.dev_path = path.into();
    }

    /// Builder function that sets the device path.
    pub fn with_dev_path<P: Into<String>>(mut self, path: P) -> Self {
        self.set_dev_path(path);
        self
    }

    /// Gets a reference to the list of rules paths.
    pub fn rules_path(&self) -> &[String] {
        let len = self.rules_path_count();
        self.rules_path[..len].as_ref()
    }

    /// Gets a mutable reference to the list of rules paths.
    pub fn rules_path_mut(&mut self) -> &mut [String] {
        let len = self.rules_path_count();
        self.rules_path[..len].as_mut()
    }

    /// Sets the list of rules paths.
    pub fn set_rules_path<R: Into<String> + Clone>(&mut self, rules: &[R]) {
        let len = cmp::min(rules.len(), RULES_PATH_LEN);
        self.rules_path[..len]
            .iter_mut()
            .zip(rules[..len].iter().cloned())
            .for_each(|(dst, src)| *dst = src.into());
        self.rules_path[len..]
            .iter_mut()
            .for_each(|s| *s = String::new());
    }

    /// Builder function that sets the list of rules paths.
    pub fn with_rules_path<R: Into<String> + Clone>(mut self, rules: &[R]) -> Self {
        self.set_rules_path(rules);
        self
    }

    /// Gets a reference to the list of rules path timestamps.
    pub fn rules_path_ts(&self) -> &[u64] {
        let len = self.rules_path_count();
        self.rules_path_ts[..len].as_ref()
    }

    /// Gets a mutable reference to the list of rules path timestamps.
    pub fn rules_path_ts_mut(&mut self) -> &mut [u64] {
        let len = self.rules_path_count();
        self.rules_path_ts[..len].as_mut()
    }

    /// Sets the list of rules path timestamps.
    pub fn set_rules_path_ts<T: Into<u64> + Clone>(&mut self, ts: &[T]) {
        let len = cmp::min(ts.len(), RULES_PATH_LEN);
        self.rules_path_ts[..len]
            .iter_mut()
            .zip(ts[..len].iter().cloned())
            .for_each(|(dst, src)| *dst = src.into());
    }

    /// Builder function that sets the list of rules path timestamps.
    pub fn with_rules_path_ts<T: Into<u64> + Clone>(mut self, ts: &[T]) -> Self {
        self.set_rules_path_ts(ts);
        self
    }

    /// Gets the number of populated rules path entries.
    pub fn rules_path_count(&self) -> usize {
        self.rules_path.iter().filter(|p| !p.is_empty()).count()
    }

    /// Gets the run path.
    pub fn run_path(&self) -> &str {
        self.run_path.as_str()
    }

    /// Sets the run path.
    pub fn set_run_path<P: Into<String>>(&mut self, path: P) {
        self.run_path = path.into();
    }

    /// Builder function that sets the run path.
    pub fn with_run_path<P: Into<String>>(mut self, path: P) -> Self {
        self.set_run_path(path);
        self
    }

    /// Gets a reference to the properties list [UdevList].
    ///
    /// **NOTE** User is responsible for initializing the [`properties_list`](UdevList) before calling.
    pub fn properties_list(&self) -> Result<&UdevList> {
        self.properties_list
            .as_ref()
            .ok_or(Error::Udev("context: missing properties_list".into()))
    }

    /// Gets a mutable reference to the properties list [UdevList].
    ///
    /// **NOTE** User is responsible for initializing the [`properties_list`](UdevList) before calling.
    pub fn properties_list_mut(&mut self) -> Result<&mut UdevList> {
        self.properties_list
            .as_mut()
            .ok_or(Error::Udev("context: missing properties_list".into()))
    }

    /// Sets the properties list [UdevList].
    pub fn set_properties_list<L: Into<UdevEntryList>>(arc: &mut Arc<Self>, list: L) {
        let udev_arc = Arc::clone(arc);
        let udev = Arc::make_mut(arc);

        match udev.properties_list.as_mut() {
            Some(prop_list) => prop_list.set_list(list),
            None => udev.properties_list = Some(UdevList::create(udev_arc, list.into())),
        }
    }

    /// Builder function that sets the properties list [UdevList].
    pub fn with_properties_list<L: Into<UdevEntryList>>(mut arc: Arc<Self>, list: L) -> Arc<Self> {
        Udev::set_properties_list(&mut arc, list);
        arc
    }

    /// Gets the [LogPriority].
    pub const fn log_priority(&self) -> LogPriority {
        self.log_priority
    }

    /// Sets the [LogPriority].
    pub fn set_log_priority<P: Into<LogPriority>>(&mut self, priority: P) {
        self.log_priority = priority.into();
    }

    /// Builder function that sets the [LogPriority].
    pub fn with_log_priority<P: Into<LogPriority>>(mut self, priority: P) -> Self {
        self.set_log_priority(priority);
        self
    }

    /// Gets whether `/dev` is mounted on `devtmpfs`.
    pub fn has_devtmpfs(&self) -> bool {
        use io::BufRead;

        let mut handle = file_handle::new();
        let mut mount_id = 0i32;

        if let (Ok(f), Ok(_)) = (
            fs::OpenOptions::new()
                .read(true)
                .open("/proc/self/mountinfo"),
            name_to_handle_at(libc::AT_FDCWD, "/dev", &mut handle, &mut mount_id, 0),
        ) {
            let mut reader = io::BufReader::new(f);
            let mut line = String::new();
            let mut ret = false;

            while reader.read_line(&mut line).is_ok() {
                if let Ok(mid) = line.parse::<i32>() {
                    if mid != mount_id {
                        continue;
                    }
                } else {
                    continue;
                }

                if let Some(e) = line.find(" - ") {
                    if let Some(p) = line[e..].strip_prefix(" - ") {
                        // accept any name that starts with the currently expected type
                        if p.starts_with("devtmpfs") {
                            ret = true;
                            break;
                        }
                    }
                }
            }

            ret
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UdevEntry;

    #[test]
    fn test_udev() -> Result<()> {
        let mut null_udev = Udev::new();

        let exp_sys_path = "test_sys_path";
        let exp_dev_path = "test_dev_path";
        let exp_rules_path =
            ["test_rules0", "test_rules1", "test_rules2", "test_rules3"].map(String::from);
        let exp_rules_ts = [17092390, 17092391, 17092392, 17092393];
        let exp_run_path = "test_run_path";
        let exp_prop_list = [UdevEntry::new().with_name("test_entry_name")];
        let exp_log_prio = LogPriority::Debug;

        let exp_udev = Udev::new()
            .with_sys_path(exp_sys_path)
            .with_dev_path(exp_dev_path)
            .with_rules_path(&exp_rules_path)
            .with_rules_path_ts(&exp_rules_ts)
            .with_run_path(exp_run_path)
            .with_log_priority(exp_log_prio);

        assert_eq!(null_udev.sys_path(), "");
        assert_eq!(null_udev.dev_path(), "");
        assert!(null_udev.rules_path().is_empty());
        assert!(null_udev.rules_path_ts().is_empty());
        assert_eq!(null_udev.run_path(), "");
        assert!(null_udev.properties_list().is_err());
        assert_eq!(null_udev.log_priority(), LogPriority::new());

        assert_eq!(exp_udev.sys_path(), exp_sys_path);
        assert_eq!(exp_udev.dev_path(), exp_dev_path);
        assert_eq!(exp_udev.rules_path(), exp_rules_path.as_ref());
        assert_eq!(exp_udev.rules_path_ts(), exp_rules_ts.as_ref());
        assert_eq!(exp_udev.run_path(), exp_run_path);

        assert_eq!(exp_udev.log_priority(), exp_log_prio);

        null_udev.set_sys_path(exp_sys_path);
        assert_eq!(null_udev.sys_path(), exp_sys_path);

        null_udev.set_dev_path(exp_dev_path);
        assert_eq!(null_udev.dev_path(), exp_dev_path);

        null_udev.set_rules_path(&exp_rules_path);
        assert_eq!(null_udev.rules_path(), exp_rules_path.as_ref());

        null_udev.set_rules_path_ts(&exp_rules_ts);
        assert_eq!(null_udev.rules_path_ts(), exp_rules_ts.as_ref());

        null_udev.set_run_path(exp_run_path);
        assert_eq!(null_udev.run_path(), exp_run_path);

        null_udev.set_log_priority(exp_log_prio);
        assert_eq!(null_udev.log_priority(), exp_log_prio);

        assert_eq!(null_udev, exp_udev);

        // Check that setting a short rules path only returns the short list.
        null_udev.set_rules_path(&exp_rules_path[..1]);
        assert_eq!(null_udev.rules_path(), &exp_rules_path[..1]);
        assert_eq!(null_udev.rules_path_mut(), &exp_rules_path[..1]);
        assert_eq!(null_udev.rules_path_ts(), &exp_rules_ts[..1]);

        // Check that a maximum of `RULES_PATH_LEN` items are set
        let oversize_rules_path = ["over"; RULES_PATH_LEN + 1].map(String::from);
        null_udev.set_rules_path(&oversize_rules_path);
        assert_eq!(
            null_udev.rules_path(),
            &oversize_rules_path[..RULES_PATH_LEN]
        );
        assert_eq!(null_udev.rules_path_ts(), &exp_rules_ts);

        let mut null_udev = Arc::new(null_udev);
        Udev::set_properties_list(&mut null_udev, exp_prop_list.clone());

        for (prop, exp_prop) in null_udev
            .properties_list()
            .unwrap()
            .iter()
            .zip(exp_prop_list.iter())
        {
            assert_eq!(prop, exp_prop);
        }

        let exp_udev =
            Udev::with_properties_list(Arc::clone(&mut null_udev), exp_prop_list.clone());

        for (prop, exp_prop) in exp_udev
            .properties_list()
            .unwrap()
            .iter()
            .zip(exp_prop_list.iter())
        {
            assert_eq!(prop, exp_prop);
        }

        assert_eq!(null_udev, exp_udev);

        Ok(())
    }
}
