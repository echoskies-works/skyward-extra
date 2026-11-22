//! The sheets that rise from the foot of the panel when something has to be
//! said and answered: a darkened or softened page behind one card, a line of
//! words and rows of keys. The timer's end and the power menu both use the
//! card; they keep their own grounds — the timer darkens the page, the power
//! menu softens it — so the same shape reads as two different things.

use super::*;

/// How long the card takes to rise and to fall back to the foot.
pub(super) const RISE_MS: u64 = 240;
pub(super) const FALL_MS: u64 = 180;
/// What is left of a darkened page under the card: the stock sheet darkens a
/// white page to 179 of 255. The power menu keeps its own, softer ground.
pub(super) const DIM: f32 = 0.70;

// The stock sheet, measured on a 1080 by 2400 panel: 33 px at either side,
// a 44 dp top corner, its contents 30 dp in and the keys 20 dp off the foot.
const SIDE: f32 = 13.0;
const FOOT: f32 = 30.0;
const RADIUS: f32 = 40.0;
const PAD_X: f32 = 30.0;
const TITLE_CY: f32 = 38.0;
const TITLE_PX: f32 = 20.0;
const BODY_CY: f32 = 82.0;
const BODY_PX: f32 = 15.0;
const LINE: f32 = 23.0;
const BODY_TO_KEYS: f32 = 61.0;
/// The keys start this far below the card's top when there is no body, as
/// on the power menu, which is only a title and a stack of whole-row keys.
const KEYS_NO_BODY: f32 = 72.0;
const BTN_H: f32 = 53.0;
const BTN_BOTTOM: f32 = 20.0;
/// The gap between the two keys of one row, and between whole rows.
const BTN_BETWEEN: f32 = 12.0;
const BTN_ROW_GAP: f32 = 12.0;

/// How far a motion has come at `ms` into `span`, easing out as it goes: the
/// card's rise, and the page going black behind it on the way down.
pub(super) fn eased(ms: u64, span: u64) -> f32 {
    1.0 - (1.0 - (ms as f32 / span as f32).min(1.0)).powi(2)
}

/// How far a thing has come down at `ms` into `span`, from rest and gathering
/// pace as it falls. The card goes back to the foot this way rather than
/// easing out: it stays in sight while the page behind it goes black, and
/// only leaves the panel when the black is nearly whole.
fn fell(ms: u64, span: u64) -> f32 {
    let t = (ms as f32 / span as f32).min(1.0);
    t * t
}

/// A sheet's keys, a row at a time: one key fills the row, two share it.
/// The power menu is four whole rows; a confirmation is one row of two with
/// whole rows of other ways out beneath it.
pub(super) type SheetRows = Vec<Vec<(Text, Action)>>;

/// Where everything on the card sits, before the rise is added.
pub(super) struct SheetGeom {
    pub card: Rect,
    pub lines: Vec<String>,
    pub rows: Vec<Vec<Rect>>,
}

/// Lay out a card for `body` and `rows`, shared by the dark sheet and the
/// soft power menu so the two read the same. `out` is only used to measure
/// words; a pixel-sized canvas answers the same as the panel's own.
pub(super) fn sheet_geom(t: &Theme, f: &Fonts, body: &str, rows: &SheetRows) -> SheetGeom {
    let m = &t.m;
    let w = m.w as f32 - 2.0 * m.dp(SIDE);
    let x = m.dp(SIDE);
    let probe = Canvas::new(1, 1);
    let lines = crate::widgets::wrap(&probe, &f.label, body, m.dp(BODY_PX), w - 2.0 * m.dp(PAD_X));
    let keys_top = if lines.is_empty() {
        m.dp(KEYS_NO_BODY)
    } else {
        m.dp(BODY_CY + (lines.len() - 1) as f32 * LINE + BODY_TO_KEYS)
    };
    let count = rows.len().max(1);
    let keys_h = count as f32 * m.dp(BTN_H) + (count as f32 - 1.0) * m.dp(BTN_ROW_GAP);
    let h = keys_top + keys_h + m.dp(BTN_BOTTOM);
    let y = m.h as f32 - m.dp(FOOT) - h;
    let inner = w - 2.0 * m.dp(PAD_X);
    let rects = rows.iter().enumerate().map(|(ri, row)| {
        let ry = y + keys_top + ri as f32 * (m.dp(BTN_H) + m.dp(BTN_ROW_GAP));
        if row.len() >= 2 {
            let each = (inner - m.dp(BTN_BETWEEN)) / 2.0;
            vec![
                Rect::new(x + m.dp(PAD_X), ry, each, m.dp(BTN_H)),
                Rect::new(x + m.dp(PAD_X) + each + m.dp(BTN_BETWEEN), ry, each, m.dp(BTN_H)),
            ]
        } else {
            vec![Rect::new(x + m.dp(PAD_X), ry, inner, m.dp(BTN_H))]
        }
    }).collect();
    SheetGeom { card: Rect::new(x, y, w, h), lines, rows: rects }
}

