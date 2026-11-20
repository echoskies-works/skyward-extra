//! The process that outlives the system: it starts Aether, waits for it to
//! stop, and says on the panel how it went.
//!
//! One process cannot report on itself. A system that has fallen over has no
//! code left to run, and the panel it would have drawn on is the thing that
//! stops with it — so the phone runs two: the recovery service forks, the
//! child is Aether, and the parent does nothing but wait. When the child is
//! gone the parent has the panel to itself and says the one thing that can
//! still be said, which is that the system is not running and, where it is
//! known, why.
//!
//! The startup menu is therefore reached from the failure screen and from
//! nowhere else: the menu *is* the parent, and the parent is only in charge
//! while there is no child to be in charge of.
//!
//! A system can also stop without dying. A driver that never answers, a lock
//! nobody releases, a screen that stays black while the process behind it sits
//! there — none of that leaves an exit code, because nothing exits. What it
//! leaves is silence, and silence is what the parent watches for: the child
//! writes the time to a file while it runs, and a file that has not moved in
//! half a minute is a system that is not going to move again.
//!
//! What the numbers mean is [`crate::verdict`]'s to say, not this module's:
//! the rule is the same whether or not a phone is there to apply it.

use crate::input::monotonic_ms;
use crate::ui::Backlight;
use crate::verdict::{self, End, KILLED};
use anyhow::{anyhow, Result};
use asmoday::{Choice, Crash};
use skyward_sys::Power;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long the child waits between two beats. A second wastes nothing on a
/// memory filesystem and leaves thirty chances before silence is believed.
pub(crate) const BEAT_EVERY: u64 = 1000;

/// The time the child is given to take a SIGTERM before it is killed.
///
/// A stopped system is stopped, but a system that is merely slow will act on
/// the signal: it closes the display and puts the port down, and the parent
/// gets the panel back without a fight. Five seconds is long enough to be
/// worth trying and short enough that nobody waits for it.
const GRACE: Duration = Duration::from_secs(5);

/// How often the parent looks at a child that is still there.
const LOOK: Duration = Duration::from_millis(100);

/// Where a running Aether says that it got a frame onto the panel.
///
/// The two failure screens differ by one line — never started, or stopped
/// while running — and this is what tells them apart: a child that dies
/// without ever writing it never had a screen up.
pub(crate) fn up() -> PathBuf {
    Path::new(skyward_sys::RUN).join("up")
}

/// Where a stopped Aether leaves the word for the app it stopped for.
///
/// The exit code says what kind of stop it was and nothing more: 67 is one
/// code for every app that is not built yet, so which app it was has to
/// travel beside the code rather than in it.
pub(crate) fn note() -> PathBuf {
    Path::new(skyward_sys::RUN).join("stop")
}

/// Where a running Aether says it is still running.
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

/// When the system last said so, in the clock [`beat`] writes.
fn last_beat() -> Option<u64> {
    std::fs::read_to_string(beat_file()).ok()?.trim().parse().ok()
}

/// What the child left in its note, where it left one.
fn note_left() -> Option<String> {
    std::fs::read_to_string(note()).ok()
}

/// Start the system, watch it, and report on it for as long as the phone is
/// in recovery.
pub fn run() -> Result<()> {
    // The panel is lit through both screens below. A system that stopped may
    // have left it dark, and a failure nobody can see is no report at all —
    // so the backlight is taken here and lit before either screen goes up. A
    // panel that will not take a level is not fatal: the screens come up
    // anyway, and one of them may be readable in daylight.
    let mut light = Backlight::open().ok();

    loop {
        let forked = Instant::now();
        let (code, crash, reason) = match wait(start()?, forked) {
            End::Clean | End::Asked => return Ok(()),
            End::Stopped { code, crash, reason } => (code, crash, reason),
        };
        eprintln!("supervise: the system stopped, {crash:?} {code} {reason:?}");

        light_panel(&mut light);
        // The screen offers one key and one way on. It does not return until
        // that key is pressed. A screen that will not come up is worth a line,
        // not the end: the menu below is still worth trying, and leaving would
        // only hand a black panel to whatever restarts this.
        if let Err(e) = asmoday::show_crash(crash, code, reason) {
            eprintln!("supervise: the failure screen would not come up: {e:#}");
            let _ = skyward_sys::kmsg(&format!("the failure screen would not come up: {e:#}"));
        }

        light_panel(&mut light);
        match asmoday::run() {
            // Back into the system: the loop starts it again.
            Ok(Choice::System) => {}
            Ok(Choice::PowerOff) => return skyward_sys::power(Power::Off),
            // The bootloader's own fastboot, as the menu says.
            Ok(Choice::Fastboot) => return skyward_sys::power(Power::Bootloader),
            // Neither screen would come up. Leaving lets init have the service
            // back; a loop here would spin on a panel nobody can see.
            Err(e) => {
                eprintln!("supervise: the startup menu would not come up: {e:#}");
                let _ = skyward_sys::kmsg(&format!("the startup menu would not come up: {e:#}"));
                return Ok(());
            }
        }
    }
}

