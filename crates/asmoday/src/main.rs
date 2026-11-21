//! The recovery service and the startup menu, one program under two names.
//!
//! init starts this as `system/bin/recovery`: it starts the system in a child
//! and owns the panel while there is no system. The menu is also reachable on
//! its own — by a link called `startup`, which is how the panel gets checked
//! from a shell, or by `--menu` after whatever this binary is called, which is
//! how it gets checked without making a link first. The system itself is a
//! separate binary, `/system/bin/skyward`, so a rebuild of the system relinks
//! nothing here.

use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    let mut words = std::env::args_os();
    let started_as = words.next().unwrap_or_default();
    let name = Path::new(&started_as).file_name().unwrap_or_default().to_string_lossy().into_owned();
    let args: Vec<String> = words.map(|word| word.to_string_lossy().into_owned()).collect();
    let job = asmoday::job_of(&name, &args)?;

    #[cfg(target_os = "android")]
    match job {
        // What init starts: watch the system, and be the menu when it stops.
        asmoday::Job::Watch => asmoday::watch(),
        asmoday::Job::Menu => match asmoday::run()? {
            asmoday::Choice::System => asmoday::watch(),
            asmoday::Choice::PowerOff => skyward_sys::power(skyward_sys::Power::Off),
            asmoday::Choice::Fastboot => skyward_sys::power(skyward_sys::Power::Bootloader),
        },
    }

    #[cfg(not(target_os = "android"))]
    {
        eprintln!("asmoday: the recovery service runs only on Android (started as {name}, {job:?})");
        Ok(())
    }
}
