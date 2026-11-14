//! What a stopped system's numbers mean.
//!
//! The parent of the system is the only thing left to say why it is not
//! running, and the only things it has to go on are an exit status, whether a
//! frame ever reached the panel, and what the system left in a note. All three
//! are read here, away from the process that gathers them, so the rule that
//! decides what a person sees on a broken phone can be checked on a desk —
//! that process only exists on a phone, and a rule nobody can run is a rule
//! nobody knows.

use skyward_startup::{Crash, Reason};
use std::time::Duration;

/// How long the system may go without saying it is still there.
///
/// Half a minute is not tight. The pages turn at least once a second even at
/// rest, and everything that takes longer than that — an install, an erase, a
/// backup, a filesystem being repaired — runs on a thread of its own while the
/// loop keeps turning. What is left is a genuine stop: a driver waiting
/// forever, a lock held by nothing.
///
/// It goes with [`crate::ui::Pages::may_sleep`] as it stands: the phone is not
/// suspended at all, so a quiet loop is a stopped one. Turning suspend back on
/// would make a sleeping system silent, and silence here means a failure.
pub(crate) const SILENCE: Duration = Duration::from_secs(30);

/// The code reported for a system that was stopped for being silent.
///
/// The parent is the one that sent the signal, so the child's own status says
/// 137 and nothing about why. 137 is kept anyway: killed is what happened, and
/// a code invented here would be one more number nobody can look up.
// Only the phone has a parent to report it, so a desk build has no use for it.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
pub(crate) const KILLED: i32 = 137;

/// How a child ended, as far as its parent can tell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum End {
    /// It left of its own accord and asked for nothing: nothing to report.
    Clean,
    /// Something outside asked it to stop — the phone restarting, init taking
    /// the service down. Not a failure, and not the panel's business:
    /// whatever asked is already in charge of what happens next.
    Asked,
    /// It stopped with this code, in this way, for this reason where there is
    /// one.
    Stopped { code: i32, crash: Crash, reason: Option<Reason> },
}

/// Whether a system that last said `beat` has been quiet for too long.
///
/// A beat older than [`SILENCE`] is silence; no beat at all counts from the
/// fork, which is all that is known to have happened by then.
pub(crate) fn overdue(beat: Option<u64>, now: u64, waited: Duration) -> bool {
    match beat {
        Some(at) => now.saturating_sub(at) >= SILENCE.as_millis() as u64,
        None => waited >= SILENCE,
    }
}

/// What the status the child left means, in the terms the panel uses.
///
/// `up` is whether a frame ever reached the panel, which is what tells a
/// system that never came up from one that was in use a second ago. `note` is
/// what the child left behind, where it stopped on purpose.
pub(crate) fn ended(status: i32, up: bool, note: Option<&str>) -> End {
    let crash = if up { Crash::Runtime } else { Crash::Startup };
    if libc::WIFSIGNALED(status) {
        let signal = libc::WTERMSIG(status);
        // Asked to stop, not failed: the phone is restarting, or init is
        // taking the service down for a reason of its own. A failure screen
        // over that would be shown to somebody already leaving the page.
        if matches!(signal, libc::SIGHUP | libc::SIGINT | libc::SIGTERM) {
            return End::Asked;
        }
        // Any other signal is a stop the system did not choose, reported the
        // way a shell reports it: 139 is a segmentation fault, 137 the system
        // killing it for memory, 134 a failed assert.
        return End::Stopped { code: 128 + signal, crash, reason: reason(note) };
    }
    if libc::WIFEXITED(status) {
        let code = libc::WEXITSTATUS(status);
        if code != 0 {
            return End::Stopped { code, crash, reason: reason(note) };
        }
    }
    End::Clean
}

/// The word in the note, where it names an app this panel knows.
///
/// An unknown word is no reason at all rather than a wrong one: a note written
/// by a newer Aether than this panel knows still leaves the code on the
/// screen, which is the part that narrows anything down.
fn reason(note: Option<&str>) -> Option<Reason> {
    note.and_then(Reason::from_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `waitpid` reports for a child that exited with `code`.
    ///
    /// The status is not the code: an exit status carries the code in its high
    /// byte, and a number put in whole is read as a signal instead.
    fn exited(code: i32) -> i32 {
        code << 8
    }

    #[test]
    fn a_system_asked_to_stop_is_not_a_system_that_failed() {
        // The phone restarting and init taking the service down both arrive as
        // one of these three, and neither is something to put on a screen.
        for signal in [libc::SIGHUP, libc::SIGINT, libc::SIGTERM] {
            assert_eq!(ended(signal, true, None), End::Asked, "{signal}");
        }
    }

    #[test]
    fn a_signal_nobody_asked_for_is_reported_as_the_shell_would() {
        // 139 is what a person looks up; anything else would be a number of our
        // own invention.
        let startup = |code| End::Stopped { code, crash: Crash::Startup, reason: None };
        assert_eq!(ended(libc::SIGSEGV, false, None), startup(139));
        assert_eq!(ended(libc::SIGKILL, false, None), startup(137));
        assert_eq!(ended(libc::SIGABRT, false, None), startup(134));
    }

    #[test]
    fn a_clean_exit_has_nothing_to_report() {
        assert_eq!(ended(0, true, None), End::Clean);
        assert_eq!(ended(0, false, None), End::Clean);
    }

    #[test]
    fn a_frame_on_the_panel_is_what_makes_it_have_stopped_running() {
        // The same code says two different things depending on whether the
        // system ever got a screen up, and that is the whole point of the two
        // failure panels.
        assert_eq!(
            ended(libc::SIGSEGV, true, None),
            End::Stopped { code: 139, crash: Crash::Runtime, reason: None }
        );
        assert_eq!(
            ended(exited(67), true, Some("music")),
            End::Stopped { code: 67, crash: Crash::Runtime, reason: Some(Reason::Music) }
        );
    }

    #[test]
    fn a_word_the_panel_does_not_know_leaves_the_code_alone() {
        // A newer system may stop for an app this panel has never heard of.
        assert_eq!(
            ended(exited(67), true, Some("calculator")),
            End::Stopped { code: 67, crash: Crash::Runtime, reason: None }
        );
    }

    #[test]
    fn silence_is_believed_only_once_half_a_minute_has_passed() {
        let now = 100_000;
        let beat = |ago: u64| Some(now - ago);
        assert!(!overdue(beat(0), now, Duration::ZERO), "just said so");
        assert!(!overdue(beat(29_000), now, Duration::from_secs(29)), "still inside the wait");
        assert!(overdue(beat(30_000), now, Duration::from_secs(30)), "half a minute of nothing");
        // A system that never spoke counts from the fork: the beat is what it
        // would have written first.
        assert!(!overdue(None, now, Duration::from_secs(29)));
        assert!(overdue(None, now, Duration::from_secs(30)));
    }
}
