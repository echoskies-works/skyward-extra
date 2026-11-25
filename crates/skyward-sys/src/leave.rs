//! The note a system leaves on its way out, and the log it takes with it.
//!
//! A system that is leaving cannot carry out its own power operation and
//! still leave a clean exit behind: the moment it writes `sys.powerctl`,
//! init tears the whole phone down and nobody waits for it. So the system
//! does the two things it can still do — keep its log, and write where it
//! wants to go — and exits. The parent, which is still alive, reads the note
//! and does the power operation on its behalf. The exit code stays clean
//! either way, which is the point: leaving is not a failure.
//!
//! The note is a file, because a file is the plainest thing both sides can
//! read and a desk build can check the whole exchange without a phone. A
//! folder that will not take it is not the end of the road: the parent opens
//! a pipe before the fork and names the writing end in the environment, and
//! the same word goes down that instead. Both places are the same agreement,
//! and only when neither works does the system stay where it is and say so.
//!
//! Everything here is plain files and plain names. Both processes link this
//! module, so the folder, the words and the pipe's name agree because there
//! is one definition of each.

use crate::{Power, RUN};
use std::path::{Path, PathBuf};

/// The note's name inside the shared folder.
const LEAVE: &str = "leave";

/// The name the parent hands the pipe's writing end over in. One definition,
/// read by both sides: the parent sets it before the exec and the system takes
/// it as it comes up.
pub const NOTE_FD: &str = "SKYWARD_LEAVE_FD";

/// Where the note is written, read and then removed.
pub fn leave_file() -> PathBuf {
    Path::new(RUN).join(LEAVE)
}

/// The folder the recovery keeps logs it has not saved to storage yet.
/// The same folder `ui.rs` exports logs into when there is no storage.
const LOGS: &str = "logs";

/// How much of a log one note keeps: the tail, up to this many bytes. A
/// recovery log is normally far smaller; the cap only matters when something
/// has been repeating, and a full tmpfs is one of the things to be careful
/// of on the way out.
const LOG_MOST: usize = 512 << 10;

/// The name of a power target, the inverse of [`Power::parse`]. All six live
/// here so adding a seventh is a decision made in one place: the note must
/// say a word the other side knows, or a reboot becomes no reboot at all.
pub fn name(power: Power) -> &'static str {
    match power {
        Power::System => "system",
        Power::Recovery => "recovery",
        Power::Fastboot => "fastboot",
        Power::Bootloader => "bootloader",
        Power::Edl => "edl",
        Power::Off => "off",
    }
}

/// Leave a note saying where to go.
///
/// The folder is asked first; a folder that will not take the note — full,
/// read-only, gone — is not the end of the road, because the parent left a
/// pipe open before it forked. The same word goes down that instead. Only
/// when neither works does the caller hear of it, and then the system stays
/// where it is and says why, rather than take the phone down itself: nothing
/// in the system writes `sys.powerctl`, and a note nobody could read would
/// leave the phone in the recovery all the same.
pub fn ask(power: Power) -> std::io::Result<()> {
    match ask_into(&leave_file(), power) {
        Ok(()) => Ok(()),
        Err(e) => {
            #[cfg(unix)]
            if ask_fd(power) {
                return Ok(());
            }
            Err(e)
        }
    }
}

/// Take the pipe the parent handed over, if there is one, as the system comes
/// up: the descriptor is marked close-on-exec once, here, so that no program
/// the system starts afterwards inherits a writer it has no use for. A pipe
/// that is not there is no error — a system started by hand has none.
pub fn adopt() {
    #[cfg(unix)]
    {
        let _ = note_fd();
    }
}

/// Write the word down the pipe, where the parent left one.
#[cfg(unix)]
fn ask_fd(power: Power) -> bool {
    note_fd().is_some_and(|fd| ask_into_fd(fd, power))
}

#[cfg(unix)]
fn ask_into_fd(fd: std::os::fd::RawFd, power: Power) -> bool {
    let word = name(power);
    // SAFETY: the descriptor is the parent's writing end, open in this
    // process; one write of a few bytes, and it is not closed here — the
    // process is on its way out and the kernel closes it.
    unsafe { libc::write(fd, word.as_ptr().cast(), word.len()) == word.len() as isize }
}

/// The pipe's writing end, read out of the environment once and closed to
/// every program started after this point.
#[cfg(unix)]
fn note_fd() -> Option<std::os::fd::RawFd> {
    use std::sync::OnceLock;

    static FD: OnceLock<Option<std::os::fd::RawFd>> = OnceLock::new();
    *FD.get_or_init(|| {
        let fd: std::os::fd::RawFd = std::env::var(NOTE_FD).ok()?.parse().ok()?;
        // SAFETY: the descriptor belongs to this process and nothing else
        // touches its flags; this only says it is not to be inherited.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        Some(fd)
    })
}

fn ask_into(path: &Path, power: Power) -> std::io::Result<()> {
    std::fs::write(path, name(power))
}

