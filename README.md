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

## WIP

Currently, there is only a Rust public API. Work is still ongoing to expose remaining subsystems via the top-level API:

- [x] [Udev] context
- [x] [UdevList] device entry lists
- [x] [UdevDevice] kernel devices
- [ ] [UdevMonitor] device monitor service
- [ ] [UdevEnumerate] device enumeration
- [ ] [UdevQueue] device queue
- [ ] [UdevHwdb] device hardware database persistent storage
- [ ] public C API via FFI
  - after the Rust API stabilizes, work can start on a C API
  - some abstractions will take some work to expose safely through the FFI barrier, e.g. `Arc<Udev>`
