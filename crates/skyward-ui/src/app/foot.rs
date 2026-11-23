//! The bar at the foot of a page: the ground it is made of — the page's own
//! colours, frosted — the shadow under it, and the pill that slides to the
//! face just touched.
//!
//! It is laid after everything else the page puts down, and outside the
//! page's canvas, for two reasons at once: the frost has to reach what lies
//! under the bar, and the bar has to stay put while the faces move. The
//! pages themselves never draw it.

use super::*;

/// What the frosted ground is made of: one pixel of the page in `STEP`
/// kept, softened `BLUR` pixels either side three times over, and laid back
/// at one of the two fills below.
const FOOT_STEP: u32 = 10;
const FOOT_BLUR: usize = 2;

/// How much of what lies under the bar reads through it. Above half, so the
/// bar takes its colour from the page it stands on and still lifts off it.
const FOOT_FILL_LIGHT: f32 = 0.82;
const FOOT_FILL_DARK: f32 = 0.84;

/// What the bar adds over the frost: the white a light page keeps, and the
/// black of a dark one. Thin either way — the frost is the bar's colour,
/// this only settles it.
const FOOT_VEIL_LIGHT: u8 = 78;
const FOOT_VEIL_DARK: u8 = 150;

/// The shadow under the bar: its deviation and how far down it is set, in
/// dp. Light enough to lift the bar off the page without reading as a line.
const FOOT_SHADOW_DP: f32 = 7.0;
const FOOT_SHADOW_DROP_DP: f32 = 2.5;

/// How long the pill takes to reach the face just touched, in ms: the page's
/// own turn, so a touch moves two things at one pace.
const FOOT_MS: u64 = FACE_MS;

/// Whether the bar frosts what lies under it. Off, it is a plain veil: the
/// same bar, a shade flatter, for a machine that cannot spare the blur.
const FOOT_FROST: bool = true;

/// The pill on its way from the face it was behind to the one just touched.
pub struct FootSlide {
    pub screen: Screen,
    pub from: f32,
    pub to: f32,
    pub since: u64,
}

impl App {
    /// Which cell the pill stands at `now`, counted from the left: a whole
    /// number for the face shown, a fraction while it is on its way — with
    /// the finger while the faces are pulled, and on its own once one was
    /// touched.
    pub(super) fn foot_at(&self, now: u64, shown: usize) -> f32 {
        // Under a finger the pill goes with the faces, so the bar says which
        // face is coming in before the finger has let go.
        if let Some(swipe) = self.face_swipe.as_ref() {
            let along = (swipe.shift.abs() / self.w.max(1) as f32).clamp(0.0, 1.0);
            return swipe.from as f32 + (swipe.to as f32 - swipe.from as f32) * along;
        }
        match &self.foot_slide {
            Some(s) if s.screen == self.screen() && now < s.since + FOOT_MS => {
                let k = dialog::eased(now.saturating_sub(s.since), FOOT_MS);
                s.from + (s.to - s.from) * k
            }
            _ => shown as f32,
        }
    }

    /// Whether the pill is still on its way somewhere.
    pub(super) fn foot_moving(&self, now: u64) -> bool {
        self.foot_slide.as_ref().is_some_and(|s| now <= s.since + FOOT_MS)
    }

    /// Send the pill from the face it was behind to `to`.
    pub(super) fn slide_foot(&mut self, now: u64, from: usize, to: usize) {
        if from == to {
            return;
        }
        self.foot_slide = Some(FootSlide { screen: self.screen(), from: from as f32, to: to as f32, since: now });
    }

    /// Lay the bar at the foot of the page, if it has one: the shadow under
    /// it, its frosted ground, its rim, and the faces it names. The page is
    /// drawn by the time this runs, which is what the frost is taken from.
    pub(super) fn paint_foot(&mut self, out: &mut Canvas, t: &Theme, f: &Fonts, now: u64) {
        let Some((faces, shown)) = self.frame.as_ref().and_then(|fr| fr.foot.as_ref()).map(|ft| (ft.faces, ft.shown)) else {
            return;
        };
        let count = faces.len();
        if count == 0 {
            return;
        }
        let geom = widgets::foot_geom(t, count, self.foot_at(now, shown));
        let (bar, r) = (geom.bar, geom.bar.h / 2.0);

        // The shadow first and set a little down, so what it darkens is the
        // page around the bar rather than the bar.
        out.shadow_round_rect(bar.x, bar.y + t.m.dp(FOOT_SHADOW_DROP_DP), bar.w, bar.h, r, t.m.dp(FOOT_SHADOW_DP), t.pal.shadow);
        if FOOT_FROST {
            let fill = if t.dark { FOOT_FILL_DARK } else { FOOT_FILL_LIGHT };
            out.frost_round_rect(bar.x, bar.y, bar.w, bar.h, r, FOOT_STEP, FOOT_BLUR, fill);
        }
        // The bar's own colour over the frost: a light page's white, a dark
        // one's black, thin enough for the frost to still be what is seen.
        let veil = if t.dark {
            Rgba::rgb(0, 0, 0).with_alpha(FOOT_VEIL_DARK)
        } else {
            Rgba::rgb(255, 255, 255).with_alpha(FOOT_VEIL_LIGHT)
        };
        out.fill_round_rect(bar.x, bar.y, bar.w, bar.h, r, veil);
        // The hairline rim every round thing in the system carries; in the
        // light it is the white of the fill and hardly shows, which is what
        // a bar on a grey page wants.
        let rim = if t.dark { Rgba::rgb(62, 62, 62).with_alpha(160) } else { Rgba::rgb(255, 255, 255) };
        out.stroke_round_rect(bar.x, bar.y, bar.w, bar.h, r, t.m.dp(0.8), rim);

        let l = self.state.lang;
        let marks: Vec<(crate::icons::Icon, &str)> = faces.iter().map(|fc| (fc.icon, l.pick(fc.name))).collect();
        widgets::foot_marks(out, t, f, &geom.cells, &marks, shown, geom.pill);
    }
}
