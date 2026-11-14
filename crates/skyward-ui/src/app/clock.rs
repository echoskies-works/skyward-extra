//! The clock the desktop opens: the stopwatch and the timer, the paper the
//! watch keeps, and how often the page has to be told to draw again while
//! either count runs. The page that shows it is in `pages`.

/// The two faces of the clock: the time counted up from nothing, and the
/// time counted down to nothing. The bar at the foot of the page shows one
/// at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockFace {
    Stopwatch,
    Timer,
}

/// One entry on the stopwatch's paper: how long the run since the entry
/// before took, and how long the watch had run when it was taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lap {
    pub split: u64,
    pub total: u64,
}

/// How many entries the paper keeps. The page shows as many as fit and
/// lets the rest go as the newest come in.
pub(super) const CLOCK_LAPS: usize = 50;

/// How long the timer is set for before anyone sets it: five minutes.
const CLOCK_SPAN: u64 = 5 * 60_000;

/// The longest the timer is set for, and the least: the hour Smartisan's
/// clock gives it, and the one minute it can be set by.
pub(super) const CLOCK_MOST: u64 = 60;
pub(super) const CLOCK_LEAST: u64 = 1;

/// How long between the frames the page is drawn again by while a count
/// runs: thirty a second, which is enough for a reading to the hundredth
/// and well short of what a frame here costs.
pub(super) const CLOCK_STEP_MS: u64 = 33;

/// The clock's own state, as its page last left it. The readings are not
/// kept: both counts are worked out from the clock the page is given, so
/// the stopwatch goes on while the page is elsewhere, and so does the
/// timer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    /// The face the page shows.
    pub face: ClockFace,
    /// The stopwatch: how long it had run when it was last stopped, and
    /// when it went on again since.
    pub stop: u64,
    pub stop_since: Option<u64>,
    /// The entries taken, the newest last.
    pub laps: Vec<Lap>,
    /// The timer: how long it is set for, how much of it was left when it
    /// was last stopped, and when it went on again since.
    pub span: u64,
    pub left: u64,
    pub left_since: Option<u64>,
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            face: ClockFace::Stopwatch,
            stop: 0,
            stop_since: None,
            laps: Vec::new(),
            span: CLOCK_SPAN,
            left: CLOCK_SPAN,
            left_since: None,
        }
    }
}

impl Clock {
    /// What the stopwatch reads at `now`.
    pub fn stopwatch(&self, now: u64) -> u64 {
        self.stop + self.stop_since.map_or(0, |since| now.saturating_sub(since))
    }

    /// What the timer has left at `now`.
    pub fn timer(&self, now: u64) -> u64 {
        self.left_since.map_or(self.left, |since| self.left.saturating_sub(now.saturating_sub(since)))
    }

    /// Whether either count is going.
    pub fn running(&self) -> bool {
        self.stop_since.is_some() || self.left_since.is_some()
    }

    /// The timer has run out: it stops at nothing, waiting to be started
    /// again. Told so once, as it gets there.
    pub(super) fn arrive(&mut self, now: u64) -> bool {
        if self.left_since.is_some() && self.timer(now) == 0 {
            self.left = 0;
            self.left_since = None;
            return true;
        }
        false
    }
}
