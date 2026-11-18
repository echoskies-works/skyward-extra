//! The sheet that rises from the foot of the panel when something has to be
//! said and answered: a dimmed page behind one card, a line of words and one
//! or two keys. The timer's end is the first to use it; the shape belongs to
//! no one page, so a later notice can borrow it unchanged.

use super::*;

/// How long the card takes to rise and to fall back to the foot.
pub(super) const RISE_MS: u64 = 240;
pub(super) const FALL_MS: u64 = 180;
/// What is left of the page under the card: the stock sheet darkens a white
/// page to 179 of 255, rather than softening it as the power menu does.
const DIM: f32 = 0.70;

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
const BTN_GAP: f32 = 61.0;
const BTN_H: f32 = 53.0;
const BTN_BOTTOM: f32 = 20.0;
const BTN_BETWEEN: f32 = 12.0;

fn eased(ms: u64, span: u64) -> f32 {
    1.0 - (1.0 - (ms as f32 / span as f32).min(1.0)).powi(2)
}

/// One sheet on the panel: what it says, the keys it offers and when it came
/// up. `closing` is set when a key is pressed; the sheet stays until it has
/// fallen back to the foot, so the page behind comes up with it.
pub(super) struct Sheet {
    pub(super) title: Text,
    pub(super) body: String,
    pub(super) buttons: Vec<(Text, Action)>,
    since: u64,
    pub(super) closing: Option<u64>,
}

impl Sheet {
    pub(super) fn new(title: Text, body: String, buttons: Vec<(Text, Action)>, now: u64) -> Self {
        Self { title, body, buttons, since: now, closing: None }
    }

    /// How far the sheet is up: none at the foot, all in its place.
    fn shown(&self, now: u64) -> f32 {
        match self.closing {
            Some(at) => 1.0 - eased(now.saturating_sub(at), FALL_MS),
            None => eased(now.saturating_sub(self.since), RISE_MS),
        }
    }

    fn done(&self, now: u64) -> bool {
        self.closing.is_some_and(|at| now >= at + FALL_MS)
    }

    /// When the rise or the fall ends, so the panel knows to draw again.
    pub(super) fn deadline(&self, now: u64) -> Option<u64> {
        if let Some(at) = self.closing {
            return (now < at + FALL_MS).then_some(at + FALL_MS);
        }
        (now < self.since + RISE_MS).then_some(self.since + RISE_MS)
    }

    /// The card's place, its wrapped lines and its keys, all before the rise
    /// is added: `out` is only read to measure the words.
    fn layout(&self, out: &Canvas, t: &Theme, f: &Fonts) -> (Rect, Vec<String>, Vec<Rect>) {
        let m = &t.m;
        let w = m.w as f32 - 2.0 * m.dp(SIDE);
        let x = m.dp(SIDE);
        let lines = crate::widgets::wrap(out, &f.label, &self.body, m.dp(BODY_PX), w - 2.0 * m.dp(PAD_X));
        let n = lines.len().max(1) as f32;
        let h = m.dp(BODY_CY + (n - 1.0) * LINE + BTN_GAP + BTN_H + BTN_BOTTOM);
        let y = m.h as f32 - m.dp(FOOT) - h;
        let key_y = y + m.dp(BODY_CY + (n - 1.0) * LINE + BTN_GAP);
        let inner = w - 2.0 * m.dp(PAD_X);
        let keys = if self.buttons.len() >= 2 {
            let each = (inner - m.dp(BTN_BETWEEN)) / 2.0;
            vec![
                Rect::new(x + m.dp(PAD_X), key_y, each, m.dp(BTN_H)),
                Rect::new(x + m.dp(PAD_X) + each + m.dp(BTN_BETWEEN), key_y, each, m.dp(BTN_H)),
            ]
        } else {
            vec![Rect::new(x + m.dp(PAD_X), key_y, inner, m.dp(BTN_H))]
        };
        (Rect::new(x, y, w, h), lines, keys)
    }

    /// The word a key answers, or none when the touch is off the card's keys.
    fn touched(&self, t: &Theme, f: &Fonts, x: f32, y: f32, shown: f32) -> Option<Action> {
        let m = &t.m;
        // Only the measure of the words is needed here; a canvas of one
        // pixel answers `text_width` the same as the panel's own.
        let probe = Canvas::new(1, 1);
        let (_, _, keys) = self.layout(&probe, t, f);
        let off = (1.0 - shown) * (m.h as f32 - m.dp(FOOT));
        keys.iter().zip(&self.buttons).find_map(|(rect, (_, action))| {
            Rect::new(rect.x, rect.y + off, rect.w, rect.h).contains(x, y).then_some(*action)
        })
    }
}

