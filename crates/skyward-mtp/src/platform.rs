//! The few Unix facts the MTP responder uses that a computer running Windows
//! has no direct call for. The phone keeps its own semantics here; the
//! computer's answers are only used by the platform-independent tests.

use std::fs::Metadata;
use std::io;
use std::path::Path;

/// A file descriptor, which the phone waits on for storage changes.
#[cfg(unix)]
pub type RawFd = std::os::fd::RawFd;
#[cfg(not(unix))]
pub type RawFd = i32;

/// The time an entry was last changed, in Unix seconds.
#[cfg(unix)]
pub fn mtime(meta: &Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.mtime()
}
#[cfg(not(unix))]
pub fn mtime(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| jiff::Timestamp::try_from(t).ok())
        .map(|t| t.as_second())
        .unwrap_or(0)
}

/// The bytes of a name, for the hash that makes an entry's ID stable.
#[cfg(unix)]
pub fn name_bytes(name: &std::ffi::OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    name.as_bytes().to_vec()
}
#[cfg(not(unix))]
pub fn name_bytes(name: &std::ffi::OsStr) -> Vec<u8> {
    name.to_string_lossy().as_bytes().to_vec()
}

/// Give an entry the time the computer asked for, leaving its access time
/// alone. The phone sets only the modification time through `utimensat`;
/// Windows files offer the same directly.
#[cfg(unix)]
pub fn set_modified(path: &Path, unix: i64) -> io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a NUL in the path"))?;
    let times = [
        libc::timespec { tv_sec: 0, tv_nsec: libc::UTIME_OMIT },
        libc::timespec { tv_sec: unix as _, tv_nsec: 0 },
    ];
    if unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
#[cfg(not(unix))]
pub fn set_modified(path: &Path, unix: i64) -> io::Result<()> {
    let at = jiff::Timestamp::from_second(unix)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(std::time::SystemTime::from(at))
}

/// Whether a write failed because the storage is full.
#[cfg(unix)]
pub fn is_storage_full(e: &io::Error) -> bool {
    e.raw_os_error() == Some(libc::ENOSPC)
}
#[cfg(not(unix))]
pub fn is_storage_full(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::StorageFull
}
