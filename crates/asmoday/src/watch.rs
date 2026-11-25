//! The process that outlives the system: it starts the system, waits for it
//! to stop, and says on the panel how it went.
//!
//! One process cannot report on itself. A system that has fallen over has no
//! code left to run, and the panel it would have drawn on is the thing that
//! stops with it — so the phone runs two: this one forks, the child becomes
//! the system by exec'ing a separate binary, and the parent does nothing but
//! wait. When the child is gone the parent has the panel to itself and says
//! the one thing that can still be said, which is that the system is not
//! running and, where it is known, why.
//!
//! The startup menu is therefore reached from the failure screen and from
//! nowhere else: the menu *is* the parent, and the parent is only in charge
//! while there is no child to be in charge of.
//!
//! A system can also stop without dying. A driver that never answers, a lock
//! nobody releases, a screen that stays black while the process behind it sits
//! there — none of that leaves an exit code, because nothing exits. What it
//! leaves is silence, and silence is what the parent watches for: the child
//! writes the time to a file while it runs, and a file that has not moved for
//! a few seconds is a system that is not going to move again.
//!
//! Silence is asked twice, because a file can stop moving for a reason that is
//! not the system's death: it lives on the same memory filesystem an update
//! unpacks itself into, and a full or read-only one breaks the file while the
//! process goes on working. So the second question goes to the kernel — has it
//! spent any processor time, has it moved any bytes — and only a system that
//! has answered neither is stopped. See [`crate::liveness`].

use crate::backlight::Backlight;
use crate::liveness::Still;
use crate::verdict::{self, End, KILLED};
use crate::{Choice, Crash};
use anyhow::{anyhow, Result};
use skyward_sys::Power;
use std::io::ErrorKind;
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The system is a separate binary. The parent exec's this in its child so a
/// change to the system relinks nothing here: this ELF can stay byte-for-byte
/// the same through any number of system rebuilds.
const SYSTEM: &str = "/system/bin/skyward";

/// How often the parent looks at a child that is still there.
const LOOK: Duration = Duration::from_millis(100);

/// The time the child is given to take a SIGTERM before it is killed.
///
/// Only a system that has already failed both questions is sent this, so the
/// grace is not a wait for a slow system to finish its work. Nothing in the
/// system handles SIGTERM today, which means in practice the process is gone
/// the moment it arrives; the grace is kept for the day something does handle
/// it, and for a kernel that takes a while to deliver to a process that is not
/// coming back. Five seconds is long enough to be worth trying and short
/// enough that nobody waits for it.
const GRACE: Duration = Duration::from_secs(5);

/// Where a running system says that it got a frame onto the panel.
///
/// The two failure screens differ by one line — never started, or stopped
/// while running — and this is what tells them apart. The system writes these
/// three files under the same path; keeping the path here, as well as on the
/// writing side, is what makes the agreement checkable.
pub(crate) fn up() -> PathBuf {
    Path::new(skyward_sys::RUN).join("up")
}

/// Where a stopped system leaves the word for the app it stopped for.
pub(crate) fn note() -> PathBuf {
    Path::new(skyward_sys::RUN).join("stop")
}

/// Where a running system says it is still running.
fn beat_file() -> PathBuf {
    Path::new(skyward_sys::RUN).join("beat")
}

/// When the system last said so, in the clock the system writes.
fn last_beat() -> Option<u64> {
    std::fs::read_to_string(beat_file()).ok()?.trim().parse().ok()
}

/// What the child left in its note, where it left one.
fn note_left() -> Option<String> {
    std::fs::read_to_string(note()).ok()
}