/// Draw the card, its words and its keys at `off` below their resting place.
/// The ground under it — dimmed or softened — is the caller's to lay first,
/// so the same card can stand over either without knowing which.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_sheet(
    out: &mut Canvas, t: &Theme, f: &Fonts, lang: crate::lang::Lang,
    title: Text, rows: &SheetRows, geom: &SheetGeom, off: f32, chosen: Option<usize>,
) {
    let m = &t.m;
    let y = geom.card.y + off;
    out.shadow_round_rect(geom.card.x, y, geom.card.w, geom.card.h, m.dp(RADIUS), m.dp(16.0), t.pal.shadow);
    out.fill_round_rect(geom.card.x, y, geom.card.w, geom.card.h, m.dp(RADIUS), t.pal.card);
    let cx = geom.card.x + geom.card.w / 2.0;
    out.text_center(&f.heading, lang.pick(title), cx, y + m.dp(TITLE_CY), m.dp(TITLE_PX), t.pal.text);
    for (i, line) in geom.lines.iter().enumerate() {
        out.text_top(&f.label, line, geom.card.x + m.dp(PAD_X), y + m.dp(BODY_CY + i as f32 * LINE), m.dp(BODY_PX), t.pal.subtitle, None);
    }
    let mut k = 0usize;
    for (rects, labels) in geom.rows.iter().zip(rows.iter()) {
        for (rect, (label, _)) in rects.iter().zip(labels.iter()) {
            let at = Rect::new(rect.x, rect.y + off, rect.w, rect.h);
            let ink = if chosen == Some(k) { t.pal.row_press } else { t.pal.track };
            out.fill_round_rect(at.x, at.y, at.w, at.h, m.dp(BTN_H / 2.0), ink);
            out.text_center(&f.label, lang.pick(*label), at.x + at.w / 2.0, at.y + at.h / 2.0, m.row_title_px, t.pal.text);
            k += 1;
        }
    }
}

/// The rise of a sheet, as a share: none at the foot, all in its place. It
/// comes up easing out and goes back down from rest, so a fall it shares
/// with the page going black keeps the card in sight a moment longer.
pub(super) fn shown_sheet(since: u64, closing: Option<u64>, now: u64) -> f32 {
    match closing {
        Some(at) => 1.0 - fell(now.saturating_sub(at), FALL_MS),
        None => eased(now.saturating_sub(since), RISE_MS),
    }
}

/// How far below their resting place the card and its keys sit while the
/// sheet is on its way up or down. A finger landing early finds them there.
pub(super) fn sheet_off(t: &Theme, shown: f32) -> f32 {
    (1.0 - shown) * (t.m.h as f32 - t.m.dp(FOOT))
}

/// The action of the key under (x, y), if any, on a card shown by `shown`.
pub(super) fn sheet_hit(t: &Theme, f: &Fonts, rows: &SheetRows, body: &str, shown: f32, x: f32, y: f32) -> Option<Action> {
    let geom = sheet_geom(t, f, body, rows);
    let off = sheet_off(t, shown);
    for (rects, labels) in geom.rows.iter().zip(rows.iter()) {
        for (rect, (_, action)) in rects.iter().zip(labels.iter()) {
            if Rect::new(rect.x, rect.y + off, rect.w, rect.h).contains(x, y) {
                return Some(*action);
            }
        }
    }
    None
}

/// Whether (x, y) falls outside the card shown by `shown`.
pub(super) fn sheet_miss(t: &Theme, f: &Fonts, rows: &SheetRows, body: &str, shown: f32, x: f32, y: f32) -> bool {
    let geom = sheet_geom(t, f, body, rows);
    let off = sheet_off(t, shown);
    !Rect::new(geom.card.x, geom.card.y + off, geom.card.w, geom.card.h).contains(x, y)
}

/// One ordinary sheet: what it says, the keys it offers and when it came up.
/// `closing` is set when a key is pressed; the sheet stays until it has
/// fallen back to the foot, so the page behind comes up with it.
pub(super) struct Sheet {
    pub(super) title: Text,
    pub(super) body: String,
    pub(super) rows: SheetRows,
    since: u64,
    pub(super) closing: Option<u64>,
}

