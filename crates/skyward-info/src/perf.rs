//! What the machine is doing as it runs: how much of the processor is in
//! use, how fast each of its cores runs and how warm it is.
//!
//! The processor's counters only ever climb, so none of this is a state
//! that can be read off once: a reading says what changed since the one
//! before it, and a [`Sampler`] keeps the last counts to compare with.

use std::fs;
use std::path::{Path, PathBuf};

/// The fields `/proc/stat` keeps for a processor or a core, from `user`
/// through `steal`. `guest` and `guest_nice` follow them and are already
/// counted in `user` and `nice`, so they are left out.
const FIELDS: usize = 8;

/// The fields that are not work: `idle` and the `iowait` after it.
const IDLE: usize = 3;

/// One core's reading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Core {
    /// The core's number as the kernel names it: `cpu3` is 3. A core the
    /// kernel has taken away is not listed, so the numbers may have gaps.
    pub id: u32,
    /// The clock the core runs at now, in kHz; absent where cpufreq does
    /// not answer for it.
    pub khz: Option<u32>,
    /// The share of the core in use since the last reading, 0 to 1;
    /// absent on the first reading, which has nothing to compare with.
    pub busy: Option<f32>,
}

/// What the processor was doing between two readings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reading {
    /// The share of the whole processor in use since the last reading, 0
    /// to 1; absent on the first reading, and on one that follows the
    /// counters being reset.
    pub busy: Option<f32>,
    /// One for each core the kernel lists, in its order.
    pub cores: Vec<Core>,
    /// The warmest sensor that speaks for the processor, in degrees
    /// Celsius; absent where none does.
    pub celsius: Option<f32>,
}

impl Reading {
    /// The clock the processor runs at, as one figure for the lot: the
    /// mean of the cores that answer, in kHz. A core the kernel has taken
    /// away counts for nothing, as it does nothing.
    pub fn khz(&self) -> Option<u32> {
        let mut sum = 0u64;
        let mut n = 0u64;
        for core in &self.cores {
            if let Some(khz) = core.khz {
                sum += u64::from(khz);
                n += 1;
            }
        }
        (n > 0).then(|| (sum / n) as u32)
    }
}

/// A reader that keeps what it saw last time, so the next reading says
/// what changed, and the reading it last worked out.
#[derive(Clone, Debug, PartialEq)]
pub struct Sampler {
    /// Where the kernel's files are: `/` on a phone, a directory of
    /// fixtures in the tests.
    root: PathBuf,
    /// The whole processor's counts at the last reading, and each core's,
    /// in the order they were listed then.
    last: Option<Counts>,
    last_cores: Vec<Counts>,
    /// The thermal zone that spoke for the processor last time, kept so
    /// the next reading need not look through them all again.
    zone: Option<PathBuf>,
    reading: Reading,
}

/// The work and the total of a processor or a core, in jiffies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    busy: u64,
    total: u64,
}

impl Counts {
    /// The counts in one `/proc/stat` line, after its name. A line with
    /// fewer than the four fields every kernel keeps says nothing.
    fn of(rest: &str) -> Option<Self> {
        let values: Vec<u64> = rest.split_whitespace().take(FIELDS).map(|v| v.parse().ok()).collect::<Option<_>>()?;
        if values.len() <= IDLE {
            return None;
        }
        let total = values.iter().sum();
        let idle = values[IDLE] + values.get(IDLE + 1).copied().unwrap_or(0);
        Some(Counts { busy: total - idle, total })
    }
}

/// What changed between two counts, as a share of the time that passed;
/// absent where the counters say none passed or have been reset since.
fn share(was: Counts, now: Counts) -> Option<f32> {
    let total = now.total.checked_sub(was.total)?;
    if total == 0 {
        return None;
    }
    let busy = now.busy.saturating_sub(was.busy);
    Some(busy.min(total) as f32 / total as f32)
}

impl Default for Sampler {
    fn default() -> Self {
        Sampler::new()
    }
}

