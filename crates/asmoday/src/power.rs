//! The power operation, in the one program that carries one out.
//!
//! The system never writes `sys.powerctl`: the write tears the phone down
//! around the caller, so a system that made it would leave no exit behind and
//! its log would end wherever the write happened to stop. It says where it
//! wants to go and exits instead, and this program — the one still running —
//! does what the word asks. Nothing else in the phone does this, so there is
//! one place to look when a phone goes somewhere it was not asked to.

use anyhow::Result;
use skyward_sys::Power;

/// Ask init to take the phone down and bring it back as `target` says. init
/// syncs and unmounts before it acts, which is why this is the property and
/// not a system call of our own.
pub fn run(target: Power) -> Result<()> {
    skyward_sys::set_prop("sys.powerctl", target.command())
}
