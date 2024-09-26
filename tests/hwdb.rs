use std::sync::Arc;

use udevrs::{Error, Result, Udev, UdevHwdb};

mod common;

#[test]
fn parse_hwdb() -> Result<()> {
    common::init();

    std::env::set_var("UDEV_HWDB_BIN", "./hwdb.bin");
    let udev = Arc::new(Udev::new());

    let mut hwdb = UdevHwdb::new(udev)?;

    // vendor
    let root_hub = hwdb.query("usb:v1D6B").ok_or(Error::UdevHwdb("no matching entry found for usb:v1D6B".into()))?
        .iter()
        .find(|e| e.name() == "ID_VENDOR_FROM_DATABASE")
        .map(|e| (e.value()));

    assert_eq!(root_hub, Some("Linux Foundation"));

    // pci in example
    let pci = hwdb.query("pci:v00008086d00001C2D*").ok_or(Error::UdevHwdb("no matching entry found for pci:v00008086d00001C2D".into()))?
        .iter()
        .find(|e| e.name() == "ID_VENDOR_FROM_DATABASE")
        .map(|e| (e.value()));

    assert_eq!(pci, Some("Intel Corporation"));


    // vendor and product
    let root_hub_30 = hwdb.query("usb:v1D6Bp0003").ok_or(Error::UdevHwdb("no matching entry found for usb:v1D6Bp0003".into()))?
        .iter()
        .find(|e| e.name() == "ID_MODEL_FROM_DATABASE")
        .map(|e| (e.value()));

    assert_eq!(root_hub_30, Some("3.0 root hub"));

    // class
    let hid = hwdb.query("usb:v*p*d*dc03*").ok_or(Error::UdevHwdb("no matching entry found for usb:v*p*d*dc03*".into()))?
        .iter()
        .find(|e| e.name() == "ID_USB_CLASS_FROM_DATABASE")
        .map(|e| (e.value()));

    assert_eq!(hid, Some("Human Interface Device"));

    // class, subclass and protocol
    let at = hwdb.query("usb:v*p*d*dc02dsc02dp05*").ok_or(Error::UdevHwdb("no matching entry found for usb:v*p*d*dc02dsc02dp05*".into()))?
        .iter()
        .find(|e| e.name() == "ID_USB_PROTOCOL_FROM_DATABASE")
        .map(|e| (e.value()));

    assert_eq!(at, Some("AT-commands (3G)"));

    Ok(())
}
