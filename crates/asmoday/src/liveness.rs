//! Whether the system is still moving.
//!
//! A beat is a promise the pages make every second, and a promise a full
//! filesystem cannot keep is not the same thing as a process that stopped.
//! The beat is a file on the recovery's own memory filesystem: an update that
//! unpacks itself there, a root that went read only, a write that took the
//! bytes and never answered — any of them can break the beat while the process
//! behind it goes on working. Acting on that silence kills the one thing the
//! watcher exists to keep.
//!
//! So silence is a question and not a verdict. The watcher asks the kernel
//! instead, which needs nothing from the system: how much processor time it
//! has been charged, how many bytes it has moved, and what it is waiting on.
//! A process still spending any of those is a process still there. One that
//! has stopped speaking *and* stopped spending is the only one worth stopping.

use std::time::{Duration, Instant};

/// How long a system may be both silent and still before it is believed gone.
///
/// Silence alone never answers the question — a beat that cannot be written
/// leaves a working system looking stopped. This is the second half, and it
/// only begins once the beat's own wait has already passed: a system that has
/// also spent no processor time and moved no bytes for this long is not slow,
/// it is over.
///
/// It is short because it is added to a wait that has already run out, and
/// long enough that a disk pausing between two writes, or a page waiting for
/// a touch it will never get, is not mistaken for a broken one.
pub(crate) const STILL: Duration = Duration::from_secs(5);

/// The shape of a process at one moment.
///
/// Everything here comes from `/proc`, so none of it depends on the system
/// cooperating: a process that cannot write its beat can still be seen to be
/// spending processor time.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Motion {
    /// What it is doing: running, sleeping, or waiting on a disk.
    state: char,
    /// Processor time charged to it, in clock ticks.
    cpu: u64,
    /// Bytes it has read and written, where the kernel keeps that count.
    io: Option<u64>,
}

/// Whether a system that has gone quiet should now be believed gone.
///
/// The first answer is about the beat, which the caller has already taken: it
/// has stopped. This one is about the process, and it is asked of the kernel
/// rather than of the system.
///
/// A system that is moving is never believed gone, however long its beat has
/// been missing: a beat that cannot be written is the file's failure, not the
/// process's. The clock only starts once it has stopped moving too, and starts
/// again — from nothing — the moment anything moves.
#[derive(Default)]
pub(crate) struct Still {
    /// The last shape it was seen in, where it has been seen at all.
    last: Option<Motion>,
    /// When that shape was first held, once it stopped changing.
    since: Option<Instant>,
}

impl Still {
    /// Whether the system behind `pid` is now to be stopped.
    ///
    /// `None` from the kernel means the question cannot be asked: no such
    /// process, or a `/proc` that does not show it. Silence is then all there
    /// is to go on, and waiting for an answer that will not come is waiting
    /// for ever, so silence alone decides.
    pub(crate) fn believes(&mut self, pid: i32, now: Instant) -> bool {
        self.believes_what(read(pid), now)
    }

    /// The rule itself, given the answer the kernel would have given.
    fn believes_what(&mut self, seen: Option<Motion>, now: Instant) -> bool {
        let Some(at) = seen else { return true };
        match self.last {
            Some(before) if !moved(&before, &at) => {
                let since = *self.since.get_or_insert(now);
                now.saturating_duration_since(since) >= STILL
            }
            _ => {
                self.last = Some(at);
                self.since = None;
                false
            }
        }
    }

    /// The system spoke again: whatever quiet there was is over.
    pub(crate) fn forget(&mut self) {
        *self = Still::default();
    }
}

/// What the process looks like now, from the kernel's own account.
fn read(pid: i32) -> Option<Motion> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let mut motion = parse_stat(&stat)?;
    motion.io = read_io(pid);
    Some(motion)
}

/// The shape of a process from one line of `/proc/<pid>/stat`.
fn parse_stat(text: &str) -> Option<Motion> {
    // The name sits in parentheses and may hold spaces and parentheses of its
    // own, so the fields begin after the last closing one: everything past it
    // is numbers and a single letter, none of which is a parenthesis.
    let fields: Vec<&str> = text.rsplit_once(')')?.1.split_whitespace().collect();
    Some(Motion {
        state: fields.first()?.chars().next()?,
        // Counting from the state, the fourteenth field of the line is the
        // processor time spent in the program's own code and the fifteenth
        // the time spent in the kernel's. Both are time this process spent,
        // which is the question being asked.
        cpu: fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?,
        io: None,
    })
}

