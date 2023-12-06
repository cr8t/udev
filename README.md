# udev

Rust port of the [`eudev`](https://github.com/eudev-project/eudev) project for interacting with the Linux `devfs` filesystem.

The project attempts to maintain a public API as close to possible with the original C project.

This library is an init-system independent implementation, just like `eudev`.

## Safety

As much as possible, code is written in safe Rust. Some points of interaction with the Linux API require `unsafe` code.

All `unsafe` code is wrapped in safe interfaces, and documented with `SAFETY` comments.

There are no C dependencies.

## Rust API

All Rust structs have a public API that is somewhat close to counterparts in the `eudev` library.

See library documentation for usage.

To generate documentation locally:

```bash
$ cd udev
$ cargo doc --all --open
```

As the project matures, use-case examples will be added to doc-tests.

- [x] [Udev](src/context.rs) context
- [x] [UdevList](src/list.rs) device entry lists
- [x] [UdevDevice](src/device.rs) kernel devices
- [x] [UdevMonitor](src/monitor.rs) device monitor service
- [x] [UdevEnumerate](src/enumerate.rs) device enumeration
- [x] [UdevQueue](src/queue.rs) device queue
- [x] [UdevHwdb](src/hwdb.rs) device hardware database persistent storage
- [x] [Top-level API](src/lib.rs) matches closely to original `libudev` API
  - basis for a future C API via FFI

## WIP

Currently, there is only a Rust public API. Work is still ongoing to expose remaining subsystems via the top-level API:
- [ ] public C API via FFI
  - after the Rust API stabilizes, work can start on a C API
  - some abstractions will take some work to expose safely through the FFI barrier, e.g. `Arc<Udev>`
