//! The line a dying system wrote about itself.
//!
//! The child puts a `file:line:column: words` line in this folder as its
//! panic hook runs. The parent reads it after the child is gone, when the
//! exit code can say that it died but not where. Nothing here depends on the
//! child still being alive or on the panel working: it is a file left behind,
//! to be picked up on the way to the failure screen.
//!
//! Kept free of `libc` and of `skyward-sys` so it can be tested on a desk,
//! where there is no recovery folder and no child to fail.

use std::path::{Path, PathBuf};

/// The recovery's own folder. Repeated from the watcher's side on purpose:
/// each side has its own test pinning the path, so a change shows up as a
/// decision in two places rather than as a silent disagreement.
const RUN: &str = "/skyward/run";

/// Where the system leaves the words it died with.
pub(crate) fn panic_file() -> PathBuf {
    Path::new(RUN).join("panic")
}

/// Read the report, if there is one.
///
/// An absent or empty file is no report. A panic before the hook was in
/// place, a panic whose file could not be written, and a system that stopped
/// without panicking all look the same from here, and none of them is worth
/// a line that claims to know where it happened.
pub(crate) fn read() -> Option<String> {
    read_from(&panic_file())
}

fn read_from(path: &Path) -> Option<String> {
    let words = std::fs::read_to_string(path).ok()?;
    let words = words.trim();
    if words.is_empty() {
        None
    } else {
        Some(words.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("asmoday-{}-{name}", std::process::id()))
    }

    #[test]
    fn the_report_is_read_from_beside_the_beat() {
        assert_eq!(panic_file(), Path::new("/skyward/run/panic"));
    }

    #[test]
    fn a_system_that_left_no_words_has_no_report() {
        let path = scratch("absent-panic");
        let _ = std::fs::remove_file(&path);
        assert_eq!(read_from(&path), None, "no file at all");
        std::fs::write(&path, "  \n\t").expect("write a blank note");
        assert_eq!(read_from(&path), None, "a file with nothing in it");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_words_left_behind_are_kept_whole() {
        let path = scratch("panic-words");
        let line = "src/ui.rs:1234:9: called Option::unwrap() on a None value";
        std::fs::write(&path, format!("{line}\n")).expect("write a panic note");
        assert_eq!(read_from(&path).as_deref(), Some(line));
        let _ = std::fs::remove_file(&path);
    }
}
