//! The clock the desktop opens: the stopwatch and the timer, the paper the
//! watch keeps, and how often the page has to be told to draw again while
//! either count runs. The page that shows it is in `pages`.

use super::{Action, App};
use crate::lang::Text;
use crate::tr;

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

/// The timer starts unset: no length has been chosen on the ruler yet.
const CLOCK_SPAN: u64 = 0;

/// The longest the ruler reaches: the three hours Smartisan's timer gives.
/// The least is zero, where the page waits for a finger to choose a minute.
pub(crate) const CLOCK_MOST: u64 = 180;
pub(super) const CLOCK_LEAST: u64 = 0;

/// The ruler keeps its place in thousandths of a minute, so a finger can
/// drag it smoothly and the page still settles on a whole minute.
pub(super) const RULER_UNITS: u32 = 1_000;
/// How wide one minute is on the ruler, in dp. The page draws by this and
/// a finger's horizontal travel is turned into minutes by it.
pub(crate) const RULER_MINUTE_DP: f32 = 24.0;

/// How long the hand takes to sweep back to zero when the watch is reset,
/// and how long it takes to fade between charcoal and blue when the watch
/// starts, stops or settles after that sweep. Both in milliseconds.
pub const HAND_REWIND_MS: u64 = 320;
/// How long a reset takes to carry the timer's ruler home: the same sweep
/// the hand makes, so the two faces settle at one pace.
pub const RULER_REWIND_MS: u64 = HAND_REWIND_MS;
pub const HAND_TINT_MS: u64 = 220;

/// The second hand's own motion, separate from the count it shows. Its
/// colour fades between charcoal and blue as the watch starts and stops;
/// on a reset it sweeps the way back to zero before fading to charcoal.
/// All the numbers are integers so the clock stays comparable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HandAnim {
    /// Where the colour fade began and where it is going, in thousandths:
    /// 0 is charcoal, 1000 is Xiaomi's blue.
    tint_from: u16,
    tint_to: u16,
    tint_since: u64,
    /// Where the backward sweep started, in hundredths of a degree. `None`
    /// when the hand is not sweeping. The fade stays blue until it ends.
    rewind_from: Option<u16>,
    rewind_since: u64,
}

impl HandAnim {
    /// How blue the hand reads now, 0 (charcoal) to 1 (blue).
    pub fn tint(&self, now: u64) -> f32 {
        let t = (now.saturating_sub(self.tint_since)).min(HAND_TINT_MS) as f32 / HAND_TINT_MS as f32;
        let eased = t * t * (3.0 - 2.0 * t);
        (self.tint_from as f32 + (self.tint_to as f32 - self.tint_from as f32) * eased) / 1000.0
    }

    /// Start a fade to `to` (0 or 1000) from wherever the colour is now.
    fn fade(&mut self, to: u16, now: u64) {
        self.tint_from = (self.tint(now) * 1000.0).round() as u16;
        self.tint_to = to;
        self.tint_since = now;
    }

    /// The start and the pause both fade the hand to the colour of that
    /// state: blue while the count moves, charcoal once it stands.
    pub fn follow(&mut self, running: bool, now: u64) {
        self.fade(if running { 1000 } else { 0 }, now);
    }

    /// The hand's angle while sweeping back, in degrees; `None` when the
    /// count's own angle should be used. Ease out so the sweep starts fast
    /// and settles onto zero, as Smartisan's does.
    pub fn rewind_deg(&self, now: u64) -> Option<f32> {
        let from = self.rewind_from?;
        let t = (now.saturating_sub(self.rewind_since)).min(HAND_REWIND_MS) as f32 / HAND_REWIND_MS as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        Some(from as f32 / 100.0 * (1.0 - eased))
    }

    /// The sweep has reached zero and can give the hand back to the count.
    fn rewind_done(&self, now: u64) -> bool {
        self.rewind_from.is_some() && now >= self.rewind_since.saturating_add(HAND_REWIND_MS)
    }

    /// Begin a reset sweep from the angle the hand was showing, then turn it
    /// blue for the trip home.
    pub fn reset(&mut self, deg: f32, now: u64) {
        self.rewind_from = Some((deg.clamp(0.0, 360.0) * 100.0).round() as u16);
        self.rewind_since = now;
        self.fade(1000, now);
    }

    /// Called once the sweep is over: hand the angle back to the count and
    /// let the colour fade to charcoal.
    fn end_rewind(&mut self, now: u64) {
        self.rewind_from = None;
        self.fade(0, now);
    }