impl Sheet {
    pub(super) fn new(title: Text, body: String, rows: SheetRows, now: u64) -> Self {
        Self { title, body, rows, since: now, closing: None }
    }

    fn shown(&self, now: u64) -> f32 {
        shown_sheet(self.since, self.closing, now)
    }

    fn done(&self, now: u64) -> bool {
        self.closing.is_some_and(|at| now >= at + FALL_MS)
    }

    /// Whether the card is on its way up or down, so the panel has to be
    /// drawn again: its place is read from the clock, and nothing else
    /// would ask for the frames in between. The frame it lands on counts
    /// too — that is the one that settles it, and the card otherwise stays
    /// over the page to take touches nothing can see.
    pub(super) fn moving(&self, now: u64) -> bool {
        match self.closing {
            Some(at) => now <= at + FALL_MS,
            None => now <= self.since + RISE_MS,
        }
    }

    /// The word a key answers, or none when the touch is off a key.
    fn touched(&self, t: &Theme, f: &Fonts, x: f32, y: f32, shown: f32) -> Option<Action> {
        sheet_hit(t, f, &self.rows, &self.body, shown, x, y)
    }
}

impl App {
    /// Put a sheet over the page. The page keeps its ground: unlike the power
    /// menu there is no blurred copy to keep, only a darkening while it stands.
    pub(super) fn show_sheet(&mut self, title: Text, body: String, rows: SheetRows, now: u64) {
        self.sheet = Some(Sheet::new(title, body, rows, now));
        self.dirty = true;
        self.stir(now);
    }

    /// A key was pressed: let the sheet fall back, then let the page brighten.
    /// The caller does the work the key stands for.
    pub(super) fn close_sheet(&mut self, now: u64) {
        if let Some(sheet) = self.sheet.as_mut()
            && sheet.closing.is_none()
        {
            sheet.closing = Some(now);
            self.dirty = true;
        }
    }

    /// The sheet takes the finger while it stands; a touch on one of its keys
    /// closes it and does that key's work.
    pub(super) fn sheet_touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> Vec<Effect> {
        let Touch::Down(x, y) = ev else { return Vec::new() };
        let t = self.theme();
        let shown = self.sheet.as_ref().map_or(0.0, |s| s.shown(now));
        let Some(action) = self.sheet.as_ref().and_then(|s| s.touched(&t, f, x, y, shown)) else {
            return Vec::new();
        };
        self.close_sheet(now);
        self.act(action, now)
    }

    /// Back and power answer the sheet's first key, as they answer the
    /// timer's 确定 today; other keys do nothing while the card stands.
    pub(super) fn sheet_key(&mut self, key: Key, now: u64) -> Vec<Effect> {
        if !matches!(key, Key::Back | Key::Power) {
            return Vec::new();
        }
        let Some(action) = self.sheet.as_ref().and_then(|s| s.rows.first().and_then(|r| r.first()).map(|(_, a)| *a)) else {
            return Vec::new();
        };
        self.close_sheet(now);
        self.act(action, now)
    }

    /// The sheet over everything else: the page darkens as the card rises and
    /// brightens as it falls, without being blurred at any point.
    pub(super) fn compose_sheet(&mut self, out: &mut Canvas, f: &Fonts, now: u64) {
        let Some(sheet) = self.sheet.as_ref() else { return };
        let shown = sheet.shown(now);
        if shown <= 0.0 {
            return;
        }
        let t = self.theme();
        let lang = self.state.lang;
        // While the phone is being seen off, the black-out takes the ground
        // over from the card: the card falls as the page goes black, and the
        // card itself keeps its colour the whole way down.
        if self.power_exit.is_none() {
            out.dim(1.0 - (1.0 - DIM) * shown);
        }
        let geom = sheet_geom(&t, f, &sheet.body, &sheet.rows);
        let off = sheet_off(&t, shown);
        paint_sheet(out, &t, f, lang, sheet.title, &sheet.rows, &geom, off, None);
    }

