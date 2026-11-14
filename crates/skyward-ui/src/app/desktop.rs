//! The desktop the phone starts on, and the pull from the foot of the
//! screen: the tasks up over it, and a page taken off for the desktop
//! again. The desktop's own page is drawn in `pages`.

use super::*;

/// How tall, in dp, the strip at the foot of the screen a finger lands in
/// to pull the page off, or the desktop's tasks up: the strip a phone
/// keeps for its gesture bar.
pub(super) const FOOT_DP: f32 = 24.0;
/// How far up, in dp, the tasks stand when they are up: the strip they take
/// at the foot of the screen, and how far a page is left standing above it.
/// Enough for the row of them, no more: they are icons on the desktop's own
/// black, not a card raised over it.
pub(super) const TASKS_DP: f32 = 120.0;
/// How far up, as a share of the tasks' height, a pull has to come to keep
/// them. It is also where the motor taps and where the page starts to dim,
/// so the tap and the dimming fall at the same moment: nothing of the tasks
/// shows until the pull is far enough to keep them.
pub(super) const TASKS_MARK: f32 = 0.5;
/// How dim the page goes while the tasks stand over it.
pub(super) const TASKS_DIM: f32 = 0.42;
/// The ink of what the tasks' strip says, taken from the dark theme's light
/// set: the desktop under it never turns light, so neither does this.
const TASKS_INK: Rgba = Rgba::rgb(150, 150, 150);
/// How far a pull let go carries on, in dp a second, to what it leads to.
pub(super) const FOOT_FLING_DP: f32 = 600.0;
/// How long a pull let go takes to settle, in seconds.
pub(super) const FOOT_PERIOD: f32 = 0.32;

/// A finger at the foot of the screen, pulling: on a page, the page up
/// off the screen so the desktop comes in under it; on the desktop, the
/// tasks up over it. Let go short of the mark, either goes back where it
/// came from; let go past it, the page is gone and the tasks stay up.
pub(super) struct Foot {
    /// How far the pull has come: pixels the page has left by, or pixels
    /// the tasks stand over the foot of the screen.
    pub(super) pull: f32,
    /// Where it comes to rest once the finger lifts: the screen's height
    /// for a page going, the tasks' height for the tasks staying, nothing
    /// for either to go back.
    pub(super) to: f32,
    /// The spring carrying it there, once the finger has gone.
    pub(super) spring: Option<Spring>,
    /// Whether the tasks are what is being pulled up, rather than a page
    /// pulled off.
    pub(super) tasks: bool,
    /// Whether the pull is past the mark, which it taps the motor on
    /// reaching.
    pub(super) armed: bool,
}

/// How far the tasks are up over what is shown, nought to one, for a pull
/// of `pull` against the height `stand` they come to rest at. They are
/// nought until the pull is far enough to keep them — where the motor taps
/// — all the way up at that height, and back down to nought as the page is
/// pushed on past it: a page on its way off the screen does not drag the
/// tasks up with it.
pub(super) fn tasks_up(pull: f32, stand: f32) -> f32 {
    let mark = stand * TASKS_MARK;
    if pull <= mark {
        return 0.0;
    }
    if pull > stand {
        return (1.0 - (pull - stand) / (stand * 0.4)).clamp(0.0, 1.0);
    }
    ((pull - mark) / (stand - mark)).clamp(0.0, 1.0)
}