    /// Whether anything here still needs a frame drawn: a fade under way,
    /// or a sweep not yet at zero.
    pub fn moving(&self, now: u64) -> bool {
        self.tint_from != self.tint_to && now < self.tint_since.saturating_add(HAND_TINT_MS)
            || self.rewind_from.is_some() && now < self.rewind_since.saturating_add(HAND_REWIND_MS)
    }

    /// Once the reset sweep lands, fade the hand to charcoal. Everything
    /// else — start and pause — is driven by the action that caused it.
    fn settle(&mut self, now: u64) {
        if self.rewind_done(now) {
            self.end_rewind(now);
        }
    }
}

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
        ClockFace::Stopwatch if clock.stop_since.is_some() || clock.hand.moving(now) => CLOCK_STEP_MS,
        // The reading is rounded up to the second it is shown by, so it
        // changes as the count comes down onto a whole second.
        // On its way home from a reset the scale is drawn again a frame at
        // a time, as the hand is.
        ClockFace::Timer if clock.ruler_sweeping(now) => CLOCK_STEP_MS,
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
    /// The second hand's colour fade and reset sweep.
    pub hand: HandAnim,
    /// Which of them is at the top of the paper, when there are more than
    /// the page shows: what a finger has scrolled down to, the oldest first.
    pub paper: usize,
    /// The timer: how long it is set for, how much of it was left when it
    /// was last stopped, and when it went on again since.
    pub span: u64,
    pub left: u64,
    pub left_since: Option<u64>,
    /// Where the ruler stands, in thousandths of a minute. It moves under
    /// the finger and settles on a whole minute.
    pub ruler: u32,
    /// Where a reset's sweep of the ruler began, in thousandths of a minute,
    /// and when: `None` while the ruler stands where it was left.
    pub ruler_from: Option<u32>,
    pub ruler_since: u64,
    /// A timer held at pause stays held when the ruler is moved; a timer
    /// merely unset or run out starts when a minute is chosen.
    pub timer_paused: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Clock {
            face: ClockFace::Stopwatch,
            stop: 0,
            stop_since: None,
            laps: Vec::new(),
            hand: HandAnim::default(),
            paper: 0,
            span: CLOCK_SPAN,
            left: CLOCK_SPAN,
            left_since: None,
            ruler: 0,
            ruler_from: None,
            ruler_since: 0,
            timer_paused: false,
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

    /// Whether a length has been chosen on the ruler. Nothing can be reset
    /// or started before that, so both keys are quiet on a new timer.
    pub fn timer_set(&self) -> bool {
        self.span > 0
    }

    /// Whether the timer's ruler may be moved: a count running locks it,
    /// while one unset, paused or run out can be set again.
    pub fn timer_can_slide(&self) -> bool {
        self.left_since.is_none()
    }

    /// Where the ruler stands, in minutes, for drawing its marks. A reset
    /// carries it home the way the hand is carried: the scale runs back to
    /// zero rather than being there the next frame.
    pub fn ruler_value(&self, now: u64) -> f32 {
        self.ruler_rewind(now).unwrap_or(self.ruler as f32) / RULER_UNITS as f32
    }

    /// Where the ruler is on its way home, in thousandths of a minute;
    /// `None` while it stands where it was left. Eased out, so the sweep
    /// leaves fast and settles onto zero as the hand's does.
    fn ruler_rewind(&self, now: u64) -> Option<f32> {
        let from = self.ruler_from?;
        let t = now.saturating_sub(self.ruler_since).min(RULER_REWIND_MS) as f32 / RULER_REWIND_MS as f32;
        let eased = 1.0 - (1.0 - t).powi(3);
        Some(from as f32 * (1.0 - eased))
    }

    /// Whether the ruler is still on its way home, which asks for a frame at
    /// a time until it lands.
    pub fn ruler_sweeping(&self, now: u64) -> bool {
        self.ruler_from.is_some() && now < self.ruler_since.saturating_add(RULER_REWIND_MS)
    }

    /// Begin a reset's sweep of the ruler from where it stands: the ruler
    /// itself goes back to nothing at once, and the sweep is what the scale
    /// is drawn by until it lands.
    pub fn sweep_ruler(&mut self, now: u64) {
        self.ruler_from = Some(self.ruler);
        self.ruler = 0;
        self.ruler_since = now;
    }

    /// The sweep has landed: the scale is drawn by the ruler again, which is
    /// where the sweep left it.
    pub fn land_ruler(&mut self, now: u64) {
        if self.ruler_from.is_some() && !self.ruler_sweeping(now) {
            self.ruler_from = None;
        }
    }

    /// Drag the ruler by a whole number of thousandths of a minute. The
    /// count returned is how many whole-minute marks the finger passed, so
    /// the page can tap the motor once for each, even in one long move.
    pub(super) fn drag_ruler(&mut self, delta: i32) -> u32 {
        let before = self.ruler as i64;
        let after = (before + delta as i64).clamp(0, CLOCK_MOST as i64 * RULER_UNITS as i64);
        let units = RULER_UNITS as i64;
        // Both ends are nonnegative, so the ceiling of a division is one
        // more than its floor when anything is left over.
        let ceiling = |v: i64| (v + units - 1) / units;
        let crossed = if after > before {
            after / units - before / units
        } else if after < before {
            ceiling(before) - ceiling(after)
        } else {
            0
        };
        self.ruler = after as u32;
        // A finger on the ruler takes it off its way home.
        self.ruler_from = None;
        crossed as u32
    }

    /// Let the ruler go. A hand pulled towards more time settles on the
    /// minute ahead; one pulled towards less settles on the minute behind.
    /// A finger that never moved leaves a whole minute where it was. An
    /// unset or finished timer then starts; a paused one stays paused.
    pub(super) fn settle_ruler(&mut self, direction: i32, now: u64) -> u64 {
        let whole = self.ruler / RULER_UNITS;
        let part = self.ruler % RULER_UNITS;
        let pick = if part == 0 {
            whole
        } else if direction > 0 {
            whole + 1
        } else if direction < 0 {
            whole
        } else if part * 2 >= RULER_UNITS {
            whole + 1
        } else {
            whole
        };
        let minute = pick.min(CLOCK_MOST as u32) as u64;
        self.ruler = minute as u32 * RULER_UNITS;
        if minute == 0 {
            self.span = 0;
            self.left = 0;
            self.left_since = None;
            self.timer_paused = false;
            return 0;
        }
        self.span = minute * 60_000;
        self.left = self.span;
        if !self.timer_paused {
            self.left_since = Some(now);
        }
        minute
    }

    /// Whether either count is going.
    pub fn running(&self) -> bool {
        self.stop_since.is_some() || self.left_since.is_some()
    }

    /// Where the stopwatch's hand points at `now`, clockwise from twelve.
    /// One turn a minute: at sixty seconds the count leaves the minute and
    /// the angle comes back to zero without a jump. A reset sweeps it back
    /// the way it came before the count takes over again.
    pub fn hand_deg(&self, now: u64) -> f32 {
        if let Some(deg) = self.hand.rewind_deg(now) {
            return deg;
        }
        (self.stopwatch(now) % 60_000) as f32 / 60_000.0 * 360.0
    }

    /// How blue the hand reads now. When no fade or sweep is under way the
    /// colour simply follows whether the count runs, so a clock set up
    /// without pressing a key — a preview, a restored state — still draws
    /// the right ink.
    pub fn hand_tint(&self, now: u64) -> f32 {
        if self.hand.moving(now) {
            self.hand.tint(now)
        } else if self.stop_since.is_some() {
            1.0
        } else {
            0.0
        }
    }

    /// Once a reset sweep lands, hand the angle back to the count and fade
    /// the blue out. Called on every tick; cheap when nothing is sweeping.
    pub fn settle_hand(&mut self, now: u64) {
        self.hand.settle(now);
    }

    /// The timer has run out: it stops at nothing, waiting to be started
    /// again. Told so once, as it gets there.
    pub(super) fn arrive(&mut self, now: u64) -> bool {
        if self.left_since.is_some() && self.timer(now) == 0 {
            self.left = 0;
            self.left_since = None;
            self.timer_paused = false;
            return true;
        }
        false
    }
}

