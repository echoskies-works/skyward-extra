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

use crate::ui::Backlight;
use anyhow::{anyhow, Result};
use skyward_startup::{Choice, Crash, Reason};
use skyward_sys::Power;
use std::path::{Path, PathBuf};

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

/// How a child ended, as far as its parent can tell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum End {
    /// It left of its own accord and asked for nothing: nothing to report.
    Clean,
    /// It stopped with this code, in this way, for this reason where there
    /// is one.
    Stopped { code: i32, crash: Crash, reason: Option<Reason> },
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
        let (code, crash, reason) = match wait(start()?) {
            End::Clean => return Ok(()),
            End::Stopped { code, crash, reason } => (code, crash, reason),
        };
        eprintln!("supervise: the system stopped, {crash:?} {code} {reason:?}");

        light_panel(&mut light);
        // The screen offers one key and one way on. It does not return until
        // that key is pressed.
        skyward_startup::show_crash(crash, code, reason)?;

        light_panel(&mut light);
        match skyward_startup::run()? {
            // Back into the system: the loop starts it again.
            Choice::System => {}
            Choice::PowerOff => return skyward_sys::power(Power::Off),
            // The bootloader's own fastboot, as the menu says.
            Choice::Fastboot => return skyward_sys::power(Power::Bootloader),
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
    // A note from a previous life would be read as this one's: an empty panel
    // would come up saying the browser is not built yet, having never been
    // asked for the browser.
    let _ = std::fs::remove_file(note());
    let _ = std::fs::remove_file(up());

    match unsafe { libc::fork() } {
        -1 => Err(anyhow!("start the system: {}", std::io::Error::last_os_error())),
        0 => {
            // This is Aether. Whatever it exits with is the whole of what its
            // parent will know about how it went.
            let code = match crate::recovery() {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("recovery: {e:#}");
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
fn wait(pid: libc::pid_t) -> End {
    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(pid, &mut status, 0) };
        if waited == pid {
            break;
        }
        // A signal arriving while waiting is the only reason to look again;
        // anything else means there is nothing left to wait for.
        if waited < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            eprintln!("supervise: waiting for the system: {}", std::io::Error::last_os_error());
            return End::Clean;
        }
    }

    // A signal is reported the way a shell reports it, so 139 is a
    // segmentation fault, 137 the system killing it for memory, 134 a failed
    // assert. Both cases go on the panel as they are: the number is the only
    // fact here that narrows anything down.
    let code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        0
    };
    if code == 0 {
        return End::Clean;
    }

    // A child that never got a frame out never had a screen: it failed to
    // start rather than stopping while it ran.
    let crash = if up().exists() { Crash::Runtime } else { Crash::Startup };
    let reason = std::fs::read_to_string(note()).ok().and_then(|tag| Reason::from_tag(&tag));
    End::Stopped { code, crash, reason }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_and_the_word_sit_in_the_recoverys_own_folder() {
        // Both files are read and written while the recovery owns the phone,
        // so they live where it keeps everything else: a folder install
        // scripts empty /tmp around does not touch.
        assert_eq!(note(), Path::new("/skyward/run/stop"));
        assert_eq!(up(), Path::new("/skyward/run/up"));
    }
}