/// Light the panel at its normal maximum, as a boot menu is lit.
fn light_panel(light: &mut Option<Backlight>) {
    let Some(light) = light.as_mut() else { return };
    if let Err(e) = light.show(Some(1.0)) {
        eprintln!("supervise: the panel would not light: {e:#}");
    }
}

/// Fork: the child becomes the system, the parent becomes the one waiting
/// for it.
///
/// The fork is taken before anything here has started a thread, and the child
/// does one thing only — run that system and leave its exit code behind — so
/// nothing of the parent's state does any work in it.
fn start() -> Result<libc::pid_t> {
    // A note or a beat from a previous life would be read as this one's: an
    // empty panel would come up saying the browser is not built yet, having
    // never been asked for the browser, and a stale beat would make a system
    // that never started look alive.
    let _ = std::fs::remove_file(note());
    let _ = std::fs::remove_file(up());
    let _ = std::fs::remove_file(beat_file());

    match unsafe { libc::fork() } {
        -1 => Err(anyhow!("start the system: {}", std::io::Error::last_os_error())),
        0 => {
            // The first beat, before anything else: from here the parent knows
            // the system is alive and watching is about silence rather than
            // about a fork that may not have happened.
            let _ = beat();
            // This is Aether. Whatever it exits with is the whole of what its
            // parent will know about how it went.
            let code = match crate::recovery() {
                Ok(()) => 0,
                Err(e) => {
                    // The panel will say 1 and nothing more — 1 is what a
                    // system that could not come up is reported as. The reason
                    // goes where a reason can still be read afterwards, which
                    // is the kernel log.
                    eprintln!("recovery: {e:#}");
                    let _ = skyward_sys::kmsg(&format!("the system could not start: {e:#}"));
                    1
                }
            };
            std::process::exit(code)
        }
        pid => Ok(pid),
    }
}

/// Wait for the child to stop, and make what it left into something a panel
/// can say.
fn wait(pid: libc::pid_t, forked: Instant) -> End {
    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if waited == pid {
            return verdict::ended(status, up().exists(), note_left().as_deref());
        }
        if waited < 0 {
            // A signal arriving while waiting is the only reason to look
            // again; anything else means there is nothing left to wait for.
            if std::io::Error::last_os_error().kind() == ErrorKind::Interrupted {
                continue;
            }
            eprintln!("supervise: waiting for the system: {}", std::io::Error::last_os_error());
            return End::Clean;
        }
        // It is still running. Whether it is still there is another question,
        // and the only one that silence can answer.
        if let Some(crash) = silent(forked) {
            eprintln!("supervise: nothing from the system for {} s", verdict::SILENCE.as_secs());
            return stuck(pid, &mut status, crash);
        }
        std::thread::sleep(LOOK);
    }
}

/// Which failure it is, where the system has gone quiet.
fn silent(forked: Instant) -> Option<Crash> {
    if !verdict::overdue(last_beat(), monotonic_ms(), forked.elapsed()) {
        return None;
    }
    // A child that never got a frame out never had a screen: it failed to
    // start rather than stopping while it ran.
    Some(if up().exists() { Crash::Runtime } else { Crash::Startup })
}

/// Stop a system that has gone quiet, and report it as stopped.
///
/// The signal is asked for before it is forced. A system that is only slow
/// still takes it and closes the display on its way out, which is what lets
/// the parent open the panel again; one that is truly stopped never hears
/// either signal and is killed.
fn stuck(pid: libc::pid_t, status: &mut i32, crash: Crash) -> End {
    unsafe { libc::kill(pid, libc::SIGTERM) };
    let asked_at = Instant::now();
    let mut gone = false;
    while asked_at.elapsed() < GRACE {
        let waited = unsafe { libc::waitpid(pid, status, libc::WNOHANG) };
        if waited == pid {
            gone = true;
            break;
        }
        if waited < 0 && std::io::Error::last_os_error().kind() != ErrorKind::Interrupted {
            gone = true;
            break;
        }
        std::thread::sleep(LOOK);
    }
    if !gone {
        unsafe { libc::kill(pid, libc::SIGKILL) };
        unsafe { libc::waitpid(pid, status, 0) };
    }
    End::Stopped { code: KILLED, crash, reason: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_the_word_and_the_beat_sit_in_the_recoverys_own_folder() {
        // All three are read and written while the recovery owns the phone, so
        // they live where it keeps everything else: a folder install scripts
        // empty /tmp around does not touch.
        assert_eq!(note(), Path::new("/skyward/run/stop"));
        assert_eq!(up(), Path::new("/skyward/run/up"));
        assert_eq!(beat_file(), Path::new("/skyward/run/beat"));
    }
}
