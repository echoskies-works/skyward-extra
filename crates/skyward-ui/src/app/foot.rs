//! The bar at the foot of a page: the translucent ground it is made of — the
//! page's own colour with a white or a black laid over it — and the pill that
//! slides to the face just touched.
//!
//! It is laid after everything else the page puts down, and outside the
//! page's canvas, for two reasons at once: its translucency has to go over
//! what lies under the bar, and the bar has to stay put while the faces move.
//! The pages themselves never draw it.

use super::*;

/// What the frosted ground is made of, read only while `FOOT_FROST` is on:
/// one pixel of the page in `STEP` kept, softened `BLUR` pixels either side
/// three times over, and laid back at one of the two fills below.
const FOOT_STEP: u32 = 10;
const FOOT_BLUR: usize = 2;

/// How much of the frosted page the bar keeps, again only while `FOOT_FROST`
/// is on. Above half, so the bar takes its colour from the page it stands on
/// and still lifts off it.
const FOOT_FILL_LIGHT: f32 = 0.82;
const FOOT_FILL_DARK: f32 = 0.84;

/// What the bar adds over the page: white, at the amount that lifts it off
/// the ground it stands on. A light page has little room above it, so the bar
/// is only a shade lighter than the page; a dark one has none at all — black
/// over black leaves nothing to see, so the bar is raised to a card's grey
/// instead. It is laid as alpha, so whatever is under the bar still shows
/// through it.
const FOOT_VEIL_LIGHT: u8 = 150;
const FOOT_VEIL_DARK: u8 = 28;

/// The shadow under the bar, in dp — the dial's shadow, not a halo: the light
/// comes from above, so it is at its darkest under the bar's foot, thin at the
/// sides, and gone above it, where the bar is between the light and the page.
/// `DIAL_SIGMA` is 11 dp on a disc 212 across; the bar is 52 high, so its fade
/// is taken at about that share of its own height.
const FOOT_SHADOW_DP: f32 = 6.0;

/// How dark that shadow is, as an alpha of black. The palette's own shadow is
/// too faint to be read under a surface this light — the dial takes a quarter
/// of black for the same reason, and this is taken at the dial's measure.
///
/// A dark page gets no shadow at all: the page is already black, and black
/// laid on black is not a shadow, it is nothing. The bar there stands off the
/// page by its own grey, the way the dial does.
const FOOT_SHADOW_INK: u8 = 60;

/// How long the pill takes to reach the face just touched, in ms: the page's
/// own turn, so a touch moves two things at one pace.
const FOOT_MS: u64 = FACE_MS;

/// Whether the bar frosts what lies under it. Off, it is a plain veil.
///
/// It is off: the store's faces are empty, so the bar stands on a page of one
/// colour and the frost comes back with that same colour — 243 in, 243 out,
/// 250 once the veil is over it, which is what the plain veil gives too. The
/// blur would be paid for on every frame of every slide and show nothing.
/// Under content it is a different matter: turn this back on when the shelves
/// are filled and the page scrolls under the bar with something to smudge.
///
/// Off does not mean opaque. The veil is white at 150/255 in the light and
/// 28/255 in the dark, laid over what is already drawn, so the page and
/// anything on it still read through the bar — just sharp.
const FOOT_FROST: bool = false;

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

    /// Lay the bar at the foot of the page, if it has one: its ground and the
    /// faces it names. The page is drawn by the time this runs, so the veil
    /// goes over the page itself — and a frost, were one turned on, would
    /// have something to take.
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

        if FOOT_FROST {
            let fill = if t.dark { FOOT_FILL_DARK } else { FOOT_FILL_LIGHT };
            out.frost_round_rect(bar.x, bar.y, bar.w, bar.h, r, FOOT_STEP, FOOT_BLUR, fill);
        }
        // The bar's own colour over the page: white in both themes, at the
        // amount above, and as alpha, so what lies under still reads through
        // the bar — sharp, since no frost is taken. Its edge is the shadow's,
        // not a stroke's: a lip around a bar this light reads as a button's
        // bevel, which is not what a surface lying over the page looks like.
        let veil = if t.dark { FOOT_VEIL_DARK } else { FOOT_VEIL_LIGHT };
        out.fill_round_rect(bar.x, bar.y, bar.w, bar.h, r, Rgba::rgb(255, 255, 255).with_alpha(veil));
        // The shadow last and outside the bar, so what it darkens is the page
        // around it and not the bar itself. The light is above, so it is at
        // its darkest under the foot, thin along the sides, and there is none
        // of it above — see `shadow_round_rect_lit`.
        if !t.dark {
            let ink = Rgba::rgb(0, 0, 0).with_alpha(FOOT_SHADOW_INK);
            out.shadow_round_rect_lit(bar.x, bar.y, bar.w, bar.h, r, t.m.dp(FOOT_SHADOW_DP), ink);
        }

        let l = self.state.lang;
        let marks: Vec<(crate::icons::Icon, &str)> = faces.iter().map(|fc| (fc.icon, l.pick(fc.name))).collect();
        widgets::foot_marks(out, t, f, &geom.cells, &marks, shown, geom.pill);
    }
}
