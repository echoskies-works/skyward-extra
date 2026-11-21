//! What a system says about the line it died on.
//!
//! The parent can only see an exit code, and in a release build every panic
//! is the same code: `panic = "abort"` turns all of them into SIGABRT, 134.
//! The one thing that would say which line went wrong went to stderr, and
//! from there into whichever destination the init script happened to choose.
//! The dying process now writes it itself, into the folder the parent already
//! reads for its beat, its up flag and its note.
//!
//! The hook deliberately does two small things and nothing else: print the
//! line, since replacing the default hook stops it printing for us, and put
//! the same line in a file. It does not open the kernel log and does not
//! touch the log's mutex — either could be held by the very panic being
//! reported, and a second panic here would leave no report at all.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// The recovery's own folder, shared with the watcher on the other side of
/// the fork. Repeated here so a change to one side has to be matched by a
/// decision on the other; both sides pin the resulting path in a test.
const RUN: &str = "/skyward/run";

/// Only the first report is kept. A hook that is reached twice is already a
/// double panic, which aborts whether this writes anything or not.
static WROTE: AtomicBool = AtomicBool::new(false);

/// Where the line about a panic is left for the parent to read.
pub(crate) fn panic_file() -> PathBuf {
    Path::new(RUN).join("panic")
}

/// Put the hook in place before anything that can panic, including the setup
/// of the folder it writes into. A panic in that setup still leaves a line.
#[cfg(target_os = "android")]
pub(crate) fn install() {
    // The watcher makes this folder before the fork, but a system started
    // by hand has no watcher and should still be able to leave a note.
    let _ = std::fs::create_dir_all(RUN);
    // A death from an earlier life would be read as this one's.
    let _ = std::fs::remove_file(panic_file());
    std::panic::set_hook(Box::new(|info| {
        let line = line_of(info);
        // set_hook replaces the default hook, which is what normally prints
        // the message. Without this, a panic is silent everywhere but here.
        eprintln!("{line}");
        note(&line);
    }));
}

/// A desk build has no parent to read the file and no folder to put it in.
#[cfg(not(target_os = "android"))]
pub(crate) fn install() {}

/// `file:line:column: words`, on one line, so a reader never has to decide
/// which part of it is the location.
fn line_of(info: &std::panic::PanicHookInfo<'_>) -> String {
    let at = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown location".to_string());
    let words = if let Some(s) = info.payload().downcast_ref::<&str>() {
        *s
    } else if let Some(s) = info.payload().downcast_ref::<String>() {
        s.as_str()
    } else {
        "no message"
    };
    format!("{at}: {words}")
}

/// Write what was caught, once. Errors are dropped on purpose: a folder that
/// has filled up or gone read-only is one of the things this is meant to
/// report, and a failing write must not become a second panic.
fn note(line: &str) {
    if WROTE.swap(true, Ordering::Relaxed) {
        return;
    }
    let _ = std::fs::write(panic_file(), line.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn the_report_sits_beside_the_beat() {
        assert_eq!(panic_file(), Path::new("/skyward/run/panic"));
    }

    /// The line is taken from a real panic, not from a line of text written
    /// here to resemble one: the payload can be a `&str` or a `String`, and
    /// the location is supplied by the compiler rather than by a test.
    #[test]
    fn a_real_panic_gives_up_its_line_and_its_words() {
        let got = Arc::new(Mutex::new(String::new()));
        let sink = got.clone();
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            *sink.lock().expect("no other hook here") = line_of(info);
        }));
        let caught = std::panic::catch_unwind(|| panic!("the wheels fell off"));
        std::panic::set_hook(before);
        assert!(caught.is_err(), "the panic really happened");
        let line = got.lock().expect("the hook is finished").clone();
        assert!(line.contains("the wheels fell off"), "{line}");
        assert!(line.contains("paniclog.rs"), "{line}");
        // file:line:column, then the message: three separators before it.
        assert!(line.matches(':').count() >= 3, "the location is complete: {line}");
    }
}
