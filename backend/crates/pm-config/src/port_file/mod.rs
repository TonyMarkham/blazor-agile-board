pub(crate) mod port_file_info;

/// Check if a process with the given PID is currently running.
///
/// Uses `kill(pid, 0)` on Unix (checks existence without sending a signal)
/// and `OpenProcess` + `GetExitCodeProcess` on Windows.
#[cfg(unix)]
pub fn is_process_running(pid: u32) -> bool {
    // SAFETY: kill with signal 0 only checks existence, no signal is sent.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
pub fn is_process_running(pid: u32) -> bool {
    use std::ffi::c_void;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    // SAFETY: OpenProcess returns null on failure (process doesn't exist or
    // access denied). GetExitCodeProcess checks if the process is still active.
    // CloseHandle releases the handle. No resources are leaked.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }

        let mut exit_code: u32 = 0;
        let success = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);

        success != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(any(unix, windows)))]
pub fn is_process_running(_pid: u32) -> bool {
    // On exotic platforms (WASM, etc.), assume alive.
    // The CLI will get a connection error if the server is actually dead.
    true
}
