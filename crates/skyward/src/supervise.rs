//! The half of the watch that lives inside the system.
//!
//! A process cannot report on itself: the one that says how the system ended
//! is the parent that outlived it, which is now a separate binary (asmoday,
//! started as `recovery`). This module keeps only what the system writes for
//! that parent to read — the three files under `/skyward/run` and the beat
//! cadence. The parent's side, including every rule about what the files mean,
//! lives in the asmoday crate. The path strings are repeated here so a change
//! to either side is a visible decision; tests on both sides pin them.

use crate::input::monotonic_ms;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// How long the system waits between two beats. A second wastes nothing on a
/// memory filesystem and leaves thirty chances before silence is believed.
pub(crate) const BEAT_EVERY: u64 = 1000;

/// Where a running system says that it got a frame onto the panel.
///
/// The watcher reads this to tell a system that never came up from one that
/// was up and then stopped. The file is written once, when the first frame
/// reaches the panel.
pub(crate) fn up() -> PathBuf {
    Path::new(skyward_sys::RUN).join("up")
}

/// Where a stopped system leaves the word for the app it stopped for.
///
/// The exit code says what kind of stop it was and nothing more: 67 is one
/// code for every app that is not built yet, so which app it was has to
/// travel beside the code rather than in it.
pub(crate) fn note() -> PathBuf {
    Path::new(skyward_sys::RUN).join("stop")
}

/// Where a running system says it is still running.
fn beat_file() -> PathBuf {
    Path::new(skyward_sys::RUN).join("beat")
}

/// Say that the system is still there. The pages call this as the loop turns.
///
/// What is written is the monotonic clock rather than the time of day: both
/// processes read the same clock, and a phone whose clock is being set has no
/// business looking stopped because of it.
pub(crate) fn beat() -> Result<()> {
    std::fs::write(beat_file(), monotonic_ms().to_string())
        .map_err(|e| anyhow!("say the system is still there: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_the_word_and_the_beat_sit_in_the_recoverys_own_folder() {
        // The watcher on the other side writes the same three paths. All are
        // read and written while the recovery owns the phone, so they live in
        // its own folder rather than one install scripts empty /tmp around.
        assert_eq!(note(), Path::new("/skyward/run/stop"));
        assert_eq!(up(), Path::new("/skyward/run/up"));
        assert_eq!(beat_file(), Path::new("/skyward/run/beat"));
    }
}
