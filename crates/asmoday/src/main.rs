//! The recovery service and the startup menu, one program under two names.
//!
//! init starts this as `system/bin/recovery`: it starts the system in a child
//! and owns the panel while there is no system. Under the name `startup` it
//! shows the menu on its own, which is how the panel gets checked from a
//! shell. The system itself is a separate binary, `/system/bin/skyward`, so
//! a rebuild of the system relinks nothing here.

use anyhow::Result;
#[cfg(target_os = "android")]
use anyhow::bail;
use std::path::Path;

fn main() -> Result<()> {
    let started_as = std::env::args_os().next().unwrap_or_default();
    let name = Path::new(&started_as).file_name().unwrap_or_default().to_string_lossy();

    #[cfg(target_os = "android")]
    match name.as_ref() {
        // What init starts. Watch the system and own the panel when it stops.
        "recovery" => asmoday::watch(),
        // A shell's way in while nothing else starts the menu yet.
        "startup" => {
            match asmoday::run()? {
                asmoday::Choice::System => asmoday::watch(),
                asmoday::Choice::PowerOff => skyward_sys::power(skyward_sys::Power::Off),
                asmoday::Choice::Fastboot => skyward_sys::power(skyward_sys::Power::Bootloader),
            }
        }
        other => bail!("asmoday: unknown command {other}"),
    }

    #[cfg(not(target_os = "android"))]
    {
        eprintln!("asmoday: the recovery service runs only on Android (started as {name})");
        Ok(())
    }
}