const TIMER_DONE_TITLE: Text = ["计时结束", "計時結束", "Timer done"];
const TIMER_DONE_OK: Text = ["确定", "確定", "OK"];

impl App {
    /// The timer ran out: write what finished into the common sheet and
    /// raise it over the page. The caller starts the long motor note; the
    /// sheet's one key, in [`App::act`], is what stops it.
    pub(super) fn show_timer_done(&mut self, minutes: u64, now: u64) {
        let lang = self.state.lang;
        let said = tr!(
            lang,
            "时长为{minutes}分钟的计时已结束",
            "時長為{minutes}分鐘的計時已結束",
            "The {minutes}-minute timer has ended"
        );
        self.show_sheet(TIMER_DONE_TITLE, said, vec![vec![(TIMER_DONE_OK, Action::TimerDoneOk)]], now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{App, Ctx, Effect, Frame, Screen, Switch, Touch, DEFAULT_TIMER_SOUND};
    use crate::canvas::Canvas;
    use crate::font::Fonts;
    use crate::theme::Theme;
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
    fn the_hand_turns_once_a_minute_from_zero() {
        let mut clock = Clock::default();
        assert_eq!(clock.hand_deg(0), 0.0, "an untouched watch reads zero");
        clock.stop_since = Some(1_000);
        assert!((clock.hand_deg(1_000) - 0.0).abs() < 0.01, "the start is at zero");
        assert!((clock.hand_deg(16_000) - 90.0).abs() < 0.01, "a quarter minute is a quarter turn");
        assert!((clock.hand_deg(31_000) - 180.0).abs() < 0.01, "half a minute is half a turn");
        // Sixty seconds lands on the minute: the angle comes back to zero,
        // not a full turn, so there is no jump between 359° and 0°.
        assert!(clock.hand_deg(60_999) > 359.0, "a hair before the minute it is nearly all the way round");
        assert!(clock.hand_deg(61_000) < 0.01, "back at the minute the hand is at zero");
        assert!((clock.hand_deg(62_000) - 6.0).abs() < 0.01, "a second in it has moved on");
        // Pausing freezes the angle where the count stopped.
        clock.stop = clock.stopwatch(16_000);
        clock.stop_since = None;
        assert!((clock.hand_deg(99_000) - 90.0).abs() < 0.01, "a paused hand holds its angle");
    }

    #[test]
    fn a_reset_sweeps_the_hand_home_then_fades_it_grey() {        // A watch paused a quarter of the way round, then reset.
        let mut clock = Clock { stop: 15_000, stop_since: None, ..Clock::default() };
        assert!((clock.hand_deg(1_000) - 90.0).abs() < 0.01, "the paused hand stands at a quarter turn");
        let deg = clock.hand_deg(1_000);
        clock.hand.reset(deg, 1_000);
        clock.stop = 0; // the reset action clears the count as the sweep begins
        // The sweep starts at the angle it was showing, blue on its way.
        let start = clock.hand_deg(1_000);
        assert!((start - 90.0).abs() < 0.01, "the sweep begins where the hand was");
        assert!(clock.hand_tint(1_000 + HAND_TINT_MS) > 0.99, "the hand is blue during the sweep");
        // Ease out: more than half the way back well before the end.
        let mid = clock.hand_deg(1_000 + HAND_REWIND_MS / 2);
        assert!(mid < 45.0, "the sweep eases out early, got {mid} degrees");
        // When it lands the count takes over at zero and the blue fades.
        clock.settle_hand(1_000 + HAND_REWIND_MS);
        assert_eq!(clock.hand_deg(1_000 + HAND_REWIND_MS), 0.0, "the sweep ends at zero");
        assert!(clock.hand_tint(1_000 + HAND_REWIND_MS) > 0.99, "still blue the moment it lands");
        assert!(clock.hand_tint(1_000 + HAND_REWIND_MS + HAND_TINT_MS) < 0.01, "then fades to charcoal");
        assert!(!clock.hand.moving(1_000 + HAND_REWIND_MS + HAND_TINT_MS + 1), "and stops asking for frames");
    }

    #[test]
    fn starting_and_pausing_fade_the_hand_between_grey_and_blue() {
        let mut clock = Clock::default();
        assert!(clock.hand_tint(0) < 0.01, "a new watch is charcoal");
        clock.stop_since = Some(1_000);
        clock.hand.follow(true, 1_000);
        assert!(clock.hand_tint(1_000) < 0.01, "the fade starts at grey");
        assert!(clock.hand_tint(1_000 + HAND_TINT_MS / 2) > 0.4, "halfway through it is mostly blue");
        assert!(clock.hand_tint(1_000 + HAND_TINT_MS) > 0.99, "then fully blue");
        clock.stop = clock.stopwatch(2_000);
        clock.stop_since = None;
        clock.hand.follow(false, 2_000);
        assert!(clock.hand.moving(2_000), "pausing starts a fade too");
        assert!(clock.hand_tint(2_000 + HAND_TINT_MS) < 0.01, "back to charcoal");
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
        clock.left = 2_000;
        clock.left_since = Some(0);
        assert_eq!(step(&clock, 1_250), 750, "a moment short of the second it shows next");
        assert_eq!(step(&clock, 2_000), 1_000, "lands on a whole second, the next one after it");
    }

    #[test]
    fn the_ruler_settles_on_the_minute_the_finger_was_travelling_towards() {
        let mut clock = Clock {
            ruler: 2_500,
            ..Clock::default()
        };
        assert_eq!(clock.settle_ruler(1, 0), 3, "pulled towards more time, it goes on");
        assert_eq!(clock.timer(0), 180_000, "and starts from three whole minutes");
        assert!(clock.left_since.is_some());

        let mut clock = Clock {
            ruler: 2_500,
            ..Clock::default()
        };
        assert_eq!(clock.settle_ruler(-1, 0), 2, "pulled towards less, it goes back");
        assert_eq!(clock.timer(0), 120_000);

        let mut clock = Clock {
            ruler: 2_500,
            ..Clock::default()
        };
        assert_eq!(clock.settle_ruler(0, 0), 3, "a finger that only stopped rounds up");
    }

    #[test]
    fn one_vibration_goes_with_each_whole_minute_the_ruler_passes() {
        let mut clock = Clock::default();
        assert_eq!(clock.drag_ruler(999), 0, "short of the first mark");
        assert_eq!(clock.drag_ruler(2), 1, "the first mark passed");
        assert_eq!(clock.drag_ruler(1_500), 1, "the second mark passed");
        assert_eq!(clock.drag_ruler(-1_000), 1, "one mark back down");
        assert_eq!(clock.drag_ruler(-400), 0, "short of another");
    }

    #[test]
    fn choosing_a_minute_starts_the_timer_and_a_paused_one_stays_paused() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.state.clock.face = ClockFace::Timer;
        let frame = app.frame(&f, &i);
        assert!(frame.clock_ruler.is_some(), "an unset timer offers its ruler");
        assert!(!frame.hits.iter().any(|h| matches!(h.action, Action::ClockGo | Action::ClockReset)), "both keys wait");

        let ruler = frame.clock_ruler.expect("the ruler");
        let (x, y) = ruler.centre();
        let step = app.theme().m.dp(RULER_MINUTE_DP);
        app.touch(Touch::Down(x, y), &f, 0);
        let fx = app.touch(Touch::Move(x - step, y), &f, 8);
        assert_eq!(fx, vec![Effect::Haptic]);
        app.touch(Touch::Up, &f, 16);
        let clock = &app.state.clock;
        assert_eq!(clock.ruler, 1_000);
        assert!(clock.left_since.is_some(), "landing on a minute starts it");

        app.act(Action::ClockGo, 100);
        assert!(app.state.clock.timer_paused);
        let frame = app.frame(&f, &i);
        assert!(frame.clock_ruler.is_some(), "a paused ruler can still be moved");
        let ruler = frame.clock_ruler.expect("the ruler");
        let (x, y) = ruler.centre();
        app.touch(Touch::Down(x, y), &f, 200);
        app.touch(Touch::Move(x - step, y), &f, 208);
        app.touch(Touch::Up, &f, 216);
        assert_eq!(app.state.clock.ruler, 2_000);
        assert!(app.state.clock.left_since.is_none(), "it stays paused after the ruler settles");
        app.act(Action::ClockGo, 300);
        assert!(app.state.clock.left_since.is_some(), "only the start key runs it again");
    }

    /// A reset carries the timer's ruler home the way it carries the hand:
    /// the scale runs back from where it stood and lands at zero, and asks
    /// for frames until it does.
    #[test]
    fn a_reset_carries_the_rulers_scale_home() {
        let mut clock = Clock { face: ClockFace::Timer, ruler: 3_000, span: 180_000, ..Clock::default() };
        clock.sweep_ruler(1_000);
        assert_eq!(clock.ruler, 0, "the ruler itself is nothing at once");
        assert!((clock.ruler_value(1_000) - 3.0).abs() < 0.01, "the scale starts where it stood");
        let half = 1_000 + RULER_REWIND_MS / 2;
        let mid = clock.ruler_value(half);
        assert!(mid > 0.0 && mid < 3.0, "and is on its way back, got {mid}");
        assert!(clock.ruler_sweeping(half), "which asks for a frame at a time");
        assert!(!clock.ruler_sweeping(1_000 + RULER_REWIND_MS), "the sweep is over at its end");
        assert_eq!(clock.ruler_value(1_000 + RULER_REWIND_MS), 0.0, "and the scale is at zero");
        clock.land_ruler(1_000 + RULER_REWIND_MS);
        assert!(clock.ruler_from.is_none(), "with nothing left asking for frames");
        assert_eq!(clock.ruler_value(9_000), 0.0, "the scale stays where it landed");
    }

    /// A finger on the ruler takes it off its way home and counts from where
    /// the ruler now stands, not from the sweep.
    #[test]
    fn a_finger_on_the_ruler_takes_it_off_its_way_home() {
        let mut clock = Clock { face: ClockFace::Timer, ruler: 3_000, ..Clock::default() };
        clock.sweep_ruler(0);
        assert!(clock.ruler_sweeping(1), "the reset has it on its way");
        clock.drag_ruler(500);
        assert!(clock.ruler_from.is_none(), "the finger takes it back");
        assert_eq!(clock.ruler, 500);
        clock.drag_ruler(500);
        assert_eq!(clock.ruler, 1_000);
    }

    #[test]
    fn a_timer_that_ends_shows_its_notice_and_its_key_clears_everything() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.state.clock.face = ClockFace::Timer;
        app.state.clock.ruler = 1_000;
        app.state.clock.span = 60_000;
        app.state.clock.left = 0;
        app.state.clock.left_since = Some(0);
        app.frame(&f, &i);
        let fx = app.tick(1);
        assert!(fx.contains(&Effect::TimerAlarm { on: true }), "the long note starts with the notice");
        let sheet = app.sheet.as_ref().expect("the notice shows");
        assert_eq!(sheet.body, "时长为1分钟的计时已结束");
        assert_eq!(sheet.rows.len(), 1, "one row of keys");
        assert_eq!(sheet.rows[0].len(), 1, "one 确定 key");

        let fx = app.act(Action::TimerDoneOk, 2);
        assert_eq!(fx, vec![Effect::TimerAlarm { on: false }], "the note stops at 确定");
        assert!(app.sheet.as_ref().unwrap().closing.is_some(), "the sheet falls back after the note stops");
        let clock = &app.state.clock;
        assert_eq!((clock.ruler, clock.span, clock.left), (0, 0, 0));
        assert!(clock.left_since.is_none());
    }

