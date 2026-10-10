//! Free space on the disk that receives the meeting WAV, for the session's disk check.

use std::path::Path;

/// Bytes available to this user on the filesystem holding `path`, or `None` when the platform
/// does not say. The caller treats `None` as unknown and never blocks a recording on it.
#[cfg(target_os = "linux")]
pub(crate) fn free_bytes(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c_path` is a valid NUL-terminated string and `stat` is a properly sized, writable
    // `statvfs` that the call fills in.
    let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if result != 0 {
        return None;
    }
    Some((stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64))
}

// TODO(windows): confirm the number against Explorer's "free space" on the Alienware.
#[cfg(windows)]
pub(crate) fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0u64;
    // SAFETY: `wide` is NUL-terminated and outlives the call; the out pointer is a valid u64.
    unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut available), None, None) }.ok()?;
    Some(available)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn free_bytes(_path: &Path) -> Option<u64> {
    None
}
