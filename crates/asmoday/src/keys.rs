//! The hardware buttons, and nothing else.
//!
//! No touch panel, no gestures, no power-key timing: the menu reacts to a
//! press and ignores how long it lasted. That is one of the things that
//! keeps this small enough to still work when the rest of the system does
//! not.

use super::menu::Key;
use evdev::raw_stream::RawDevice;
use evdev::{EventSummary, KeyCode};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::os::fd::{AsRawFd, RawFd};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The keys the menu listens for, and what they mean here.
const WANTED: [KeyCode; 3] = [KeyCode::KEY_VOLUMEUP, KeyCode::KEY_VOLUMEDOWN, KeyCode::KEY_POWER];

fn as_menu_key(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::KEY_VOLUMEUP => Some(Key::Up),
        KeyCode::KEY_VOLUMEDOWN => Some(Key::Down),
        KeyCode::KEY_POWER => Some(Key::Power),
        _ => None,
    }
}

fn set_nonblocking(fd: RawFd) -> std::io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Every device under /dev/input that carries a power or volume key.
pub struct Buttons {
    devices: Vec<(PathBuf, RawDevice)>,
    seen: HashSet<PathBuf>,
}

impl Buttons {
    /// Look for the buttons once. A device that is not there yet is picked
    /// up by [`Buttons::wait`], since ueventd may still be creating nodes
    /// when the first frame goes out.
    pub fn new() -> Self {
        let mut buttons = Buttons { devices: Vec::new(), seen: HashSet::new() };
        buttons.scan();
        buttons
    }

    /// Whether any button device has turned up.
    pub fn found(&self) -> bool {
        !self.devices.is_empty()
    }

    /// Take in devices not seen before. Called again on every wait timeout,
    /// which is cheap: /dev/input holds a handful of nodes.
    pub fn scan(&mut self) {
        for (path, dev) in evdev::raw_stream::enumerate() {
            if !self.seen.insert(path.clone()) {
                continue;
            }
            let carries = dev.supported_keys().is_some_and(|k| WANTED.iter().any(|c| k.contains(*c)));
            if !carries {
                continue;
            }
            if let Err(e) = set_nonblocking(dev.as_raw_fd()) {
                eprintln!("startup: {}: {e}", path.display());
                continue;
            }
            eprintln!("startup: buttons on {} ({})", path.display(), dev.name().unwrap_or("?"));
            self.devices.push((path, dev));
        }
    }

    /// Presses that have arrived since the last call, oldest first. Releases
    /// and the kernel's own key repeats are dropped: the menu acts once per
    /// press, and holding a key is not a gesture it knows.
    pub fn poll(&mut self) -> Vec<Key> {
        let mut out = Vec::new();
        for (path, dev) in &mut self.devices {
            match dev.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if let EventSummary::Key(_, code, 1) = event.destructure()
                            && let Some(key) = as_menu_key(code)
                        {
                            out.push(key);
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) if e.raw_os_error() == Some(libc::ENODEV) => {
                    eprintln!("startup: {}: removed", path.display());
                }
                Err(e) => eprintln!("startup: {}: {e}", path.display()),
            }
        }
        out
    }

    /// Block until a press arrives or `timeout` passes, then report what
    /// came in. Sleeps rather than using inotify: the menu is drawn once and
    /// then waits, so a missed hotplug is a device that is not there on the
    /// next scan either.
    pub fn wait(&mut self, timeout: Duration) -> Vec<Key> {
        let until = Instant::now() + timeout;
        loop {
            let keys = self.poll();
            if !keys.is_empty() {
                return keys;
            }
            let left = until.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Vec::new();
            }
            std::thread::sleep(left.min(Duration::from_millis(40)));
        }
    }
}

impl Default for Buttons {
    fn default() -> Self {
        Buttons::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_keys_are_mapped() {
        assert_eq!(as_menu_key(KeyCode::KEY_VOLUMEUP), Some(Key::Up));
        assert_eq!(as_menu_key(KeyCode::KEY_VOLUMEDOWN), Some(Key::Down));
        assert_eq!(as_menu_key(KeyCode::KEY_POWER), Some(Key::Power));
        assert_eq!(as_menu_key(KeyCode::KEY_BACK), None);
        assert_eq!(as_menu_key(KeyCode::KEY_HOME), None);
    }
}
