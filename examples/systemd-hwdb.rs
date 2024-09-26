use std::env;
use std::process;
use udevrs::{udev_new, UdevHwdb};

/// Simple program to query the systemd hwdb like `systemd-hwdb`
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <hwdb_key>", args[0]);
        process::exit(1);
    }

    let key = &args[1];

    // Initialize the Hwdb
    let mut hwdb = match UdevHwdb::new(udev_new()) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to initialize hwdb: {}", e);
            process::exit(1);
        }
    };

    // Query the hwdb with the provided key
    if let Some(properties) = hwdb.query(key) {
        properties
            .iter()
            .for_each(|e| println!("{}: {}", e.name(), e.value()));
    };
}