impl Sampler {
    /// A reader of the phone's own files.
    pub fn new() -> Self {
        Sampler::at("/")
    }

    /// A reader of the files under `root`.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Sampler { root: root.into(), last: None, last_cores: Vec::new(), zone: None, reading: Reading::default() }
    }

    /// The reading worked out last time, or an empty one before the first.
    pub fn reading(&self) -> &Reading {
        &self.reading
    }

    /// Read the processor, and say what it has done since the last time.
    pub fn read(&mut self) -> &Reading {
        self.reading = match fs::read_to_string(self.root.join("proc/stat")) {
            Ok(text) => self.of_stat(&text),
            Err(_) => Reading::default(),
        };
        &self.reading
    }

    fn of_stat(&mut self, text: &str) -> Reading {
        let (whole, cores) = counts(text);
        // A core the kernel has brought back has no counts of its own to
        // compare yet, and one it has taken away leaves nothing to hand on.
        let busy = self.last.and_then(|was| share(was, whole));
        let per: Vec<Option<f32>> =
            cores.iter().enumerate().map(|(k, (_, now))| self.last_cores.get(k).and_then(|was| share(*was, *now))).collect();
        self.last = Some(whole);
        self.last_cores = cores.iter().map(|(_, c)| *c).collect();
        let cores: Vec<Core> =
            cores.iter().zip(per).map(|((id, _), busy)| Core { id: *id, khz: self.khz(*id), busy }).collect();
        Reading { busy, cores, celsius: self.celsius() }
    }

    /// The clock a core runs at now, in kHz: what the governor has set,
    /// or else what the hardware says it is actually running at.
    fn khz(&self, id: u32) -> Option<u32> {
        let dir = self.root.join(format!("sys/devices/system/cpu/cpu{id}/cpufreq"));
        ["scaling_cur_freq", "cpuinfo_cur_freq"].iter().find_map(|name| fs::read_to_string(dir.join(name)).ok()?.trim().parse().ok())
    }

    /// The warmest sensor that speaks for the processor, in degrees
    /// Celsius.
    fn celsius(&mut self) -> Option<f32> {
        // The zone that answered last time is asked first: a phone's list
        // of them does not change, and looking through it every second
        // costs more than the one file's read.
        if let Some(zone) = &self.zone
            && let Some(celsius) = warm(zone)
        {
            return Some(celsius);
        }
        let class = self.root.join("sys/class/thermal");
        let mut best: Option<(PathBuf, f32)> = None;
        for entry in fs::read_dir(class).ok()?.flatten() {
            let zone = entry.path();
            let Ok(kind) = fs::read_to_string(zone.join("type")) else { continue };
            if !speaks_for_processor(kind.trim()) {
                continue;
            }
            let Some(celsius) = warm(&zone) else { continue };
            if best.as_ref().is_none_or(|(_, warmest)| celsius > *warmest) {
                best = Some((zone, celsius));
            }
        }
        let (zone, celsius) = best?;
        self.zone = Some(zone);
        Some(celsius)
    }
}

/// The counts in `/proc/stat`: the whole processor first, then each core
/// by its number, in the order the kernel lists them.
fn counts(text: &str) -> (Counts, Vec<(u32, Counts)>) {
    let mut whole = Counts::default();
    let mut cores = Vec::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(char::is_whitespace) else { continue };
        let Some(counts) = Counts::of(rest) else { continue };
        if name == "cpu" {
            whole = counts;
        } else if let Some(id) = name.strip_prefix("cpu").and_then(|id| id.parse().ok()) {
            cores.push((id, counts));
        }
    }
    (whole, cores)
}

/// Whether a thermal zone's kind is the processor's: Qualcomm and MediaTek
/// both name theirs after what they sit on (`cpu-0-0`, `soc-thermal`,
/// `cpu-1-0-usr`), and the battery, the charger and the modem name
/// themselves after nothing of the sort.
fn speaks_for_processor(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    kind.contains("cpu") || kind.contains("soc")
}