/// Milliseconds on the monotonic clock, matching what the system stamps.
fn monotonic_ms() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    let rc = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    assert_eq!(rc, 0, "read the monotonic clock");
    ts.tv_sec as u64 * 1000 + ts.tv_nsec as u64 / 1_000_000
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
        let (pid, note) = start()?;
        let ended = wait(pid, forked);
        // A system that left deliberately says where it wants to go. Reading
        // the note comes before judging the exit: the note makes this an
        // ordinary departure whatever the code says, and the power operation
        // belongs here, in the process that is still alive to carry it out.
        // A folder that would not take the note left the same word down the
        // pipe instead, so both places are asked before the exit is believed.
        // Anything the system could not say either way is an ordinary stop.
        let asked = skyward_sys::leave::take().or_else(|| note.and_then(pipe_note));
        if let Some(target) = asked {
            let _ = skyward_sys::kmsg(&format!("the system left for {}; carrying it out", skyward_sys::leave::name(target)));
            return crate::power::run(target);
        }
        let (code, crash, reason) = match ended {
            End::Clean | End::Asked => return Ok(()),
            End::Stopped { code, crash, reason } => (code, crash, reason),
        };
        eprintln!("asmoday: the system stopped, {crash:?} {code} {reason:?}");
        // Taken before the panel is touched: even if the failure screen
        // cannot come up, the line about where it died is already in the
        // kernel log. The exit code says how, this says where.
        if let Some(line) = crate::crashlog::read() {
            let said = format!("the system died here: {line}");
            eprintln!("asmoday: {said}");
            let _ = skyward_sys::kmsg(&said);
        }

        light_panel(&mut light);
        // The screen offers one key and one way on. It does not return until
        // that key is pressed. A screen that will not come up is worth a line,
        // not the end: the menu below is still worth trying, and leaving would
        // only hand a black panel to whatever restarts this.
        if let Err(e) = crate::show_crash(crash, code, reason) {
            eprintln!("asmoday: the failure screen would not come up: {e:#}");
            let _ = skyward_sys::kmsg(&format!("the failure screen would not come up: {e:#}"));
        }

        light_panel(&mut light);
        match crate::run() {
            // Back into the system: the loop starts it again.
            Ok(Choice::System) => {}
            Ok(Choice::PowerOff) => return crate::power::run(Power::Off),
            // The bootloader's own fastboot, as the menu says.
            Ok(Choice::Fastboot) => return crate::power::run(Power::Bootloader),
            // Neither screen would come up. Leaving lets init have the service
            // back; a loop here would spin on a panel nobody can see.
            Err(e) => {
                eprintln!("asmoday: the startup menu would not come up: {e:#}");
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
        eprintln!("asmoday: the panel would not light: {e:#}");
    }
}

/// Fork: the child becomes the system by exec'ing its own binary, and the
/// parent becomes the one waiting for it.
///
/// The fork is taken before anything here has started a thread, and the child
/// does one thing only — run that system and leave its exit code behind — so
/// nothing of the parent's state does any work in it. exec, rather than a call
/// into this program, is the whole point: the two binaries stay independent.
fn start() -> Result<(libc::pid_t, Option<OwnedFd>)> {
    // The system makes this folder when it comes up, which is later than the
    // first beat below. Without it that beat is written into nothing, and the
    // parent would go back to counting silence from the fork — the one number
    // this window exists to stop it counting.
    let _ = std::fs::create_dir_all(skyward_sys::RUN);
    // A note or a beat from a previous life would be read as this one's: an
    // empty panel would come up saying the browser is not built yet, having
    // never been asked for the browser, and a stale beat would make a system
    // that never started look alive.
    let _ = std::fs::remove_file(note());
    let _ = std::fs::remove_file(up());
    let _ = std::fs::remove_file(beat_file());
    // A panic note from a previous life would be read as this one's: an
    // ordinary stop would come up claiming to know which line panicked.
    let _ = std::fs::remove_file(crate::crashlog::panic_file());
    // A leave note from a previous life would turn an ordinary stop into a
    // power operation: this system would reboot the phone when it only
    // stopped. The system clears it too on the way in; clearing it here as
    // well covers a system that never got that far.
    skyward_sys::leave::clear();

    // A note the folder will not take goes down this pipe instead. It is
    // opened before the fork so the child inherits the writing end, and the
    // reading end comes back with the pid. A pipe that will not open costs
    // the fallback, not the start: the note still has its folder.
    let mut ends = [-1; 2];
    let piped = unsafe { libc::pipe(ends.as_mut_ptr()) } == 0;
    if piped {
        // The parent reads once, after the child is gone, and must not sit
        // waiting on a word that was never written.
        unsafe { libc::fcntl(ends[0], libc::F_SETFL, libc::O_NONBLOCK) };
    }

    match unsafe { libc::fork() } {
        -1 => {
            if piped {
                unsafe { libc::close(ends[0]) };
                unsafe { libc::close(ends[1]) };
            }
            Err(anyhow!("start the system: {}", std::io::Error::last_os_error()))
        }
        0 => {
            let note = if piped {
                // The child never reads: only the parent has something to
                // hear, and a reading end left open here would keep the
                // parent from seeing the pipe close.
                unsafe { libc::close(ends[0]) };
                Some(ends[1])
            } else {
                None
            };
            // The first beat, before the exec: from here the parent knows the
            // system is alive and watching is about silence rather than about
            // a fork that may not have happened. The system keeps writing the
            // same file after it takes over.
            let _ = std::fs::write(beat_file(), monotonic_ms().to_string());
            // exec the system. On success this never returns: the child's
            // image, including every byte of this program, is replaced.
            let error = exec(SYSTEM, note);
            // A system that could not be exec'd has nothing to report but the
            // code and the reason in the kernel log.
            eprintln!("asmoday: exec {SYSTEM}: {error:?}");
            let _ = skyward_sys::kmsg(&format!("could not start the system: {error:?}"));
            std::process::exit(1)
        }
        pid => {
            if piped {
                // The parent writes nothing: a writing end left open here
                // would keep it from seeing the pipe close.
                unsafe { libc::close(ends[1]) };
                // SAFETY: the descriptor is open here, is not held by
                // anything else in this process, and is closed by the value
                // once the note has been read.
                Ok((pid, Some(unsafe { OwnedFd::from_raw_fd(ends[0]) })))
            } else {
                Ok((pid, None))
            }
        }
    }
}