/// Take what the system left, if anything. An absent or unknown word is no
/// request: an ordinary stop, or a note half-written by a system that died
/// before it finished, must never turn into a reboot. The note is removed on
/// the way out so a later start cannot act on it a second time.
pub fn take() -> Option<Power> {
    take_from(&leave_file())
}

fn take_from(path: &Path) -> Option<Power> {
    let words = std::fs::read_to_string(path).ok()?;
    let _ = std::fs::remove_file(path);
    Power::parse(words.trim())
}

/// Forget any note from an earlier life before this system starts.
pub fn clear() {
    let _ = std::fs::remove_file(leave_file());
}

/// Keep the tail of the recovery log in the recovery's own folder on the way
/// out. Nothing here is worth blocking a shutdown: a log that cannot be read,
/// a folder that cannot be made and a file that will not take the write are
/// all silent. The parent does not need this to decide what to do.
pub fn keep_logs() {
    let _ = keep_logs_between(Path::new(crate::LOG), &Path::new(RUN).join(LOGS));
}

fn keep_logs_between(log: &Path, into: &Path) -> std::io::Result<()> {
    let bytes = std::fs::read(log)?;
    let tail = if bytes.len() > LOG_MOST { &bytes[bytes.len() - LOG_MOST..] } else { &bytes };
    std::fs::create_dir_all(into)?;
    std::fs::write(into.join("recovery.log"), tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("skyward-leave-{}-{name}", std::process::id()))
    }

    /// The note says a word the other side can read back into the same power
    /// target, and the command init gets is unchanged. This is what keeps a
    /// reboot to recovery, the bootloader or EDL exactly what it was before
    /// the parent took over: the note carries the name, `power()` still makes
    /// the command.
    #[test]
    fn every_target_round_trips_and_keeps_its_command() {
        for (power, word, cmd) in [
            (Power::System, "system", "reboot,"),
            (Power::Recovery, "recovery", "reboot,recovery"),
            (Power::Fastboot, "fastboot", "reboot,fastboot"),
            (Power::Bootloader, "bootloader", "reboot,bootloader"),
            (Power::Edl, "edl", "reboot,edl"),
            (Power::Off, "off", "shutdown,"),
        ] {
            assert_eq!(name(power), word);
            assert_eq!(Power::parse(name(power)), Some(power));
            assert_eq!(power.command(), cmd);
        }
    }

    #[test]
    fn the_note_is_read_once_and_words_that_are_not_targets_are_no_request() {
        let path = scratch("note");
        let _ = std::fs::remove_file(&path);
        assert_eq!(take_from(&path), None, "no note at all");

        ask_into(&path, Power::Bootloader).expect("write the note");
        assert_eq!(take_from(&path), Some(Power::Bootloader));
        assert!(!path.exists(), "a note is used up once");

        std::fs::write(&path, "not a target").expect("write a stray note");
        assert_eq!(take_from(&path), None, "an unknown word asks for nothing");
        assert!(!path.exists(), "a stray note is cleared too");
    }

    #[test]
    fn the_log_keeps_its_tail_and_a_missing_log_is_no_error() {
        let dir = scratch("logs");
        let log = scratch("recovery.log");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&log);

        assert!(keep_logs_between(&log, &dir).is_err(), "no log to keep");

        let short = b"a short recovery log";
        std::fs::write(&log, short).expect("write a short log");
        keep_logs_between(&log, &dir).expect("keep the short log");
        assert_eq!(std::fs::read(dir.join("recovery.log")).unwrap(), short);

        let long = vec![b'x'; LOG_MOST + 1024];
        std::fs::write(&log, &long).expect("write a long log");
        keep_logs_between(&log, &dir).expect("keep the long log");
        let kept = std::fs::read(dir.join("recovery.log")).unwrap();
        assert_eq!(kept.len(), LOG_MOST, "only the tail is kept");
        assert_eq!(kept, long[long.len() - LOG_MOST..]);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&log);
    }

    /// A folder that will not take the note is not the end of the road: the
    /// pipe carries the same word, and the parent reads back the target it
    /// names. This is what keeps the system from writing `sys.powerctl`
    /// itself when the memory filesystem is full.
    #[cfg(unix)]
    #[test]
    fn a_note_the_folder_cannot_take_goes_down_the_pipe() {
        let mut ends = [-1; 2];
        assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0, "a pipe");

        assert!(ask_into_fd(ends[1], Power::Bootloader), "the word goes down");
        let mut buf = [0u8; 16];
        let read = unsafe { libc::read(ends[0], buf.as_mut_ptr().cast(), buf.len()) };
        assert!(read > 0, "something came back");
        let word = std::str::from_utf8(&buf[..read as usize]).expect("a word");
        assert_eq!(word, "bootloader", "the same word the note would hold");
        assert_eq!(Power::parse(word), Some(Power::Bootloader), "and the parent reads the target");

        unsafe { libc::close(ends[0]) };
        unsafe { libc::close(ends[1]) };
    }
}
