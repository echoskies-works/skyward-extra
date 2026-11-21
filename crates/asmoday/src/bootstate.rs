//! The Secure Boot bit the bootloader hands the kernel.
//!
//! One platform, one key, three answers. On a Qualcomm phone the panel reads
//! `androidboot.secureboot` out of bootconfig: `"1"` says the boot chain is
//! checked, `"0"` says it is not. Anything else — another platform, a missing
//! key, a value that is neither — is nothing to report, and no line is drawn.
//!
//! This is what the bootloader declares, not what the fuse in QFPROM holds.
//! Reading the fuse itself needs the bootloader, and Asmoday runs after it:
//! the two agree on the phones checked so far (onyx, SM8735: `fastboot getvar
//! secure` says `yes`, this key says `1`, while AVB at the same time says
//! `unlocked` / `verifiedbootstate=orange`), but the panel repeats the
//! declaration rather than claiming to have seen the fuse.
//!
//! Nothing here guesses. Only this exact key, on this exact platform, with
//! one of these two exact values counts. A new spelling or a different
//! platform needs its own real-phone check before it joins.

use std::fs;
use std::sync::OnceLock;

/// What the panel is allowed to report.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SecureBoot {
    /// Nothing to say: not a Qualcomm phone, the key is missing, its value is
    /// neither of the two, or the files could not be read. A phone that did
    /// not answer is not a phone with the fuse unset.
    Unknown,
    /// `androidboot.hardware = "qcom"` and `androidboot.secureboot = "0"`.
    Off,
    /// `androidboot.hardware = "qcom"` and `androidboot.secureboot = "1"`.
    On,
}

/// Find one key in a kernel file, in either of the two grammars they use.
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    // cmdline: whitespace-separated `key=value` tokens on one line.
    for tok in text.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=')
            && k == key
        {
            return Some(v.trim().trim_matches('"').trim_end_matches(','));
        }
    }
    // bootconfig: one `key = "value"` per line.
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().trim_matches('"').trim_end_matches(','));
        }
    }
    None
}

/// One key, bootconfig first and the cmdline behind it. The two files carry
/// the same boot parameters — bootconfig is the newer spelling of it — so a
/// key found in either is the bootloader's own word.
fn value_in<'a>(bootconfig: Option<&'a str>, cmdline: Option<&'a str>, key: &str) -> Option<&'a str> {
    bootconfig
        .and_then(|text| value_of(text, key))
        .or_else(|| cmdline.and_then(|text| value_of(text, key)))
}

/// Read the two keys out of the kernel's own files. Pure, so a desk build
/// checks the grammar and the decision by supplying the text, not by
/// pretending to have a phone with the fuse set.
fn parse(bootconfig: Option<&str>, cmdline: Option<&str>) -> SecureBoot {
    // The platform decides whether the bit means anything at all: a phone
    // that is not Qualcomm is not read, even if it carries the key.
    if value_in(bootconfig, cmdline, "androidboot.hardware") != Some("qcom") {
        return SecureBoot::Unknown;
    }
    match value_in(bootconfig, cmdline, "androidboot.secureboot") {
        Some("1") => SecureBoot::On,
        Some("0") => SecureBoot::Off,
        _ => SecureBoot::Unknown,
    }
}