/// Replace this process with the system binary, keeping its arguments empty
/// and its environment as it is but for one name: the pipe's writing end,
/// where there is one, so the system can say where it wants to go even when
/// the folder cannot take the note.
#[cfg(target_os = "android")]
fn exec(path: &str, note: Option<libc::c_int>) -> Result<()> {
    use std::ffi::CString;

    let path = CString::new(path).map_err(|e| anyhow!("system path has a nul: {e}"))?;
    // The child runs a single thread, so writing the environment here is safe,
    // and execv keeps what is written: this is the whole hand-over of the
    // pipe, and it needs no argument the system would read as a command.
    if let Some(fd) = note
        && let (Ok(name), Ok(value)) = (
            CString::new(skyward_sys::leave::NOTE_FD),
            CString::new(fd.to_string()),
        )
    {
        unsafe { libc::setenv(name.as_ptr(), value.as_ptr(), 1) };
    }
    // The array ends at the first null pointer. Putting an empty string in
    // that place would not be "no arguments": it would be one argument, an
    // empty one, and the system reads everything after its own name as a
    // command — `skyward ""` asks it for a subcommand called nothing. The
    // system is started with its name and no words after it.
    let argv = [path.as_ptr(), std::ptr::null()];
    // SAFETY: `path` and `argv` outlive the call and are nul-terminated. On
    // success execv does not return; on failure the errno says why.
    unsafe { libc::execv(path.as_ptr(), argv.as_ptr()) };
    Err(anyhow!("execv: {}", std::io::Error::last_os_error()))
}

/// What the child wrote down the pipe, if anything: the same word the folder
/// would have held. An empty pipe, or a word that is not a target, is no
/// request — an ordinary stop, as with the note.
#[cfg(target_os = "android")]
fn pipe_note(fd: OwnedFd) -> Option<Power> {
    use std::os::fd::AsRawFd;

    let mut buf = [0u8; 32];
    let read = unsafe { libc::read(fd.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) };
    if read <= 0 {
        return None;
    }
    Power::parse(std::str::from_utf8(&buf[..read as usize]).ok()?.trim())
}

/// Wait for the child to stop, and make what it left into something a panel
/// can say.
fn wait(pid: libc::pid_t, forked: Instant) -> End {
    let mut status = 0;
    let mut still = Still::default();
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
            eprintln!("asmoday: waiting for the system: {}", std::io::Error::last_os_error());
            return End::Clean;
        }
        // It is still running. Silence is the first question and movement the
        // second: a system that cannot write its beat is not a system that has
        // stopped, and stopping one that is still working is the failure this
        // parent exists to prevent. Only a system that has done neither for
        // long enough is believed gone.
        match silent(forked) {
            Some(crash) => {
                if still.believes(pid, Instant::now()) {
                    eprintln!(
                        "asmoday: the system has said nothing for {} s and moved nothing for {} s",
                        verdict::SILENCE.as_secs(),
                        crate::liveness::STILL.as_secs()
                    );
                    return stuck(pid, &mut status, crash);
                }
            }
            // It beat again: whatever quiet there was is over, and the next
            // one is counted from nothing.
            None => still.forget(),
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
/// Only called once the system has failed both questions: it stopped writing
/// its beat, and the kernel agrees it has spent nothing since. A system that
/// is merely slow is not here at all.
///
/// The signal is asked for before it is forced, because a signal a program
/// handles is a chance to put the display down and leave; this one handles
/// none today, so in practice the first signal is already the last. That is
/// why the second is still sent after the grace: it costs nothing to try, and
/// a system that has hung in the kernel never hears either.
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
        // A process that ignored SIGTERM for five seconds may be waiting on a
        // disk that will never answer, and one in that state takes no signal
        // at all — not this one either. Waiting for it without a limit would
        // take the panel down with it, which is the one thing the parent is
        // here to prevent, so this wait has a limit too. A process that
        // outlasts it is left where it is rather than waited on for ever.
        let killed_at = Instant::now();
        while killed_at.elapsed() < GRACE {
            if unsafe { libc::waitpid(pid, status, libc::WNOHANG) } == pid {
                break;
            }
            std::thread::sleep(LOOK);
        }
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
        // empty /tmp around does not touch. These are the paths the system
        // writes too; both sides must agree.
        assert_eq!(note(), Path::new("/skyward/run/stop"));
        assert_eq!(up(), Path::new("/skyward/run/up"));
        assert_eq!(beat_file(), Path::new("/skyward/run/beat"));
    }
}
