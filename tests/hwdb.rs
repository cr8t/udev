use std::sync::Arc;

use udevrs::{Result, Udev, UdevHwdb};

mod common;

#[test]
fn parse_hwdb() -> Result<()> {
    common::init();

    std::env::set_var("UDEV_HWDB_BIN", "./hwdb.bin");
    let udev = Arc::new(Udev::new());

    let _hwdb = UdevHwdb::new(udev)?;

    Ok(())
}