impl App {
    /// The tasks up over the page: the page dimmed, and a line said at the
    /// foot of the screen where they will stand. Nothing is listed there
    /// yet, so the strip says so. No card is raised under it: the strip is
    /// the desktop's own black, come clear as the page is pulled off.
    ///
    /// Nothing of them shows until the pull is far enough to keep them,
    /// which is where the motor taps: short of that the page is only being
    /// felt, not put away. A page pushed past the height they stand at is on
    /// its way off the screen rather than coming to rest over it, so they
    /// give way with it instead of riding up on its back — a throw to the
    /// desktop shows nothing but the page and the desktop under it.
    pub(super) fn compose_tasks(&mut self, out: &mut Canvas, f: &Fonts) {
        let Some(foot) = self.foot.as_ref() else { return };
        let t = self.theme();
        let m = t.m;
        let stand = m.dp(TASKS_DP);
        // How far up they are, nought to one: nought where the motor taps,
        // all the way at their own height, and back down to nought as the
        // page goes on past it.
        let up = tasks_up(foot.pull, stand);
        if up <= 0.0 {
            return;
        }
        // The page goes as dim as the tasks are up over it. Over the desktop
        // there is no page lifted off it, so the whole screen darkens; a page
        // pulled off is darkened by its own lift, which dims what lies under
        // the page and leaves the page itself bright, so its rising is not
        // fought by a second darkening laid over the whole frame.
        if foot.tasks {
            out.dim(TASKS_DIM * up);
        }
        // The line comes in a little behind the dimming, so it is read on a
        // strip already dark enough to hold it.
        let shown = ((up - 0.2) / 0.8).clamp(0.0, 1.0);
        if shown <= 0.0 {
            return;
        }
        let said = self.state.lang.pick(["没有后台进行中", "沒有後台進行中", "Nothing running"]);
        let ink = TASKS_INK.with_alpha((shown * 255.0) as u8);
        let (w, h) = (m.w as f32, self.h as f32);
        out.text_center(&f.label, said, w / 2.0, h - stand / 2.0, m.sub_px, ink);
    }

    /// The desktop drawn on its own, status bar and all: what a page pulled
    /// off the screen leaves showing.
    pub(super) fn desktop_canvas(&mut self, f: &Fonts, info: &DeviceInfo, clock: &str) -> Canvas {
        let t = self.theme();
        let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: None, now: self.now };
        let mut c = pages::render(Screen::Desktop, &cx).canvas;
        let bar = self.bar(clock, info);
        widgets::status_bar(&mut c, &t, f, &bar, pages::DESKTOP_STATUS, true);
        c
    }

    /// Where the tasks stand, over the foot of the screen, while they are
    /// up: their top row, or nothing while they are not. They stand no
    /// higher than their own height, past which the page is going.
    pub(super) fn tasks_top(&self) -> Option<f32> {
        let foot = self.foot.as_ref()?;
        let pull = foot.pull.min(self.theme().m.dp(TASKS_DP));
        (pull > 0.5).then_some(self.h as f32 - pull)
    }

    /// Put the tasks down: they slide back into the foot of the screen and
    /// the desktop is left as it was.
    pub(super) fn lower_tasks(&mut self, now: u64) {
        if let Some(foot) = self.foot.as_mut() {
            foot.to = 0.0;
            foot.armed = false;
            foot.spring = Some(Spring::new(now, foot.pull, 0.0, FOOT_PERIOD));
        }
        self.dirty = true;
    }

    /// Carry a pull let go on to where it goes: the tasks up and standing,
    /// the page off the screen with the desktop left, or either back where
    /// it came from.
    pub(super) fn settle_foot(&mut self, now: u64) -> Vec<Effect> {
        let fx = Vec::new();
        let (h, tasks_h) = (self.h as f32, self.theme().m.dp(TASKS_DP));
        let Some(foot) = self.foot.as_mut() else { return fx };
        let Some(spring) = &foot.spring else { return fx };
        let (off, settled) = spring.at(now);
        let cap = if foot.tasks { tasks_h } else { h };
        foot.pull = (foot.to + off).clamp(0.0, cap);
        self.dirty = true;
        if !settled {
            return fx;
        }
        let to = foot.to;
        // Down: whatever was up shows alone again.
        if to <= 0.5 {
            self.foot = None;
            return fx;
        }
        // The tasks' height: they stay where they came to, over whatever
        // they were pulled up over, until a touch above them.
        if to < h - 0.5 {
            let foot = self.foot.as_mut().expect("a pull is under way");
            foot.pull = to;
            foot.spring = None;
            return fx;
        }
        // The screen's height: the page is gone, and the desktop is what it
        // leaves for, whatever lay between them.
        self.foot = None;
        self.leave_for_desktop();
        fx
    }
}