/// Bytes the process has read and written, where the kernel counts them.
fn read_io(pid: i32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/io")).ok()?;
    parse_io(&text)
}

/// The bytes in the kernel's own report, where it keeps one.
///
/// A kernel without task IO accounting answers `None`, and the state and the
/// processor time are then all there is — which is still enough: a process
/// moving an image to a disk spends processor time doing it.
fn parse_io(text: &str) -> Option<u64> {
    let mut total = 0;
    let mut counted = false;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let Ok(bytes) = value.trim().parse::<u64>() else { continue };
        if matches!(key.trim(), "rchar" | "wchar") {
            total += bytes;
            counted = true;
        }
    }
    counted.then_some(total)
}

/// Whether anything about the process changed between two looks.
///
/// The state counts as movement: a process that has gone from sleeping to
/// waiting on a disk is asking that disk for something, whether or not it has
/// answered yet. Being generous here costs a later verdict on a system that is
/// already over; being stingy costs a working system its life, in the middle
/// of an install, with the phone half erased.
fn moved(before: &Motion, after: &Motion) -> bool {
    after.cpu > before.cpu
        || after.state != before.state
        || match (before.io, after.io) {
            (Some(before), Some(after)) => after > before,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One line of `/proc/<pid>/stat`, with the fields that matter set.
    ///
    /// The fields between the state and the processor times are placeholders:
    /// the kernel writes numbers there and the rule only ever counts past
    /// them, so what they hold cannot change the answer.
    fn stat_line(name: &str, state: char, utime: u64, stime: u64) -> String {
        // The state, then ppid, pgrp, session, tty_nr, tpgid, flags, minflt,
        // cminflt, majflt and cmajflt — the ten fields before utime.
        let mut fields = vec![state.to_string()];
        fields.extend(["1", "4242", "4242", "0", "-1", "4194560", "0", "0", "0", "0"].map(String::from));
        fields.push(utime.to_string());
        fields.push(stime.to_string());
        format!("4242 ({name}) {}", fields.join(" "))
    }

    /// The times are found past the name, on the fields the kernel puts them.
    #[test]
    fn the_processor_time_is_both_halves_of_the_one_number() {
        let motion = parse_stat(&stat_line("skyward", 'S', 7, 3)).expect("a line the kernel writes");
        assert_eq!(motion.cpu, 10, "time in the program's code and in the kernel's alike");
        assert_eq!(motion.state, 'S');
    }

    /// A name may hold anything at all, including the character that is meant
    /// to end it, and must not shift the fields that follow.
    #[test]
    fn a_name_with_parentheses_does_not_shift_the_fields() {
        let motion = parse_stat(&stat_line("a (b) c)", 'R', 100, 25)).expect("still a line the kernel writes");
        assert_eq!(motion.cpu, 125);
        assert_eq!(motion.state, 'R');
    }

    /// A line that stops before the times is not a process that has spent
    /// none of them: it is a line that cannot be read, and the caller is told
    /// so rather than told "nothing".
    #[test]
    fn a_truncated_line_is_not_a_process_that_did_nothing() {
        assert_eq!(parse_stat("4242 (skyward) S 1 4242"), None);
        assert_eq!(parse_stat("not a stat line at all"), None);
    }

    #[test]
    fn the_bytes_counted_are_the_ones_read_and_written() {
        let counted = "rchar: 4096\nwchar: 8192\nsyscr: 12\nsyscw: 3\nread_bytes: 0\nwrite_bytes: 0\n";
        assert_eq!(parse_io(counted), Some(4096 + 8192), "the bytes, not the number of calls");
        assert_eq!(parse_io("syscr: 12\n"), None, "a kernel that counts no bytes says so by saying nothing");
    }

    #[test]
    fn any_of_the_three_is_movement() {
        let at = |state, cpu, io| Motion { state, cpu, io };
        assert!(moved(&at('S', 10, Some(100)), &at('S', 11, Some(100))), "processor time");
        assert!(moved(&at('S', 10, Some(100)), &at('S', 10, Some(101))), "a byte moved");
        assert!(moved(&at('S', 10, Some(100)), &at('D', 10, Some(100))), "asking a disk for something");
        assert!(!moved(&at('S', 10, Some(100)), &at('S', 10, Some(100))), "nothing at all");
        assert!(!moved(&at('S', 10, None), &at('S', 10, None)), "and nothing, where bytes are not counted");
    }

    /// A system that cannot write its beat but keeps working is never stopped,
    /// however long the beat has been missing. This is the whole reason the
    /// second question exists.
    #[test]
    fn silence_alone_is_never_a_verdict() {
        let mut still = Still::default();
        let start = Instant::now();
        // Twenty minutes of a full filesystem, asked every hundred
        // milliseconds, with the process working the whole time.
        for step in 0..12_000u64 {
            let working = Motion { state: 'R', cpu: step, io: Some(step) };
            assert!(!still.believes_what(Some(working), start + Duration::from_millis(step * 100)), "still working at step {step}");
        }
    }

    /// Both halves together are a verdict, and it is reached a full [`STILL`]
    /// after the process was last seen to change.
    #[test]
    fn a_system_that_stops_both_speaking_and_moving_is_believed_gone() {
        let mut still = Still::default();
        let start = Instant::now();
        let quiet = Motion { state: 'S', cpu: 10, io: Some(100) };
        assert!(!still.believes_what(Some(quiet), start), "the first look only says where it stands");
        assert!(!still.believes_what(Some(quiet), start), "and this one only starts the clock");
        assert!(!still.believes_what(Some(quiet), start + STILL - Duration::from_millis(1)), "not yet");
        assert!(still.believes_what(Some(quiet), start + STILL), "the whole wait of nothing");
    }

    /// Movement gives the whole wait back: a system that worked at the fourth
    /// second is not stopped at the fifth, it is stopped five seconds later.
    #[test]
    fn a_system_that_starts_moving_again_is_given_the_whole_wait_back() {
        let mut still = Still::default();
        let start = Instant::now();
        let quiet = Motion { state: 'S', cpu: 10, io: Some(100) };
        let working = Motion { state: 'R', cpu: 11, io: Some(100) };
        assert!(!still.believes_what(Some(quiet), start));
        assert!(!still.believes_what(Some(quiet), start + Duration::from_secs(4)), "four seconds of nothing");
        assert!(!still.believes_what(Some(working), start + Duration::from_secs(5)), "it moved again");
        assert!(!still.believes_what(Some(working), start + Duration::from_secs(9)), "and held that new shape");
        assert!(still.believes_what(Some(working), start + Duration::from_secs(14)), "five seconds of it");
    }

    /// Where the kernel will not say, silence alone decides — the alternative
    /// is a watcher that waits for ever on a question it cannot ask.
    #[test]
    fn a_process_the_kernel_will_not_describe_is_judged_on_silence_alone() {
        let mut still = Still::default();
        assert!(still.believes_what(None, Instant::now()));
    }

    /// The field positions, checked against a real kernel rather than against
    /// a line written here: this very process, described by `/proc`.
    ///
    /// A host build on a system without `/proc` skips it. On Linux it is the
    /// only test that would notice the fields being read from the wrong place,
    /// which a made-up line cannot catch — it was made up by the same reading
    /// of the format that the rule uses.
    ///
    /// The state is the part worth pinning: it is the first field after the
    /// name, so a name holding a bracket of its own would turn it into a
    /// number. Processor time is not asserted — this reads the process's first
    /// thread, which under a test harness is usually waiting, and a wait is
    /// charged nothing.
    #[cfg(unix)]
    #[test]
    fn this_process_describes_itself_the_way_the_rule_reads_it() {
        let motion = read(std::process::id() as i32).expect("a system with /proc describes its own process");
        assert!(matches!(motion.state, 'R' | 'S' | 'D'), "a letter, not a number: {:?}", motion.state);
        if let Some(io) = motion.io {
            assert!(io > 0, "reading this report is itself bytes read: {io}");
        }
    }

    /// Speaking again ends the quiet, so a later silence is counted from
    /// nothing rather than from the silence that came before it.
    #[test]
    fn a_system_that_speaks_again_starts_its_next_silence_from_nothing() {
        let mut still = Still::default();
        let start = Instant::now();
        let quiet = Motion { state: 'S', cpu: 10, io: Some(100) };
        assert!(!still.believes_what(Some(quiet), start));
        assert!(!still.believes_what(Some(quiet), start + Duration::from_secs(4)), "four seconds of nothing");
        still.forget();
        // Speaking again leaves nothing behind: the first look after it only
        // says where the system stands, and the wait runs from the one after.
        assert!(!still.believes_what(Some(quiet), start + Duration::from_secs(5)), "the first look after speaking");
        assert!(!still.believes_what(Some(quiet), start + Duration::from_secs(10)), "the wait begins here");
        assert!(still.believes_what(Some(quiet), start + Duration::from_secs(15)), "and runs out five seconds later");
    }
}
