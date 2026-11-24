//! The filesystem facts a recovery keeps that a computer running Windows has
//! no counterpart for. Unix keeps its own semantics here; elsewhere each call
//! degrades explicitly, so the crate still builds and its platform-independent
//! callers can be checked and tested on a computer. None of the degraded paths
//! run in the recovery, which is Linux.

use std::fs::{self, File};
use std::io;
use std::path::Path;

#[cfg(unix)]
use anyhow::{Context, Result};
#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};

#[cfg(not(unix))]
use anyhow::{bail, Context, Result};

/// The device an entry lives on, used to tell a mounted filesystem apart.
/// Windows has no such number; without mounts to detect, every entry reads as
/// being on the same device.
#[cfg(unix)]
pub fn dev(meta: &fs::Metadata) -> u64 {
    meta.dev()
}
#[cfg(not(unix))]
pub fn dev(_: &fs::Metadata) -> u64 {
    0
}

/// The number that tells two names for one entry apart. Windows gives none
/// here, so an entry is only ever itself by its path.
#[cfg(unix)]
pub fn ino(meta: &fs::Metadata) -> u64 {
    meta.ino()
}
#[cfg(not(unix))]
pub fn ino(_: &fs::Metadata) -> u64 {
    0
}

/// The permission and type bits. Windows filesystems keep no Unix mode.
#[cfg(unix)]
pub fn mode(meta: &fs::Metadata) -> u32 {
    meta.mode()
}
#[cfg(not(unix))]
pub fn mode(_: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
pub fn uid(meta: &fs::Metadata) -> u32 {
    meta.uid()
}
#[cfg(not(unix))]
pub fn uid(_: &fs::Metadata) -> u32 {
    0
}

#[cfg(unix)]
pub fn gid(meta: &fs::Metadata) -> u32 {
    meta.gid()
}
#[cfg(not(unix))]
pub fn gid(_: &fs::Metadata) -> u32 {
    0
}

/// How many names share an entry's contents. One, on a filesystem that has no
/// hard links.
#[cfg(unix)]
pub fn nlink(meta: &fs::Metadata) -> u64 {
    meta.nlink()
}
#[cfg(not(unix))]
pub fn nlink(_: &fs::Metadata) -> u64 {
    1
}

/// The device number a special file stands for.
#[cfg(unix)]
pub fn rdev(meta: &fs::Metadata) -> u64 {
    meta.rdev()
}
#[cfg(not(unix))]
pub fn rdev(_: &fs::Metadata) -> u64 {
    0
}

/// A new file with the private mode the recovery gives the files it makes.
#[cfg(unix)]
pub fn create_new_mode(path: &Path) -> io::Result<File> {
    fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
}
#[cfg(not(unix))]
pub fn create_new_mode(path: &Path) -> io::Result<File> {
    fs::OpenOptions::new().write(true).create_new(true).open(path)
}

/// Open a file without waiting for a writer, so a pipe can be told for what
/// it is. Windows has no non-blocking flag to pass here.
#[cfg(unix)]
pub fn open_nonblock(path: &Path) -> io::Result<File> {
    fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path)
}
#[cfg(not(unix))]
pub fn open_nonblock(path: &Path) -> io::Result<File> {
    fs::File::open(path)
}

/// Give an entry its owner without following a link.
#[cfg(unix)]
pub fn chown(path: &Path, uid: u32, gid: u32) -> Result<()> {
    std::os::unix::fs::lchown(path, Some(uid), Some(gid)).with_context(|| format!("give {} its owner", path.display()))
}
#[cfg(not(unix))]
pub fn chown(_: &Path, _: u32, _: u32) -> Result<()> {
    Ok(())
}

/// Give an entry its permission bits. Windows filesystems keep no Unix mode;
/// this is a no-op there, and `mode` correspondingly reads back zero.
#[cfg(unix)]
pub fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).with_context(|| format!("give {} its permissions", path.display()))
}
#[cfg(not(unix))]
pub fn set_mode(_: &Path, _: u32) -> Result<()> {
    Ok(())
}

/// Give a copy the times the original had, without following a link.
#[cfg(unix)]
pub fn set_times(path: &Path, meta: &fs::Metadata) -> Result<()> {
    let c = CString::new(path.as_os_str().as_bytes()).with_context(|| format!("{} holds a NUL", path.display()))?;
    let times = [
        libc::timespec { tv_sec: meta.atime() as _, tv_nsec: meta.atime_nsec() as _ },
        libc::timespec { tv_sec: meta.mtime() as _, tv_nsec: meta.mtime_nsec() as _ },
    ];
    if unsafe { libc::utimensat(libc::AT_FDCWD, c.as_ptr(), times.as_ptr(), libc::AT_SYMLINK_NOFOLLOW) } != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| format!("give {} its times", path.display()));
    }
    Ok(())
}
#[cfg(not(unix))]
pub fn set_times(_: &Path, _: &fs::Metadata) -> Result<()> {
    Ok(())
}

/// Whether an entry is a device node or a pipe, which a copy makes by call.
#[cfg(unix)]
pub fn is_special(kind: &fs::FileType) -> bool {
    kind.is_block_device() || kind.is_char_device() || kind.is_fifo()
}
#[cfg(not(unix))]
pub fn is_special(_: &fs::FileType) -> bool {
    false
}

/// Make a device node or a pipe of the kind and device the entry had.
#[cfg(unix)]
pub fn make_node(path: &Path, mode: u32, rdev: u64) -> Result<()> {
    let c = CString::new(path.as_os_str().as_bytes()).with_context(|| format!("{} holds a NUL", path.display()))?;
    if unsafe { libc::mknod(c.as_ptr(), mode as libc::mode_t, rdev as libc::dev_t) } != 0 {
        return Err(std::io::Error::last_os_error()).with_context(|| format!("make {}", path.display()));
    }
    Ok(())
}
#[cfg(not(unix))]
pub fn make_node(path: &Path, _: u32, _: u64) -> Result<()> {
    bail!("{} is a special file, which this computer cannot make", path.display())
}

/// Make a link pointing where the original does. Windows needs to be told
/// whether it points at a file or a folder.
#[cfg(unix)]
pub fn symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link).with_context(|| format!("make the link {}", link.display()))
}
#[cfg(not(unix))]
pub fn symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::windows::fs::symlink_file(target, link).with_context(|| format!("make the link {}", link.display()))
}

/// The error a move gets when it crosses a filesystem and must copy instead.
#[cfg(unix)]
pub const EXDEV: i32 = libc::EXDEV;
#[cfg(not(unix))]
pub const EXDEV: i32 = 17; // ERROR_NOT_SAME_DEVICE