/// Read the state where the menu will actually run, once.
///
/// Asmoday is root in recovery, so these files are readable here even on
/// builds where Android's own shell is kept away from them. The menu is
/// painted again on every key, and neither file can change while it is up,
/// so the answer is kept the first time it is asked for.
///
/// A file that is missing or unreadable is a phone that reports nothing, not
/// a phone with the fuse unset — either way, no line is drawn.
pub(crate) fn secure_boot() -> SecureBoot {
    static READ: OnceLock<SecureBoot> = OnceLock::new();
    *READ.get_or_init(|| {
        let bootconfig = fs::read_to_string("/proc/bootconfig").ok();
        let cmdline = fs::read_to_string("/proc/cmdline").ok();
        let state = parse(bootconfig.as_deref(), cmdline.as_deref());
        // Recovery's stderr lands in the kernel log, which is where a phone
        // that draws the wrong line is looked into.
        eprintln!("asmoday: secure boot: {state:?}");
        state
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The onyx reading, trimmed to the keys that matter. AVB is unlocked;
    /// the declared bit is one. That contrast is why this is worth reading
    /// beside the About page's AVB state rather than instead of it.
    const ONYX_BOOTCONFIG: &str = r#"
androidboot.hardware = "qcom"
androidboot.hardware.sku = "onyx"
androidboot.secureboot = "1"
androidboot.verifiedbootstate = "orange"
androidboot.vbmeta.device_state = "unlocked"
androidboot.slot_suffix = "_b"
"#;

    #[test]
    fn a_qualcomm_phone_that_declares_one_is_on() {
        assert_eq!(parse(Some(ONYX_BOOTCONFIG), None), SecureBoot::On);
    }

    #[test]
    fn a_qualcomm_phone_that_declares_zero_is_off() {
        let bc = "androidboot.hardware = \"qcom\"\nandroidboot.secureboot = \"0\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Off);
    }

    #[test]
    fn the_cmdline_is_read_too() {
        let cmdline = "console=ttynull androidboot.hardware=qcom androidboot.secureboot=1 ro=1";
        assert_eq!(parse(None, Some(cmdline)), SecureBoot::On);
        let cmdline = "androidboot.hardware=qcom androidboot.secureboot=0";
        assert_eq!(parse(None, Some(cmdline)), SecureBoot::Off);
    }

    #[test]
    fn bootconfig_speaks_before_the_cmdline_does() {
        let bc = "androidboot.hardware = \"qcom\"\nandroidboot.secureboot = \"0\"\n";
        let cmdline = "androidboot.hardware=qcom androidboot.secureboot=1";
        assert_eq!(parse(Some(bc), Some(cmdline)), SecureBoot::Off, "the newer file wins");
    }

    #[test]
    fn another_platform_is_not_read() {
        let bc = "androidboot.hardware = \"mtk\"\nandroidboot.secureboot = \"1\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Unknown, "a MediaTek key is not a Qualcomm answer");
        let bc = "androidboot.hardware = \"mtk\"\nandroidboot.secureboot = \"0\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Unknown, "nor does an unset one say anything");
    }

    #[test]
    fn a_missing_key_reports_nothing_even_on_qualcomm() {
        let bc = "androidboot.hardware = \"qcom\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Unknown);
    }

    #[test]
    fn a_value_that_is_neither_one_nor_zero_reports_nothing() {
        for bit in ["2", "true", "yes", "", "01"] {
            let bc = format!("androidboot.hardware = \"qcom\"\nandroidboot.secureboot = \"{bit}\"\n");
            assert_eq!(parse(Some(&bc), None), SecureBoot::Unknown, "a bit of {bit:?} is not a reading");
        }
    }

    #[test]
    fn nothing_read_means_nothing_reported() {
        assert_eq!(parse(None, None), SecureBoot::Unknown);
        assert_eq!(parse(Some(""), Some("")), SecureBoot::Unknown);
    }

    #[test]
    fn a_longer_key_that_starts_with_this_one_is_not_it() {
        // The sku line sits right beside the one that is read: matching on a
        // prefix would take `onyx` as the platform and stop reading.
        let bc = "androidboot.hardware = \"qcom\"\nandroidboot.hardware.sku = \"onyx\"\nandroidboot.secureboot = \"1\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::On);
        let bc = "androidboot.hardware = \"mtk\"\nandroidboot.hardware.sku = \"qcom\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Unknown, "the sku is not the platform");
        let bc = "androidboot.hardware = \"qcom\"\nandroidboot.secureboot.enabled = \"1\"\n";
        assert_eq!(parse(Some(bc), None), SecureBoot::Unknown, "nor is a longer secureboot key");
    }
}
