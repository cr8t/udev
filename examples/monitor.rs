use std::sync::Arc;
use std::{thread, time};

fn main() -> udevrs::Result<()> {
    env_logger::init();
    let udev = Arc::new(udevrs::Udev::new());
    let mut monitor = udevrs::UdevMonitor::new_from_netlink(udev, "udev")?;

    monitor.enable_receiving()?;

    loop {
        println!("{:?}", monitor.receive_device());

        thread::sleep(time::Duration::from_secs(1));
    }
}