/// A thermal zone's temperature in degrees Celsius. Most drivers report
/// millidegrees; a few report degrees, which no phone reaches past 200.
fn warm(zone: &Path) -> Option<f32> {
    let raw: f32 = fs::read_to_string(zone.join("temp")).ok()?.trim().parse().ok()?;
    Some(if raw > 200.0 { raw / 1000.0 } else { raw })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A phone made of files, with `/proc/stat` saying what `stat` says.
    fn phone(stat: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("proc")).unwrap();
        fs::write(root.path().join("proc/stat"), stat).unwrap();
        root
    }

    #[test]
    fn a_reading_says_what_changed_since_the_one_before() {
        let root = phone(
            "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 50 0 50 400 0 0 0 0 0 0\ncpu1 50 0 50 400 0 0 0 0 0 0\nintr 12 0 0\n",
        );
        let mut sampler = Sampler::at(root.path());
        let first = sampler.read().clone();
        assert_eq!(first.busy, None, "the first reading has nothing to compare with");
        assert_eq!(first.cores.len(), 2);
        assert!(first.cores.iter().all(|core| core.busy.is_none()));
        assert_eq!(first.cores.iter().map(|core| core.id).collect::<Vec<_>>(), [0, 1]);

        // A quarter of the time that passed was work, for the processor and
        // for either core.
        fs::write(
            root.path().join("proc/stat"),
            "cpu  150 0 150 1100 0 0 0 0 0 0\ncpu0 75 0 75 550 0 0 0 0 0 0\ncpu1 75 0 75 550 0 0 0 0 0 0\nintr 12 0 0\n",
        )
        .unwrap();
        let second = sampler.read().clone();
        assert_eq!(second.busy, Some(0.25));
        assert_eq!(second.cores.iter().map(|core| core.busy).collect::<Vec<_>>(), [Some(0.25), Some(0.25)]);
    }

    #[test]
    fn the_clocks_of_the_cores_that_answer_make_the_one_figure() {
        let root = phone("cpu  0 0 0 0 0 0 0 0 0 0\ncpu0 0 0 0 0 0 0 0 0 0 0\ncpu1 0 0 0 0 0 0 0 0 0 0\n");
        for (id, khz) in [(0, 3_200_000), (1, 2_000_000)] {
            let dir = root.path().join(format!("sys/devices/system/cpu/cpu{id}/cpufreq"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("scaling_cur_freq"), format!("{khz}\n")).unwrap();
        }
        let reading = Sampler::at(root.path()).read().clone();
        assert_eq!(reading.cores.iter().map(|core| core.khz).collect::<Vec<_>>(), [Some(3_200_000), Some(2_000_000)]);
        assert_eq!(reading.khz(), Some(2_600_000), "the mean of the two");
    }

    #[test]
    fn the_warmest_sensor_that_speaks_for_the_processor_is_the_one_read() {
        let root = phone("cpu  0 0 0 0 0 0 0 0 0 0\n");
        let class = root.path().join("sys/class/thermal");
        for (name, kind, temp) in [("thermal_zone0", "battery", "45000"), ("thermal_zone1", "cpu-0-0", "61000"), ("thermal_zone2", "cpu-1-0", "72000")] {
            let zone = class.join(name);
            fs::create_dir_all(&zone).unwrap();
            fs::write(zone.join("type"), format!("{kind}\n")).unwrap();
            fs::write(zone.join("temp"), format!("{temp}\n")).unwrap();
        }
        assert_eq!(Sampler::at(root.path()).read().celsius, Some(72.0), "the battery is not the processor");
    }

    #[test]
    fn a_machine_without_the_kernel_s_files_reads_nothing() {
        let root = tempfile::tempdir().unwrap();
        let reading = Sampler::at(root.path()).read().clone();
        assert_eq!(reading, Reading::default(), "no counters, no cores, no warmth");
        assert_eq!(reading.khz(), None);
    }
}
