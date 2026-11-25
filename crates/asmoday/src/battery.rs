//! The charge the panel reports, read from the kernel.
//!
//! The panel is up before the system and its battery service exist, so there
//! is nothing to be told and nothing to ask: the only account of the charge
//! is the kernel's own, one directory per power supply, and the battery is
//! the one that calls itself a battery. `/sys` is mounted by then — init
//! mounts it before recovery starts — so this is a read of two small files,
//! not a service to wait for.
//!
//! A charger in the phone changes nothing here. The line says the battery is
//! low, and a phone on a cable at nine per cent is a phone that will not last
//! long once the cable comes out — which is what the line is for. The pages
//! know the charger's state as well and say the two things separately; the
//! panel runs where that reading is not worth the second file.

use std::fs;
use std::path::Path;

/// Where the kernel lists the phone's power supplies.
const SUPPLIES: &str = "/sys/class/power_supply";

/// The charge the panel begins saying the battery is low at, in per cent:
/// the same line the pages draw, so the two screens call the same battery
/// low at the same charge. Twenty itself counts — the card that says the
/// battery is running down comes at twenty, and a panel that stayed quiet
/// there would disagree with the system it is standing in front of.
pub(crate) const LOW_AT: u8 = 20;

/// The battery's charge, or nothing where there is no battery to read.
pub(crate) fn level() -> Option<u8> {
    // A desk build has no battery under it. The variable is the one way to
    // see the line drawn there — the same way `AETHER_LANG` is the one way
    // to see the panel in either language — and nothing on a phone sets it.
    if let Ok(text) = std::env::var("AETHER_BATTERY")
        && let Some(level) = parse(text.trim())
    {
        return Some(level);
    }
    of(Path::new(SUPPLIES))
}

/// The charge in the supplies listed under `root`, or nothing.
///
/// Split from [`level`] so a desk build can lay out a directory of its own
/// and check what the panel makes of it, rather than needing a phone.
fn of(root: &Path) -> Option<u8> {
    let mut supplies: Vec<_> = fs::read_dir(root).ok()?.flatten().map(|entry| entry.path()).collect();
    // Read in a settled order: a phone may list more than one supply, and
    // the answer must not change between two frames.
    supplies.sort();
    supplies.into_iter().find_map(|supply| {
        let kind = fs::read_to_string(supply.join("type")).ok()?;
        if kind.trim() != "Battery" {
            return None;
        }
        parse(fs::read_to_string(supply.join("capacity")).ok()?.trim())
    })
}

/// A charge as the kernel writes it: whole per cent, nought to a hundred.
///
/// Anything else is nothing to report rather than a guess. A reading of a
/// hundred and one is a driver writing something other than a percentage,
/// and a panel that clamped it would say the battery is full when it has no
/// idea.
fn parse(text: &str) -> Option<u8> {
    text.trim().parse().ok().filter(|level| *level <= 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A directory of made-up supplies, removed when the test is done.
    struct Supplies(PathBuf);

    impl Supplies {
        fn new(name: &str) -> Supplies {
            let root = std::env::temp_dir().join(format!("asmoday-battery-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("a directory to lay out");
            Supplies(root)
        }

        /// One supply, of `kind`, at `capacity`; no capacity where it is
        /// nothing.
        fn supply(&self, name: &str, kind: &str, capacity: Option<&str>) -> &Self {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).expect("a supply");
            fs::write(dir.join("type"), format!("{kind}\n")).expect("its type");
            if let Some(capacity) = capacity {
                fs::write(dir.join("capacity"), format!("{capacity}\n")).expect("its charge");
            }
            self
        }
    }

    impl Drop for Supplies {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_battery_is_the_supply_that_calls_itself_one() {
        let root = Supplies::new("battery");
        // The charger and the wireless receiver are supplies too, and
        // neither of them has a charge to report.
        root.supply("main", "Battery", Some("43")).supply("usb", "Mains", None).supply("qc", "USB", None);
        assert_eq!(of(&root.0), Some(43));
    }

    #[test]
    fn a_supply_that_is_not_the_battery_is_not_read() {
        let root = Supplies::new("mains");
        root.supply("usb", "Mains", Some("90"));
        assert_eq!(of(&root.0), None, "a charger's number is not the battery's");
    }

    #[test]
    fn nothing_to_read_is_nothing_reported() {
        let empty = Supplies::new("empty");
        assert_eq!(of(&empty.0), None, "no supplies at all");
        let bare = Supplies::new("bare");
        bare.supply("battery", "Battery", None);
        assert_eq!(of(&bare.0), None, "a battery with no capacity file");
        let missing = std::env::temp_dir().join("asmoday-battery-no-such-place");
        assert_eq!(of(&missing), None, "a path that is not there");
    }

    #[test]
    fn a_number_that_is_not_a_charge_is_nothing_reported() {
        for text in ["", "full", "-1", "101", "1000", "43.5"] {
            assert_eq!(parse(text), None, "{text:?} is not a charge");
        }
        for (text, level) in [("0", 0), ("20", 20), ("100", 100), (" 43 ", 43)] {
            assert_eq!(parse(text), Some(level), "{text:?}");
        }
    }

    #[test]
    fn the_file_is_read_with_the_newline_the_kernel_writes() {
        let root = Supplies::new("newline");
        root.supply("battery", "Battery", Some("7\n"));
        assert_eq!(of(&root.0), Some(7), "a trailing newline is not part of the number");
    }
}