    /// The timer wears the stopwatch's dial from the first frame, bare of its
    /// hand, and says how a length is set until one is: the short rule under
    /// the count and the words under it. Once the ruler has set a length the
    /// pair goes, and a reset brings both back.
    #[test]
    fn the_timer_wears_the_dial_and_says_how_it_is_set_until_it_has_been() {
        let (f, i) = (fonts(), info());
        let (w, h, ppd) = (1080, 2400, 2.625);
        let mut app = App::new(Screen::Clock, w, h, ppd);
        app.state.clock.face = ClockFace::Timer;
        let (t, m) = (app.theme(), Theme::light(w, h, ppd).m);
        // The middle of a row `cy` dp down the page.
        let at = |app: &App, cy: f32| {
            let fr = anew(app, &f, &i, 0);
            let k = (m.dp(cy).round() as usize * w as usize + w as usize / 2) * 4;
            [fr.canvas.data[k], fr.canvas.data[k + 1], fr.canvas.data[k + 2]]
        };
        // The dial is the very disc the keys under it are made of, which is
        // the chip's fill rather than the page's card.
        let round = crate::widgets::disc_fill(&t);
        let disc = [round.r, round.g, round.b];
        let rule = [t.pal.hairline.r, t.pal.hairline.g, t.pal.hairline.b];
        let page = [t.pal.bg.r, t.pal.bg.g, t.pal.bg.b];
        assert_eq!(at(&app, crate::pages::DIAL_CY), disc, "the dial is the stopwatch's own face, and no hand on it");
        assert_eq!(at(&app, crate::pages::RULE_CY), rule, "a new timer says how a length is set");
        app.state.clock.span = 60_000;
        app.state.clock.left = 60_000;
        assert_eq!(at(&app, crate::pages::RULE_CY), page, "with a length on the ruler the rule goes");
        app.state.clock.span = 0;
        app.state.clock.left = 0;
        assert_eq!(at(&app, crate::pages::RULE_CY), rule, "and a reset brings it back");
    }

