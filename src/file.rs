use std::{ffi, io};

use crate::{Error, Result};

/// Represents an FFI type from `fcntl.h` for a `file_handle`.
#[repr(C)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct file_handle {
    handle_bytes: u8,
    handle_type: i32,
    f_handle: *mut u8,
}

impl file_handle {
    /// Creates a new [file_handle].
    pub const fn new() -> Self {
        Self {
            handle_bytes: 0,
            handle_type: 0,
            f_handle: std::ptr::null_mut(),
        }
    }

    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }

    pub fn as_void_ptr(&self) -> *const ffi::c_void {
        self.as_ptr() as *const _
    }

    pub fn as_ptr_mut(&mut self) -> *mut Self {
        self as *mut _
    }

    pub fn as_void_ptr_mut(&mut self) -> *mut ffi::c_void {
        self.as_ptr_mut() as *mut _
    }
}

/// Wrapper around a syscall that returns an opaque handle that corresponds to a specified file.
pub fn name_to_handle_at(
    dir_fd: i32,
    path: &str,
    handle: &mut file_handle,
    mount_id: &mut i32,
    flags: i32,
) -> Result<()> {
    let path_str = ffi::CString::new(path)?;

    // SAFETY: parameters are initialized properly, and pointers reference valid memory.
    let ret = unsafe {
        libc::syscall(
            libc::SYS_name_to_handle_at,
            dir_fd,
            path_str.as_ptr(),
            handle.as_void_ptr_mut(),
            mount_id as *mut i32,
            flags,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        let errno = io::Error::last_os_error();
        let errmsg = format!("error calling `name_to_handle_at`, ret: {ret}, errno: {errno}");

        log::warn!("{errmsg}");

        Err(Error::Io(errmsg))
    }
}
