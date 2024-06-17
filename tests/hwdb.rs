use std::sync::Arc;

use udevrs::{Result, Udev, UdevHwdb};

mod common;

#[test]
fn parse_hwdb() -> Result<()> {
    common::init();

    std::env::set_var("UDEV_HWDB_BIN", "./hwdb.bin");
    let udev = Arc::new(Udev::new());

    let mut hwdb = UdevHwdb::new(udev)?;

    let entry = hwdb.get_properties_list_entry("usb:v1D6Bp0001", 0).unwrap();

    assert_eq!(entry.value(), "Linux Foundation");

    Ok(())
}