    /// The gear at the page's top right is the bar's round button, the `<`
    /// chip mirrored: a page that does not scroll never brings the chip up,
    /// so the gear stands there on its own. It opens the clock's settings,
    /// which hold the timer's sound, a rule, and the two switches that say
    /// whether the clock ticks.
    #[test]
    fn the_gear_opens_the_clock_s_settings_and_its_switches_move() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        assert_eq!(app.frame(&f, &i).button, Some((crate::widgets::Glyph::Gear, Action::Open(Screen::ClockSettings))));
        tap_button(&mut app, &f, &i, 0);
        assert_eq!(app.screen(), Screen::ClockSettings);
        let frame = app.frame(&f, &i);
        assert!(frame.back && frame.header, "a page of the settings goes back with `<`");
        let acts: Vec<Action> = frame.hits.iter().map(|h| h.action).collect();
        assert_eq!(
            acts,
            [Action::Open(Screen::ClockSound), Action::Toggle(Switch::ClockTicking), Action::Toggle(Switch::ClockTickingBackground)]
        );
        assert!(app.state.clock_ticking, "the clock ticks until it is told not to");
        assert!(!app.state.clock_ticking_background, "and stops when its page goes, until told otherwise");
        tap_at(&mut app, &f, &i, Action::Toggle(Switch::ClockTicking), 1_000);
        assert!(!app.state.clock_ticking);
        tap_at(&mut app, &f, &i, Action::Toggle(Switch::ClockTickingBackground), 2_000);
        assert!(app.state.clock_ticking_background);
    }

    /// The gear stands at the top right of the calculator and of the player
    /// as it does on the clock, and both keep it on its chip rather than
    /// wait for a scroll that cannot come. Neither has a settings page yet,
    /// so a touch leaves the page where it is and says nothing about it.
    #[test]
    fn the_gear_stands_on_the_calculator_and_the_player_with_nothing_behind_it() {
        let (f, i) = (fonts(), info());
        for screen in [Screen::Calculator, Screen::Music] {
            let mut app = App::new(screen, W, H, PPD);
            let frame = app.frame(&f, &i);
            assert_eq!(frame.button, Some((crate::widgets::Glyph::Gear, Action::Idle)), "{screen:?} carries the clock's gear");
            assert!(frame.button_still, "{screen:?} never scrolls, so its chip stands");
            tap_button(&mut app, &f, &i, 0);
            assert_eq!(app.screen(), screen, "{screen:?} stays where it is");
            assert!(app.toast.is_none(), "{screen:?} says nothing about a page it does not have");
        }
    }

    /// The sound a timer's end makes is one of the sounds the ramdisk
    /// carries, shown by the name the names file gives it, and the one in
    /// use is ticked. The page stays once another is taken, as the
    /// language page's does.
    #[test]
    fn the_timer_s_sound_is_chosen_from_the_ones_the_ramdisk_carries() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Clock, W, H, PPD);
        let named = |zh: &str, en: &str| skyward_config::SoundName { zh: zh.into(), en: en.into() };
        app.state.sound_names.ring.insert(DEFAULT_TIMER_SOUND.into(), named("恋爱裁判", "Love Trial"));
        app.state.sound_names.ring.insert("bell".into(), named("铃声", "Bell"));
        // The two by their keys: `bell` comes before `love_trial`.
        assert_eq!(crate::pages::timer_sounds(&app.state, crate::Lang::Hans), [
            ("bell".to_string(), "铃声".to_string()),
            (DEFAULT_TIMER_SOUND.to_string(), "恋爱裁判".to_string()),
        ]);
        tap_button(&mut app, &f, &i, 0);
        tap(&mut app, &f, &i, Action::Open(Screen::ClockSound));
        assert_eq!(app.screen(), Screen::ClockSound);
        tap(&mut app, &f, &i, Action::ClockSound(0));
        assert_eq!(app.state.timer_sound, "bell");
        assert_eq!(app.screen(), Screen::ClockSound, "the page stays with its tick moved");
        tap(&mut app, &f, &i, Action::ClockSound(1));
        assert_eq!(app.state.timer_sound, DEFAULT_TIMER_SOUND, "and the one it started with is there to go back to");
    }
}
