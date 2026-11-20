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
/// How long a pull let go takes to carry the page off the screen. The lock's
/// own going away, to the second: a page leaving and the lock leaving are
/// the same motion, and should not run at two paces.
pub(super) const FOOT_AWAY: f32 = 0.52;
/// How long a pull takes to come back when it went nowhere. The lock's own
/// coming back, and quicker than going away: coming back reads as undoing
/// a move, not as a move of its own.
pub(super) const FOOT_BACK: f32 = 0.34;

/// A finger at the foot of a page, pulling it up off the screen so the
/// desktop comes in under it. Let go without a flick and it goes back where
/// it came from; flick it and the page is gone.
pub(super) struct Foot {
    /// How far the pull has come, in pixels the page has left by.
    pub(super) pull: f32,
    /// Where it comes to rest once the finger lifts: the screen's height
    /// for a page going, or nothing for it to go back.
    pub(super) to: f32,
    /// The spring carrying it there, once the finger has gone.
    pub(super) spring: Option<Spring>,
}

impl App {
    /// The desktop drawn on its own, status bar and all: what a page pulled
    /// off the screen leaves showing.
    pub(super) fn desktop_canvas(&mut self, f: &Fonts, info: &DeviceInfo, clock: &str) -> Canvas {
        let t = self.theme();
        let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: None, now: self.now, wallpaper: self.wallpaper.as_ref() };
        let mut c = pages::render(Screen::Desktop, &cx).canvas;
        let bar = self.bar(clock, info);
        widgets::status_bar(&mut c, &t, f, &bar, pages::DESKTOP_STATUS, true);
        c
    }

    /// Carry a pull let go on to where it goes: the page off the screen
    /// with the desktop left, or back where it came from.
    pub(super) fn settle_foot(&mut self, now: u64) -> Vec<Effect> {
        let fx = Vec::new();
        let h = self.h as f32;
        let Some(foot) = self.foot.as_mut() else { return fx };
        let Some(spring) = &foot.spring else { return fx };
        let (off, settled) = spring.at(now);
        foot.pull = (foot.to + off).clamp(0.0, h);
        self.dirty = true;
        // Held at the end, a spring that overshot has nothing left to show,
        // even while it reads as unsettled by a fraction of a pixel: a page
        // never quite gone is a page that never quite lets a finger through.
        if !settled && (foot.pull - foot.to).abs() >= 0.5 {
            return fx;
        }
        let to = foot.to;
        if to <= 0.5 {
            // Back down: the page shows alone again.
            self.foot = None;
            return fx;
        }
        // The screen's height: the page is gone, and the desktop is what it
        // leaves for, whatever lay between them.
        self.foot = None;
        self.leave_for_desktop();
        fx
    }
}
