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
/// runs: sixty a second, so the hundredths move one or two at a frame. The
/// page is told to draw again only the rows its reading is in, which is
/// what lets it keep to that pace.
pub(super) const CLOCK_STEP_MS: u64 = 16;

/// How long until the reading next changes, while a count on the face shown
/// runs. The stopwatch reads to the hundredth, so it takes a frame on; the
/// timer reads only to the second, so it takes the rest of the second it is
/// showing. A face left standing still takes a second, which is near enough
/// for a count on the other face that runs out to be tapped for then.
pub(super) fn step(clock: &Clock, now: u64) -> u64 {
    match clock.face {
        ClockFace::Stopwatch if clock.stop_since.is_some() => CLOCK_STEP_MS,
        // The reading is rounded up to the second it is shown by, so it
        // changes as the count comes down onto a whole second.
        ClockFace::Timer if clock.left_since.is_some() => match clock.timer(now) % 1_000 {
            0 => 1_000,
            left => left,
        },
        _ => 1_000,
    }
}

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
    /// Which of them is at the top of the paper, when there are more than
    /// the page shows: what a finger has scrolled down to, the oldest first.
    pub paper: usize,
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
            paper: 0,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{App, Ctx, Frame, Screen};
    use crate::canvas::Canvas;
    use crate::font::Fonts;
    use skyward_info::DeviceInfo;

    const W: u32 = 400;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// The page as it stands at `now` drawn from nothing, to compare what
    /// the frame kept against.
    fn anew(app: &App, f: &Fonts, i: &DeviceInfo, now: u64) -> Frame {
        let t = app.theme();
        crate::pages::render(
            Screen::Clock,
            &Ctx { t: &t, f, info: i, state: &app.state, term: None, band: None, now, wallpaper: app.wallpaper.as_ref() },
        )
    }

    #[test]
    fn the_reading_is_drawn_again_as_the_stopwatch_runs() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        let start = 1_000;
        app.state.clock.stop_since = Some(start);
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", start);
        // A moment of the count later, the page kept is the page as it now
        // stands: the reading has to follow the count.
        app.tick(start + 500);
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", start + 500);
        let kept = &app.frame.as_ref().expect("composed").canvas;
        assert!(kept.data == anew(&app, &f, &i, start + 500).canvas.data, "the reading follows the count");
    }

    #[test]
    fn the_reading_is_drawn_again_as_the_timer_runs_down() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        let start = 1_000;
        app.state.clock.face = ClockFace::Timer;
        app.state.clock.left_since = Some(start);
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", start);
        app.tick(start + 1_200);
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", start + 1_200);
        let kept = &app.frame.as_ref().expect("composed").canvas;
        assert!(kept.data == anew(&app, &f, &i, start + 1_200).canvas.data, "the reading runs down");
    }

    #[test]
    fn the_stopwatch_is_waited_for_a_frame_and_the_timer_for_the_second_it_shows() {
        let mut clock = Clock::default();
        assert_eq!(step(&clock, 0), 1_000, "a clock at rest is waited for a second at a time");
        clock.stop_since = Some(0);
        assert_eq!(step(&clock, 4_000), CLOCK_STEP_MS, "the stopwatch reads to the hundredth: a frame on");
        // The timer is rounded up to the second it shows, so it changes as
        // the count comes down onto a whole second, not as a second passes.
        clock.face = ClockFace::Timer;
        clock.left_since = Some(0);
        assert_eq!(step(&clock, 1_250), 750, "a moment short of the second it shows next");
        assert_eq!(step(&clock, 2_000), 1_000, "lands on a whole second, the next one after it");
    }
}