impl App {
    /// Put a sheet over the page. The page keeps its ground: unlike the power
    /// menu there is no blurred copy to keep, only a darkening while it stands.
    pub(super) fn show_sheet(&mut self, title: Text, body: String, buttons: Vec<(Text, Action)>, now: u64) {
        self.sheet = Some(Sheet::new(title, body, buttons, now));
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
    /// does that key's work and sends the sheet back down.
    pub(super) fn sheet_touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> Vec<Effect> {
        let Touch::Down(x, y) = ev else { return Vec::new() };
        let t = self.theme();
        let shown = self.sheet.as_ref().map_or(0.0, |s| s.shown(now));
        let Some(action) = self.sheet.as_ref().and_then(|s| s.touched(&t, f, x, y, shown)) else {
            return Vec::new();
        };
        self.act(action, now)
    }

    /// Back and power answer the sheet's first key, as they answer the
    /// timer's 确定 today; other keys do nothing while the card stands.
    pub(super) fn sheet_key(&mut self, key: Key, now: u64) -> Vec<Effect> {
        if !matches!(key, Key::Back | Key::Power) {
            return Vec::new();
        }
        let Some(action) = self.sheet.as_ref().and_then(|s| s.buttons.first().map(|(_, a)| *a)) else {
            return Vec::new();
        };
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
        let m = &t.m;
        out.dim(1.0 - (1.0 - DIM) * shown);
        let (card, lines, keys) = sheet.layout(out, &t, f);
        let off = (1.0 - shown) * (m.h as f32 - m.dp(FOOT));
        let y = card.y + off;
        out.shadow_round_rect(card.x, y, card.w, card.h, m.dp(RADIUS), m.dp(16.0), t.pal.shadow);
        out.fill_round_rect(card.x, y, card.w, card.h, m.dp(RADIUS), t.pal.card);
        let cx = card.x + card.w / 2.0;
        out.text_center(&f.heading, self.state.lang.pick(sheet.title), cx, y + m.dp(TITLE_CY), m.dp(TITLE_PX), t.pal.text);
        for (i, line) in lines.iter().enumerate() {
            out.text_top(&f.label, line, card.x + m.dp(PAD_X), y + m.dp(BODY_CY + i as f32 * LINE), m.dp(BODY_PX), t.pal.subtitle, None);
        }
        for (rect, (label, _)) in keys.iter().zip(&sheet.buttons) {
            let at = Rect::new(rect.x, rect.y + off, rect.w, rect.h);
            out.fill_round_rect(at.x, at.y, at.w, at.h, m.dp(BTN_H / 2.0), t.pal.track);
            out.text_center(&f.label, self.state.lang.pick(*label), at.x + at.w / 2.0, at.y + at.h / 2.0, m.row_title_px, t.pal.text);
        }
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
    const TITLE: Text = ["一声", "一聲", "A word"];
    const OK: Text = ["确定", "確定", "OK"];

    fn sheet(app: &mut App, body: &str, now: u64) {
        app.show_sheet(TITLE, body.to_string(), vec![(OK, Action::TimerDoneOk)], now);
    }

    /// At its place the card sits near the foot with a margin at either
    /// side; on the first frame it is a whole card's height lower.
    #[test]
    fn the_card_rises_from_the_foot_and_sits_inside_the_panel() {
        let f = fonts();
        let i = info();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        sheet(&mut app, "时长为1分钟的计时已结束", 0);
        let s = app.sheet.as_ref().unwrap();
        let (card, _, keys) = s.layout(&Canvas::new(1, 1), &t, &f);
        assert_eq!(card.x, t.m.dp(SIDE), "the stock sheet's side margin");
        assert_eq!(card.x + card.w, W as f32 - t.m.dp(SIDE));
        assert!(card.y + card.h < H as f32, "the card leaves the gesture bar clear");
        assert_eq!(keys.len(), 1, "one key");
        assert!(s.shown(0) < 0.01 && s.shown(RISE_MS) == 1.0, "it rises over its time");

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
        sheet(&mut app, "时长为1分钟的计时已结束", 0);
        let (_, _, keys) = app.sheet.as_ref().unwrap().layout(&Canvas::new(1, 1), &t, &f);
        let key = keys[0];
        let at = (key.x + key.w / 2.0, key.y + key.h / 2.0);

        app.touch(Touch::Down(at.0, at.1), &f, RISE_MS + 1);
        assert!(app.sheet.as_ref().unwrap().closing.is_some(), "the key sends it back down");
        assert_eq!(app.sheet.as_ref().unwrap().shown(RISE_MS + 1), 1.0, "still all the way up that frame");
        assert!(app.sheet.as_ref().unwrap().shown(RISE_MS + 1 + FALL_MS / 2) < 0.5);
        app.tick(RISE_MS + 1 + FALL_MS);
        assert!(app.sheet.is_none(), "and then it is gone");
        let _ = i;
    }

    /// Two keys share the card evenly and neither falls under the other.
    #[test]
    fn two_keys_sit_side_by_side_with_a_gap() {
        let f = fonts();
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Clock, W, H, PPD);
        app.show_sheet(TITLE, "先说清楚再做".to_string(), vec![(OK, Action::TimerDoneOk), (OK, Action::ClockGo)], 0);
        let (card, _, keys) = app.sheet.as_ref().unwrap().layout(&Canvas::new(1, 1), &t, &f);
        assert_eq!(keys.len(), 2);
        assert!(keys[0].x + keys[0].w < keys[1].x, "a gap between them");
        assert!(keys[1].x + keys[1].w < card.x + card.w, "and a margin at the card's edge");
    }
}
