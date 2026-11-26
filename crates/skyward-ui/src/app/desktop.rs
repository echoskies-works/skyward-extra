//! The desktop the phone starts on, and the pull from the foot of the
//! screen that takes a page off for the desktop again. The desktop's own
//! page is drawn in `pages`.

use super::*;

/// How tall, in dp, the strip at the foot of the screen a finger lands in
/// to pull the page off: the strip a phone keeps for its gesture bar.
pub(super) const FOOT_DP: f32 = 24.0;
/// How far a pull let go carries on, in screen heights a second, for the
/// page to leave. The same measure the lock uses for its flick, so the same
/// push does the same thing whether the screen is locked or not.
pub(super) const FOOT_FLING: f32 = 1.2;
/// How long a pull let go takes to carry the page up as far as the throw
/// goes. Quicker than the lock's `LOCK_AWAY`, which leaves the screen
/// entire: a page only goes a quarter of the way up and then folds, so the
/// same pace over a shorter way would read as a page dragged, not thrown.
pub(super) const FOOT_AWAY: f32 = 0.30;
/// How long a pull takes to come back when it went nowhere. The lock's own
/// coming back, and quicker than going away: coming back reads as undoing
/// a move, not as a move of its own.
pub(super) const FOOT_BACK: f32 = 0.34;
/// The most a pull may take the page up, in screen heights: its top edge
/// never goes more than a quarter of a screen past the top, however far the
/// finger travels and however hard it is thrown. Past that the page holds
/// still and the finger leaves it behind — a page is being lifted to be
/// folded away, not thrown out of sight.
pub(super) const FOOT_LIFT: f32 = 0.25;
/// How far a pull has to come, in screen heights, for letting go to take
/// the page off. Short of it the page goes back unless the finger was
/// thrown; past it, a finger that comes to rest and lifts still leaves for
/// the desktop, since a page held a quarter of the way up has plainly been
/// asked to go.
pub(super) const FOOT_MARK: f32 = 0.125;

/// A finger at the foot of a page, lifting it so the desktop comes in under
/// it. Let go past the mark and the page leaves: thrown, it carries on up
/// as far as `FOOT_LIFT` allows and then folds into its icon; barely moving,
/// it folds at once. Let go short of the mark without a throw and it sinks
/// back where it came from.
pub(super) struct Foot {
    /// How far the pull has come, in pixels the page has left by.
    pub(super) pull: f32,
    /// Where it comes to rest once the finger lifts: as far up as the throw
    /// carries it, or the screen's height for the lock, or nothing for it to
    /// go back.
    pub(super) to: f32,
    /// The spring carrying it there, once the finger has gone.
    pub(super) spring: Option<Spring>,
}

impl App {
    /// How far up a pull may take the page, in pixels: its top edge never
    /// goes more than `FOOT_LIFT` past the top of the screen.
    pub(super) fn lift_top(&self) -> f32 {
        self.h as f32 * FOOT_LIFT
    }

    /// The desktop drawn on its own, status bar and all: what a page pulled
    /// off the screen leaves showing.
    pub(super) fn desktop_canvas(&mut self, f: &Fonts, info: &DeviceInfo, clock: &str) -> Canvas {
        let t = self.theme();
        let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: None, now: self.now, wallpaper: self.wallpaper.as_ref(), tasks: &self.tasks };
        let mut c = pages::render(Screen::Desktop, &cx).canvas;
        let bar = self.bar(clock, info);
        widgets::status_bar(&mut c, &t, f, &bar, pages::DESKTOP_STATUS, true);
        c
    }

    /// Carry a pull let go on to where it goes: the page up as far as the
    /// throw takes it and then folding into its icon, or back where it came
    /// from.
    pub(super) fn settle_foot(&mut self, now: u64) -> Vec<Effect> {
        let fx = Vec::new();
        let top = self.lift_top();
        let Some(foot) = self.foot.as_mut() else { return fx };
        let Some(spring) = &foot.spring else { return fx };
        let (off, settled) = spring.at(now);
        foot.pull = (foot.to + off).clamp(0.0, top);
        self.dirty = true;
        // Held at the end, a spring that overshot has nothing left to show,
        // even while it reads as unsettled by a fraction of a pixel: a page
        // never quite gone is a page that never quite lets a finger through.
        if !settled && (foot.pull - foot.to).abs() >= 0.5 {
            return fx;
        }
        let (to, pull) = (foot.to, foot.pull);
        if to > 0.5 {
            // Up as far as it goes: from here the page folds into the icon
            // it came out of, the way going back from a page does. The fold
            // puts the foot away itself, and leaves it standing if work
            // that must not be cut short is under way.
            self.leave_by_foot(pull);
            return fx;
        }
        // Back down: the page shows alone again.
        self.foot = None;
        fx
    }
}