    /// When the card has reached the foot again it is gone and the page is
    /// left as it was.
    pub(super) fn settle_sheet(&mut self, now: u64) {
        if self.sheet.as_ref().is_some_and(|s| s.done(now)) {
            self.sheet = None;
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    const W: u32 = 1080;
    const H: u32 = 2400;
    const PPD: f32 = 2.625;
    const TITLE: Text = tri("一声", "一聲", "A word");
    const OK: Text = tri("确定", "確定", "OK");

    fn geom(app: &App, t: &Theme, f: &Fonts) -> SheetGeom {
        let s = app.sheet.as_ref().unwrap();
        sheet_geom(t, f, &s.body, &s.rows)
    }

    /// At its place the card sits near the foot with a margin at either
    /// side; on the first frame it is a whole card's height lower.
    #[test]
    fn the_card_rises_from_the_foot_and_sits_inside_the_panel() {
        let f = fonts();
        let i = info();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.show_sheet(TITLE, "时长为1分钟的计时已结束".to_string(), vec![vec![(OK, Action::TimerDoneOk)]], 0);
        let g = geom(&app, &t, &f);
        assert_eq!(g.card.x, t.m.dp(SIDE), "the stock sheet's side margin");
        assert_eq!(g.card.x + g.card.w, W as f32 - t.m.dp(SIDE));
        assert!(g.card.y + g.card.h < H as f32, "the card leaves the gesture bar clear");
        assert_eq!(g.rows.len(), 1, "one row");
        assert_eq!(g.rows[0].len(), 1, "one key in it");
        assert!(app.sheet.as_ref().unwrap().shown(0) < 0.01 && app.sheet.as_ref().unwrap().shown(RISE_MS) == 1.0, "it rises over its time");

        // The card's white reaches much lower once it has risen.
        let mut ink = |now| {
            let mut c = Canvas::new(W, H);
            app.compose(&mut c, &f, &i, "15:25", "11月18日 周二", now);
            grey_at(&c, W as f32 / 2.0, (H - 300) as f32)
        };
        assert!(ink(0) < 250, "the card has not got there yet");
        assert_eq!(ink(RISE_MS), 255, "and then it has");
    }

    /// A touch on the key does its work; the sheet stays for its fall and is
    /// gone once the fall is over, without waiting for another frame.
    #[test]
    fn its_key_answers_once_and_the_sheet_falls_back() {
        let f = fonts();
        let i = info();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.show_sheet(TITLE, "时长为1分钟的计时已结束".to_string(), vec![vec![(OK, Action::TimerDoneOk)]], 0);
        let g = geom(&app, &t, &f);
        let key = g.rows[0][0];
        let at = (key.x + key.w / 2.0, key.y + key.h / 2.0);

        app.touch(Touch::Down(at.0, at.1), &f, RISE_MS + 1);
        assert!(app.sheet.as_ref().unwrap().closing.is_some(), "the key sends it back down");
        assert_eq!(app.sheet.as_ref().unwrap().shown(RISE_MS + 1), 1.0, "still all the way up that frame");
        assert!(app.sheet.as_ref().unwrap().shown(RISE_MS + 1 + FALL_MS * 3 / 4) < 0.5);
        app.tick(RISE_MS + 1 + FALL_MS);
        assert!(app.sheet.is_none(), "and then it is gone");
        let _ = i;
    }

    /// Two keys share a row evenly and neither falls under the other.
    #[test]
    fn two_keys_sit_side_by_side_with_a_gap() {
        let f = fonts();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.show_sheet(TITLE, "先说清楚再做".to_string(), vec![vec![(OK, Action::TimerDoneOk), (OK, Action::ClockGo)]], 0);
        let g = geom(&app, &t, &f);
        assert_eq!(g.rows.len(), 1);
        assert_eq!(g.rows[0].len(), 2);
        assert!(g.rows[0][0].x + g.rows[0][0].w < g.rows[0][1].x, "a gap between them");
        assert!(g.rows[0][1].x + g.rows[0][1].w < g.card.x + g.card.w, "and a margin at the card's edge");
    }

    /// A confirmation's keys can stack: a row of two, then whole rows below,
    /// without any of them overlapping.
    #[test]
    fn keys_can_stack_as_rows_of_two_and_one() {
        let f = fonts();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.show_sheet(TITLE, "要继续吗？".to_string(), vec![
            vec![(OK, Action::TimerDoneOk), (OK, Action::SheetCancel)],
            vec![(OK, Action::RebootTo(crate::app::Target::Recovery))],
            vec![(OK, Action::RebootTo(crate::app::Target::Bootloader))],
        ], 0);
        let g = geom(&app, &t, &f);
        assert_eq!(g.rows.len(), 3, "three rows");
        assert_eq!(g.rows[0].len(), 2, "the first is a pair");
        assert_eq!(g.rows[1].len(), 1, "then whole rows");
        assert_eq!(g.rows[2].len(), 1);
        assert!(g.rows[0][0].y + g.rows[0][0].h < g.rows[1][0].y, "a gap between the rows");
        assert!(g.rows[1][0].y + g.rows[1][0].h < g.rows[2][0].y);
        assert!(g.rows[2][0].y + g.rows[2][0].h < g.card.y + g.card.h, "all inside the card");
    }
}
