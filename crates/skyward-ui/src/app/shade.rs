//! The panel a finger pulls down from the status bar: the control centre in
//! a solid card at the top, and the notices under it, both floating on the
//! page as it stands.
//!
//! It is an overlay, not a page: it stands over whatever is shown without
//! taking the page away, and the status bar stays where it is, whole, over
//! its top edge. What it is pulled to is how far down it stands — nought
//! put away, one all the way — and the finger moves it one for one, as the
//! lock is moved up.

use super::*;
use crate::icons::{self, Icon};
use crate::theme::Palette;
use crate::tr;

/// How far down a finger drags, as a share of the screen's height, for
/// letting go to leave the panel out rather than send it back.
pub(super) const SHADE_SWIPE: f32 = 0.30;
/// A drag quicker than this, in dp a second, leaves it out whatever the
/// share it reached: a flick reads as intent where a slow pull does not.
const SHADE_FLICK_DP: f32 = 800.0;
/// How long the panel takes to settle once the finger lifts.
const SHADE_PERIOD: f32 = 0.34;

/// The control centre is a card of the pages' own kind — the same white, the
/// same corners, the same room from the screen's edges as the notices below
/// it — and, like them, flat on the page: no shadow round either, the corners
/// alone saying where a card ends and the page begins. How far below the
/// status bar it floats, how much room it keeps from its own edges, and how
/// far its last row stands above its foot.
const CARD_TOP_DP: f32 = 12.0;
const CARD_PAD_DP: f32 = 12.0;
const CARD_FOOT_DP: f32 = 16.0;

const BRIGHT_H_DP: f32 = 16.0;
/// The brightness bar's top, and the keys' first row, down from the card's
/// own top rather than from the screen's.
const BRIGHT_TOP_DP: f32 = 22.0;
const SUN_DP: f32 = 18.0;
const SUN_GAP_DP: f32 = 10.0;
const DOT_DP: f32 = 9.0;
const TILE_GAP_DP: f32 = 10.0;
const TILE_TOP_DP: f32 = 56.0;
const TILE_R_DP: f32 = 22.0;
const TILE_GLYPH_DP: f32 = 26.0;
const TILE_LABEL_DP: f32 = 11.0;
const TILE_LABEL_GAP_DP: f32 = 8.0;
const TILE_LABEL_H_DP: f32 = 15.0;
/// The card's foot to the first notice's top.
const HEAD_FOOT_DP: f32 = 10.0;

const CARD_H_DP: f32 = 66.0;
const CARD_R_DP: f32 = 14.0;
const CARD_GAP_DP: f32 = 8.0;
const CARD_ICON_DP: f32 = 18.0;
const CARD_TITLE_DP: f32 = 13.0;
const CARD_BODY_DP: f32 = 12.0;
const CARD_ICON_X_DP: f32 = 18.0;
const CARD_WORDS_X_DP: f32 = 46.0;
const CARD_TITLE_Y_DP: f32 = 24.0;
const CARD_BODY_Y_DP: f32 = 44.0;
const CARD_BAR_H_DP: f32 = 3.0;
const BODY_TOP_DP: f32 = 10.0;
const CLEAR_H_DP: f32 = 30.0;
const CLEAR_GAP_DP: f32 = 8.0;

/// The eight keys of the control centre, in the order shown. Each has a
/// backend already in the tree: nothing here is a switch that only looks
/// like one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadeKey {
    Wlan,
    Theme,
    Mute,
    Vibrate,
    Shot,
    Sleep,
    Restart,
    PowerOff,
}

pub(super) const SHADE_KEYS: [(ShadeKey, Icon, Text); 8] = [
    (
        ShadeKey::Wlan,
        Icon::Wlan,
        tri("无线网络", "無線網路", "Wi-Fi"),
    ),
    (
        ShadeKey::Theme,
        Icon::Moon,
        tri("深色模式", "深色模式", "Dark mode"),
    ),
    (ShadeKey::Mute, Icon::Bell, tri("静音", "靜音", "Silence")),
    (
        ShadeKey::Vibrate,
        Icon::Phone,
        tri("振动", "振動", "Vibrate"),
    ),
    (
        ShadeKey::Shot,
        Icon::Screenshot,
        tri("截图", "截圖", "Screenshot"),
    ),
    (
        ShadeKey::Sleep,
        Icon::Lock,
        tri("熄屏", "熄屏", "Screen off"),
    ),
    (
        ShadeKey::Restart,
        Icon::Replay,
        tri("重启", "重啟", "Restart"),
    ),
    (
        ShadeKey::PowerOff,
        Icon::Power,
        tri("关机", "關機", "Power off"),
    ),
];

/// Where the panel's parts stand on this screen, in pixels.
#[derive(Clone, Debug)]
pub(super) struct ShadeGeom {
    /// The status bar's foot: where the panel begins.
    pub top: f32,
    /// The control centre's card, floating on the page: the same kind of
    /// card the notices below it stand in, the same room from the screen's
    /// edges.
    pub card: Rect,
    pub head_h: f32,
    pub body_h: f32,
    pub bright: Rect,
    pub sun: (f32, f32),
    pub tiles: Vec<(Rect, Rect)>,
    pub clear: Rect,
}

impl ShadeGeom {
    pub(super) fn panel_h(&self) -> f32 {
        self.head_h + self.body_h
    }
}

/// Where the panel stands: how far out it is, where it is going, and the
/// card and the notices drawn once and moved with the finger.
pub(super) struct Shade {
    /// How far the panel is out, nought put away to one all the way.
    pub(super) open: f32,
    /// Where it comes to rest once the finger lifts.
    pub(super) to: f32,
    /// The spring carrying it there.
    pub(super) spring: Option<Spring>,
    /// How far the notices are scrolled, in pixels.
    pub(super) scroll: f32,
    /// The control centre drawn once and kept, opaque: put down whole as
    /// the finger moves it, which is a copy rather than a mix.
    pub(super) head: Option<Canvas>,
    /// The notices drawn once and kept, see-through where no card stands:
    /// laid as a layer over the page, so the page shows through.
    pub(super) body: Option<Canvas>,
    /// The panel already mixed into the page it stands over, a row of it
    /// for every row the panel covers. The page under a panel that is out
    /// does not move, so the mixing is done once where the panel first
    /// shows and every frame of the pull copies the rows of it out: a pull
    /// is a fifth of a second of a finger, and mixing a screen of glass
    /// for each frame of it is the whole of what that costs.
    pub(super) blend: Option<Canvas>,
}

impl Shade {
    fn new() -> Shade {
        Shade {
            open: 0.0,
            to: 0.0,
            spring: None,
            scroll: 0.0,
            head: None,
            body: None,
            blend: None,
        }
    }
}

/// How long a notice that has just come stands over the page before it
/// slips back into the list, where it stays until a finger takes it.
const BANNER_HOLD_MS: u64 = 3_000;
/// How long it takes to slip down out of the bar, and to go back.
const BANNER_MOVE_MS: u64 = 180;

/// A notice that has just come, shown over the page for a few seconds: not
/// instead of the list it also goes into, but so that a page in hand does
/// not keep a person from knowing it came.
pub(super) struct Banner {
    pub notice: Notification,
    /// When it came.
    pub since: u64,
    /// The card drawn once and kept, moved as it slips down and back.
    card: Option<Canvas>,
}

impl Banner {
    /// A card for a notice that has just come.
    pub(super) fn new(notice: Notification, since: u64) -> Banner {
        Banner {
            notice,
            since,
            card: None,
        }
    }

    /// How much of it stands out: nought not yet or gone, one whole.
    fn rise(&self, now: u64) -> f32 {
        let t = now.saturating_sub(self.since);
        if t < BANNER_MOVE_MS {
            t as f32 / BANNER_MOVE_MS as f32
        } else if t < BANNER_HOLD_MS {
            1.0
        } else if t < BANNER_HOLD_MS + BANNER_MOVE_MS {
            1.0 - (t - BANNER_HOLD_MS) as f32 / BANNER_MOVE_MS as f32
        } else {
            0.0
        }
    }

    /// Whether it has come and gone, and may be let go.
    pub(super) fn gone(&self, now: u64) -> bool {
        now >= self.since + BANNER_HOLD_MS + BANNER_MOVE_MS
    }
}

impl App {
    /// Whether a notice that has just come may stand over the page: only on
    /// a lit panel with nothing over it that a card could cover — a lock, a
    /// sheet asking for a word, the power menu, the charging screen, or the
    /// panel itself, which already shows the list.
    pub(super) fn banner_allowed(&self, now: u64) -> bool {
        self.display_on
            && self.lock.is_none()
            && self.sheet.is_none()
            && self.power_menu.is_none()
            && self.power_exit.is_none()
            && self.charging.is_none()
            && !self.shade_shows(now)
    }

    /// The notice that has just come, over the page under the panel: it
    /// slips down out of the bar, stands a moment, and slips back into the
    /// list it also went into.
    pub(super) fn compose_banner(&mut self, out: &mut Canvas, f: &Fonts, now: u64) {
        if self.banner.is_none() {
            return;
        }
        let t = self.theme();
        let m = t.m;
        if self.banner.as_ref().is_some_and(|b| b.card.is_none()) {
            let w = m.w as f32 - 2.0 * m.margin;
            let mut card = Canvas::new(w.round() as u32, m.dp(CARD_H_DP).round() as u32);
            let notice = self.banner.as_ref().expect("just checked").notice.clone();
            notice_card(&mut card, &t, f, &notice, 0.0, 0.0, w);
            self.banner.as_mut().expect("just checked").card = Some(card);
        }
        let rise = self.banner.as_ref().map_or(0.0, |b| b.rise(now));
        if rise <= 0.0 {
            return;
        }
        let h = m.dp(CARD_H_DP);
        let y = m.status_cy * 2.0 - h * (1.0 - rise);
        if let Some(card) = self.banner.as_ref().and_then(|b| b.card.as_ref()) {
            out.lay(card, m.margin.round() as i32, y.round() as i32, 1.0);
        }
    }

    /// The panel, from a finger going down on the status bar: put away but
    /// for where the finger takes it.
    pub fn shade_down(&mut self) {
        if self.shade.is_none() {
            self.shade = Some(Shade::new());
        }
        if let Some(shade) = self.shade.as_mut() {
            shade.open = 0.0;
            shade.to = 0.0;
            shade.spring = None;
            shade.scroll = 0.0;
            shade.head = None;
            shade.body = None;
        }
        // A card standing over the page has said its piece: the panel
        // itself is the better place to read it now.
        self.banner = None;
        self.dirty = true;
    }

    /// Whether the panel stands out at all, and so takes a finger of its
    /// own rather than letting the page have it.
    pub(super) fn shade_shows(&self, now: u64) -> bool {
        self.shade.is_some() && self.shade_open(now) > 0.0
    }

    /// Set how far out the panel is, from a finger dragging it.
    pub fn shade_pull(&mut self, open: f32) {
        if let Some(shade) = self.shade.as_mut() {
            shade.open = open.clamp(0.0, 1.0);
            shade.to = shade.open;
            shade.spring = None;
        }
        self.dirty = true;
    }

    /// How far the notices are scrolled down the panel.
    pub(super) fn shade_scrolled(&self) -> f32 {
        self.shade.as_ref().map_or(0.0, |s| s.scroll)
    }

    /// Scroll the notices by `delta` pixels, as far as they go, and answer
    /// how far they went: the pull that asked for more than they had left is
    /// the pull that takes the panel away. The panel's own key stays where it
    /// is: only the notices move under it.
    fn shade_scroll_by(&mut self, delta: f32) -> f32 {
        let max = self.shade_max_scroll();
        let Some(shade) = self.shade.as_mut() else {
            return 0.0;
        };
        let before = shade.scroll;
        let next = (shade.scroll + delta).clamp(0.0, max);
        let moved = next - before;
        if moved.abs() < 0.5 {
            return 0.0;
        }
        shade.scroll = next;
        // The notices are drawn with their scroll in them: drawn again
        // rather than moved, since the cards come and go at the edges.
        shade.body = None;
        self.dirty = true;
        moved
    }

    /// How far the notices may be scrolled: all of them, less the room the
    /// panel has for them.
    fn shade_max_scroll(&self) -> f32 {
        let geom = self.shade_geom();
        let m = &self.theme().m;
        let pitch = m.dp(CARD_H_DP) + m.dp(CARD_GAP_DP);
        let content = m.dp(BODY_TOP_DP)
            + self.notifier.len() as f32 * pitch
            + m.dp(CLEAR_H_DP)
            + m.dp(CLEAR_GAP_DP);
        (content - geom.body_h).max(0.0)
    }

    /// Let go: take it out or send it back, by how far it came and how
    /// quickly, and settle it there.
    pub(super) fn shade_release(&mut self, now: u64, velocity: f32) {
        let flick = velocity.abs() > self.theme().m.dp(SHADE_FLICK_DP);
        let Some(shade) = self.shade.as_mut() else {
            return;
        };
        let out = if flick {
            velocity > 0.0
        } else {
            shade.open >= SHADE_SWIPE
        };
        let to = if out { 1.0 } else { 0.0 };
        shade.to = to;
        shade.spring = Some(Spring::new(now, shade.open - to, velocity, SHADE_PERIOD));
        self.dirty = true;
    }

    /// Where the panel is now, from its spring while it settles.
    pub(super) fn shade_open(&self, now: u64) -> f32 {
        match self.shade.as_ref() {
            None => 0.0,
            Some(shade) => match &shade.spring {
                None => shade.open,
                Some(spring) => {
                    let (x, settled) = spring.at(now);
                    if settled {
                        shade.to
                    } else {
                        (shade.to + x).clamp(0.0, 1.0)
                    }
                }
            },
        }
    }

    /// The panel's parts on this screen.
    pub(super) fn shade_geom(&self) -> ShadeGeom {
        shade_geom(&self.theme())
    }

    /// What the panel holds has changed, or its theme: drawn again next
    /// frame, rather than moved as it was.
    pub(super) fn shade_gone(&mut self) {
        if let Some(shade) = self.shade.as_mut() {
            shade.head = None;
            shade.body = None;
            shade.blend = None;
        }
    }

    /// The panel over the page: the control centre's card, and the notices
    /// under it, both on the page as it stands. Its top edge is the status
    /// bar's foot, so the bar stays whole over it however far it is pulled.
    pub(super) fn compose_shade(&mut self, out: &mut Canvas, f: &Fonts, now: u64) {
        if self.shade.is_none() {
            return;
        }
        let open = self.shade_open(now);
        if open <= 0.0 {
            return;
        }
        if self
            .shade
            .as_ref()
            .is_some_and(|s| s.head.is_none() || s.body.is_none())
        {
            self.shade_draw(f);
            if let Some(shade) = self.shade.as_mut() {
                shade.blend = None;
            }
        }
        let geom = self.shade_geom();
        // The panel hangs from the status bar's foot and grows down from it:
        // how much of it is out is how tall the window it shows through is.
        // Its parts stand where they always stand, so pulling it down shows
        // the top of it first — the card's own top edge, the brightness, the
        // keys — and the notices only once there is room for them.
        let top = geom.top.round() as i32;
        let seen = (geom.panel_h() * open).round() as i32;

        let want = seen.min(geom.panel_h() as i32);
        if want <= 0 || top >= out.h as i32 {
            return;
        }
        // The panel mixed into the page once, at the size it will be pulled
        // to. Laid rather than put: the card rounds away from its corners,
        // so the page has to show through where the card is not — and that
        // is the mix this keeps, so the frames after it are a copy.
        if self.shade.as_ref().is_some_and(|s| s.blend.is_none()) {
            let (head, body) = match self.shade.as_ref() {
                Some(s) => (s.head.as_ref(), s.body.as_ref()),
                None => (None, None),
            };
            let mut blend = Canvas::new(out.w, out.h);
            blend.put_rows(out, top, top + geom.panel_h() as i32, top);
            if let Some(head) = head {
                blend.lay_rows(head, 0, top, geom.head_h as i32, 1.0);
            }
            if let Some(body) = body {
                blend.lay_rows(body, 0, top + geom.head_h as i32, body.h as i32, 1.0);
            }
            if let Some(shade) = self.shade.as_mut() {
                shade.blend = Some(blend);
            }
        }
        if let Some(blend) = self.shade.as_ref().and_then(|s| s.blend.as_ref()) {
            out.put_rows(blend, top, top + want, top);
        }
    }

    /// Draw the panel's two parts whole onto canvases of their own, ready to
    /// be moved with the finger frame after frame without being drawn again.
    fn shade_draw(&mut self, f: &Fonts) {
        let t = self.theme();
        let m = &t.m;
        let lang = self.state.lang;
        let geom = shade_geom(&t);
        let pal = &t.pal;
        let top = geom.top;

        let w = m.w;
        let mut head = Canvas::new(w, geom.head_h as u32);
        // The card floats straight on the page, as the notices below do:
        // no sheet of dark under either, and no shadow around either. A veil
        // here showed as one dark rectangle on the wallpaper behind it, and a
        // shadow on a card that stands over its own page reads as a second
        // edge drawn round the first — the corners are the edge.
        let card = geom.card;
        head.fill_round_rect(
            card.x,
            card.y - top,
            card.w,
            card.h,
            m.card_radius,
            pal.card,
        );
        icons::glyph(
            &mut head,
            Icon::Sun,
            geom.sun.0,
            geom.sun.1 - top,
            m.dp(SUN_DP),
            pal.subtitle,
        );
        let b = geom.bright;
        let (bx, by, bw, bh) = (b.x, b.y - top, b.w, b.h);
        let level = self.state.brightness.clamp(0.0, 1.0);
        head.fill_round_rect(bx, by, bw, bh, bh / 2.0, pal.track);
        head.fill_round_rect(bx, by, bw * level, bh, bh / 2.0, pal.accent);
        head.fill_circle(
            bx + bw * level,
            by + bh / 2.0,
            m.dp(DOT_DP) / 2.0,
            Rgba::rgb(255, 255, 255),
        );

        for (i, (key, icon, label)) in SHADE_KEYS.iter().enumerate() {
            let (r, lr) = &geom.tiles[i];
            let on = self.shade_key_on(*key);
            let fill = if on { pal.accent } else { pal.track };
            let ink = if on {
                Rgba::rgb(255, 255, 255)
            } else {
                pal.subtitle
            };
            head.fill_round_rect(r.x, r.y - top, r.w, r.h, m.dp(TILE_R_DP), fill);
            icons::glyph(
                &mut head,
                *icon,
                r.x + r.w / 2.0,
                r.y - top + r.h / 2.0,
                m.dp(TILE_GLYPH_DP),
                ink,
            );
            head.text_center(
                &f.status,
                lang.pick(*label),
                lr.x + lr.w / 2.0,
                lr.y - top + lr.h / 2.0,
                m.dp(TILE_LABEL_DP),
                pal.subtitle,
            );
        }

        let mut body = Canvas::new(w, geom.body_h as u32);
        let body_top = top + geom.head_h;
        let margin = m.margin;
        let card_w = w as f32 - 2.0 * margin;
        let pitch = m.dp(CARD_H_DP) + m.dp(CARD_GAP_DP);
        let scroll = self.shade.as_ref().map_or(0.0, |s| s.scroll);
        let mut y = m.dp(BODY_TOP_DP) - scroll;

        if self.notifier.is_empty() {
            // On the page rather than a card, so the page's own ink would be
            // the wrong one in either theme: white, softer than a notice's
            // title, is what reads on the wallpaper behind it.
            body.text_center(
                &f.body,
                &tr!(lang, "暂无通知", "暫無通知", "No notifications"),
                w as f32 / 2.0,
                m.dp(BODY_TOP_DP) + m.dp(CARD_H_DP) / 2.0,
                m.dp(CARD_BODY_DP),
                Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 200,
                },
            );
        }
        for notice in self.notifier.iter() {
            if y + m.dp(CARD_H_DP) < 0.0 {
                y += pitch;
                continue;
            }
            if y > geom.body_h {
                break;
            }
            notice_card(&mut body, &t, f, notice, margin, y, card_w);
            y += pitch;
        }

        if !self.notifier.is_empty() {
            let c = &geom.clear;
            body.fill_round_rect(c.x, c.y - body_top, c.w, c.h, c.h / 2.0, pal.card);
            body.text_center(
                &f.value,
                &tr!(lang, "全部清除", "全部清除", "Clear all"),
                c.x + c.w / 2.0,
                c.y - body_top + c.h / 2.0,
                m.dp(CARD_BODY_DP),
                pal.subtitle,
            );
        }

        if let Some(shade) = self.shade.as_mut() {
            shade.head = Some(head);
            shade.body = Some(body);
        }
        self.dirty = true;
    }

    /// Whether the panel is still settling after the finger let it go.
    pub(super) fn shade_moving(&self, now: u64) -> bool {
        self.shade
            .as_ref()
            .and_then(|s| s.spring.as_ref())
            .is_some_and(|spring| !spring.at(now).1)
    }

    /// Settle the panel once its spring has come to rest: it stays where it
    /// went, and one sent all the way back is let go whole, so nothing is
    /// drawn or kept for it.
    pub(super) fn settle_shade(&mut self, now: u64) {
        let Some(shade) = self.shade.as_mut() else {
            return;
        };
        let Some(spring) = shade.spring.as_ref() else {
            return;
        };
        if !spring.at(now).1 {
            return;
        }
        shade.open = shade.to;
        shade.spring = None;
        if shade.to <= 0.0 {
            self.shade = None;
        }
        self.dirty = true;
    }

    /// Send the panel back, from wherever it stands.
    pub(super) fn shade_away(&mut self, now: u64) {
        let Some(shade) = self.shade.as_mut() else {
            return;
        };
        shade.to = 0.0;
        shade.spring = Some(Spring::new(now, shade.open, 0.0, SHADE_PERIOD));
        self.dirty = true;
    }

    /// A finger on the panel while it stands: its eight keys, its
    /// brightness, its notices, and the pull that puts it away again. What
    /// lies under it hears nothing of any of it.
    pub(super) fn shade_touch(&mut self, ev: Touch, now: u64) -> Vec<Effect> {
        let t = self.theme();
        let geom = self.shade_geom();
        let mut fx = Vec::new();
        match ev {
            Touch::Down(x, y) => {
                self.tracker.clear();
                self.tracker.add(now, y);
                self.stir(now);
                let open = self.shade_open(now);
                // Short of out whole, the panel covers only the top of the
                // screen: a finger below where it ends is a finger on the page
                // meant for putting it away, as one on the page of a sheet is.
                if open < 1.0 && y > geom.top + geom.panel_h() * open {
                    self.shade_away(now);
                    return fx;
                }
                // The bar the panel comes from still begins a pull with the
                // panel out: a finger there drags it on down or back up, and
                // nowhere else does a finger on the bar mean the list.
                let bar = y < geom.top;
                if bar {
                    self.drag = Drag::Shade { last: y, bar };
                    return fx;
                }
                if let Some(i) = geom.tiles.iter().position(|(r, _)| r.contains(x, y)) {
                    fx.extend(self.shade_key(SHADE_KEYS[i].0, now));
                    self.shade_gone();
                    return fx;
                }
                if geom.bright.contains(x, y) {
                    self.set_slider(&t, Slider::Brightness, geom.bright, x);
                    self.drag = Drag::Slider(Slider::Brightness, geom.bright);
                    return fx;
                }
                if !self.notifier.is_empty() && geom.clear.contains(x, y) {
                    self.notifier.clear();
                    self.shade_gone();
                    return fx;
                }
                if let Some(action) = self.shade_card_at(y) {
                    self.shade_away(now);
                    fx.extend(self.act(action, now));
                    return fx;
                }
                self.drag = Drag::Shade {
                    last: y,
                    bar: false,
                };
            }
            Touch::Move(x, y) => match std::mem::replace(&mut self.drag, Drag::Idle) {
                Drag::Shade { last, bar } => {
                    // Out whole, the panel is a list: a finger on it scrolls
                    // what it holds until there is no more of it, and a pull
                    // past that — or any pull from the bar itself — moves the
                    // panel. So the way it was pulled out is the way it goes
                    // back, and a list with no room left to scroll does not
                    // hold the panel out.
                    let pulled = y - last;
                    let open = self.shade_open(now);
                    let push = if bar {
                        pulled
                    } else if open >= 1.0 && (pulled < 0.0 || self.shade_scrolled() > 0.0) {
                        let want = -pulled;
                        let moved = self.shade_scroll_by(want);
                        -(want - moved)
                    } else {
                        pulled
                    };
                    if push != 0.0 {
                        self.shade_pull(open + push / geom.panel_h().max(1.0));
                    }
                    self.tracker.add(now, y);
                    self.drag = Drag::Shade { last: y, bar };
                }
                Drag::Slider(which, track) => {
                    self.set_slider(&t, which, track, x);
                    self.drag = Drag::Slider(which, track);
                }
                other => self.drag = other,
            },
            Touch::Up => {
                if let Drag::Shade { .. } = std::mem::replace(&mut self.drag, Drag::Idle) {
                    let speed = self.tracker.velocity(now);
                    // A finger lifted off a list it has scrolled leaves the
                    // panel out: a flick up there is meant for the list, not
                    // for putting the panel away.
                    if self.shade_open(now) >= 1.0 && self.shade_scrolled() > 0.0 {
                        self.shade_pull(1.0);
                    } else {
                        self.shade_release(now, speed);
                    }
                }
            }
        }
        fx
    }

    /// What a tap on a notice does: the page it points at, where it points
    /// at one. The notice itself stays, as nothing but a finger on its
    /// card's clear key or the phone going off takes one away.
    fn shade_card_at(&self, y: f32) -> Option<Action> {
        let m = &self.theme().m;
        let geom = self.shade_geom();
        let body_top = geom.top + geom.head_h;
        if y < body_top {
            return None;
        }
        let scroll = self.shade.as_ref().map_or(0.0, |s| s.scroll);
        let pitch = m.dp(CARD_H_DP) + m.dp(CARD_GAP_DP);
        let mut top = body_top + m.dp(BODY_TOP_DP) - scroll;
        for notice in self.notifier.iter() {
            if y >= top && y < top + m.dp(CARD_H_DP) {
                return notice.action;
            }
            top += pitch;
        }
        None
    }

    /// What one of the control centre's keys does. Every one of them has a
    /// backend already in the tree: none is a switch that only looks like
    /// one.
    fn shade_key(&mut self, key: ShadeKey, now: u64) -> Vec<Effect> {
        match key {
            ShadeKey::Wlan => self.act(Action::Toggle(Switch::Wlan), now),
            ShadeKey::Theme => {
                let dark = !self.state.dark;
                self.act(Action::Theme { dark }, now)
            }
            ShadeKey::Mute => self.act(Action::Toggle(Switch::Mute), now),
            ShadeKey::Vibrate => self.act(Action::Toggle(Switch::Vibrate), now),
            ShadeKey::Shot => {
                self.shade_away(now);
                vec![Effect::Screenshot { storage: false }]
            }
            ShadeKey::Sleep => {
                self.shade_away(now);
                self.sleep(now);
                Vec::new()
            }
            // The two that leave the phone ask for a sheet to confirm them,
            // as the power menu does: a tap here is a thumb's width from
            // its neighbour.
            ShadeKey::Restart => {
                self.shade_away(now);
                self.act(Action::PowerMenuChoice(power::PowerChoice::Restart), now)
            }
            ShadeKey::PowerOff => {
                self.shade_away(now);
                self.act(Action::PowerMenuChoice(power::PowerChoice::PowerOff), now)
            }
        }
    }

    /// Whether a key of the control centre stands on.
    fn shade_key_on(&self, key: ShadeKey) -> bool {
        match key {
            ShadeKey::Wlan => self
                .state
                .wlan
                .as_ref()
                .is_some_and(|w| w.power == Power::On),
            ShadeKey::Theme => self.state.dark,
            ShadeKey::Mute => self.state.muted,
            ShadeKey::Vibrate => self.state.vibrate,
            _ => false,
        }
    }
}

/// One notice as a card, its left edge at `x` and its top at `y`: the mark
/// it carries, its words, and the bar of work under way.
fn notice_card(
    out: &mut Canvas,
    t: &Theme,
    f: &Fonts,
    notice: &Notification,
    x: f32,
    y: f32,
    w: f32,
) {
    let m = &t.m;
    let pal = &t.pal;
    out.fill_round_rect(x, y, w, m.dp(CARD_H_DP), m.dp(CARD_R_DP), pal.card);
    let (icon, ink) = notice_mark(notice, pal);
    icons::glyph(
        out,
        icon,
        x + m.dp(CARD_ICON_X_DP),
        y + m.dp(CARD_TITLE_Y_DP),
        m.dp(CARD_ICON_DP),
        ink,
    );
    out.text(
        &f.value,
        &notice.title,
        x + m.dp(CARD_WORDS_X_DP),
        y + m.dp(CARD_TITLE_Y_DP) + m.dp(CARD_TITLE_DP) * 0.36,
        m.dp(CARD_TITLE_DP),
        pal.text,
        None,
    );
    out.text(
        &f.body,
        &notice.body,
        x + m.dp(CARD_WORDS_X_DP),
        y + m.dp(CARD_BODY_Y_DP) + m.dp(CARD_BODY_DP) * 0.36,
        m.dp(CARD_BODY_DP),
        pal.subtitle,
        None,
    );
    if let Some(level) = notice.level() {
        out.fill_round_rect(
            x + m.dp(CARD_WORDS_X_DP),
            y + m.dp(CARD_H_DP) - m.dp(10.0),
            (w - m.dp(CARD_WORDS_X_DP) - m.dp(CARD_ICON_X_DP)) * level,
            m.dp(CARD_BAR_H_DP),
            m.dp(CARD_BAR_H_DP) / 2.0,
            pal.accent,
        );
    }
}

/// The mark a notice carries, and the ink it is drawn in: an event in the
/// accent, work under way in it too, and what did not go in the danger ink.
fn notice_mark(notice: &Notification, pal: &Palette) -> (Icon, Rgba) {
    // The clock's notices name the app they come from, whatever shape they
    // take: a count still running wears the clock's own mark.
    if notice.source == Origin::App(CLOCK_APP) {
        return (Icon::Clock, pal.accent);
    }
    match notice.kind {
        Kind::Event => (Icon::Bell, pal.accent),
        Kind::Progress => (Icon::Install, pal.accent),
        Kind::Failure => (Icon::Close, pal.danger),
    }
}

fn shade_geom(t: &Theme) -> ShadeGeom {
    let m = &t.m;
    let top = m.status_cy * 2.0;
    let margin = m.margin;
    let card_x = margin;
    let card_w = m.w as f32 - 2.0 * margin;
    let card_top = top + m.dp(CARD_TOP_DP);
    // The card keeps its own room from its edges, so the keys sit a little
    // inside it rather than on its rim.
    let pad = m.dp(CARD_PAD_DP);
    let inner_x = card_x + pad;
    let inner_w = card_w - 2.0 * pad;
    let gap = m.dp(TILE_GAP_DP);
    let tile = (inner_w - 3.0 * gap) / 4.0;
    let pitch = tile + gap;
    let row1 = card_top + m.dp(TILE_TOP_DP);
    let row2 = row1 + tile + m.dp(TILE_LABEL_GAP_DP) + m.dp(TILE_LABEL_H_DP) + gap;
    let card_h = row2 + tile + m.dp(TILE_LABEL_GAP_DP) + m.dp(TILE_LABEL_H_DP) + m.dp(CARD_FOOT_DP)
        - card_top;
    let head_foot = card_top + card_h + m.dp(HEAD_FOOT_DP);
    let body_h = (m.h as f32 - head_foot).max(0.0);
    let bright_x = inner_x + m.dp(SUN_DP) + m.dp(SUN_GAP_DP);
    let bright_y = card_top + m.dp(BRIGHT_TOP_DP);
    let mut tiles = Vec::with_capacity(SHADE_KEYS.len());
    for (i, _) in SHADE_KEYS.iter().enumerate() {
        let (row, col) = (i / 4, i % 4);
        let x = inner_x + col as f32 * pitch;
        let y = if row == 0 { row1 } else { row2 };
        tiles.push((
            Rect::new(x, y, tile, tile),
            Rect::new(
                x,
                y + tile + m.dp(TILE_LABEL_GAP_DP),
                tile,
                m.dp(TILE_LABEL_H_DP),
            ),
        ));
    }
    ShadeGeom {
        top,
        card: Rect::new(card_x, card_top, card_w, card_h),
        head_h: head_foot - top,
        body_h,
        bright: Rect::new(
            bright_x,
            bright_y,
            inner_x + inner_w - bright_x,
            m.dp(BRIGHT_H_DP),
        ),
        sun: (
            inner_x + m.dp(SUN_DP) / 2.0,
            bright_y + m.dp(BRIGHT_H_DP) / 2.0,
        ),
        tiles,
        clear: Rect::new(
            margin,
            head_foot + body_h - m.dp(CLEAR_H_DP) - m.dp(CLEAR_GAP_DP),
            m.w as f32 - 2.0 * margin,
            m.dp(CLEAR_H_DP),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::joined;

    /// A phone with the panel already out, as if a finger had pulled it
    /// down and let go past the mark.
    fn pulled(app: &mut App, f: &Fonts) {
        app.shade_down();
        app.shade_pull(1.0);
        app.touch(Touch::Down(200.0, 4.0), f, 0);
        app.touch(Touch::Up, f, 10);
        app.tick(1_000);
    }

    /// A finger on the bar pulls the panel down with it, and let go past
    /// the mark the panel stays out.
    #[test]
    fn the_bar_pulls_the_panel_down_and_it_stays_past_the_mark() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        let top = app.shade_geom().top;
        let panel = app.shade_geom().panel_h();
        let from = top / 2.0;
        app.touch(Touch::Down(200.0, from), &f, 0);
        app.touch(Touch::Move(200.0, from + panel * 0.4), &f, 100);
        assert!(
            app.shade_open(100) > 0.0,
            "the panel comes along with the finger"
        );
        app.touch(Touch::Up, &f, 200);
        app.tick(1_000);
        assert_eq!(app.shade_open(1_000), 1.0, "past the mark it stands out");
    }

    /// A pull short of the mark sends it back, and it is let go whole: no
    /// canvas is kept for a panel that is not there.
    #[test]
    fn a_short_pull_goes_back_and_is_let_go() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        let top = app.shade_geom().top;
        let panel = app.shade_geom().panel_h();
        let from = top / 2.0;
        app.touch(Touch::Down(200.0, from), &f, 0);
        app.touch(Touch::Move(200.0, from + panel * 0.1), &f, 200);
        app.touch(Touch::Up, &f, 400);
        app.tick(2_000);
        assert!(app.shade.is_none(), "back it goes, and is let go whole");
    }

    /// The bar is the handle, not the whole screen: a finger on the page
    /// under it presses the page.
    #[test]
    fn a_finger_on_the_page_does_not_pull_the_panel() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.touch(Touch::Down(200.0, 600.0), &f, 0);
        assert!(
            !matches!(app.drag, Drag::Shade { .. }),
            "the page keeps the finger"
        );
        assert!(app.shade.is_none(), "and no panel came down");
    }

    /// The locked phone has no panel: what is not drawn is not touched.
    #[test]
    fn a_locked_phone_pulls_nothing_down() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.touch(Touch::Down(200.0, 4.0), &f, 0);
        assert!(
            app.shade.is_none(),
            "the bar of a locked phone begins nothing"
        );
    }

    /// The panel begins at the status bar's foot, so the bar stands whole
    /// over it however far it is pulled.
    #[test]
    fn the_panel_begins_under_the_status_bar() {
        let app = App::new(Screen::Desktop, 400, 800, 2.0);
        let t = app.theme();
        assert_eq!(
            app.shade_geom().top,
            t.m.status_cy * 2.0,
            "the bar keeps its own rows"
        );
    }

    /// A key of the control centre does what it says: the WLAN one turns
    /// the radio off.
    #[test]
    fn a_key_of_the_control_centre_turns_what_it_names() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.state.wlan = Some(joined("home", Joining::Joining));
        pulled(&mut app, &f);
        let (r, _) = app.shade_geom().tiles[0];
        let fx = app.touch(Touch::Down(r.x + r.w / 2.0, r.y + r.h / 2.0), &f, 1_100);
        assert_eq!(
            fx,
            vec![Effect::Wlan(false)],
            "the key asks for the radio to go off"
        );
        assert_eq!(
            app.state.wlan.as_ref().expect("still there").power,
            Power::Off
        );
    }

    /// One tap on the panel's own key takes every notice at once.
    #[test]
    fn clearing_takes_every_notice_at_once() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.notify(Notification::event(Origin::System, "a", "一号", "正文"));
        app.notify(Notification::event(Origin::System, "b", "二号", "正文"));
        assert_eq!(app.notifier.len(), 2, "two stand");
        pulled(&mut app, &f);
        let c = app.shade_geom().clear;
        app.touch(Touch::Down(c.x + c.w / 2.0, c.y + c.h / 2.0), &f, 1_100);
        assert!(app.notifier.is_empty(), "one tap takes them all");
    }

    /// What lies under the panel hears nothing of a finger on it.
    #[test]
    fn the_panel_keeps_the_page_under_it_from_the_finger() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        pulled(&mut app, &f);
        let before = app.screen();
        app.touch(Touch::Down(200.0, 700.0), &f, 1_100);
        app.touch(Touch::Up, &f, 1_200);
        app.tick(2_000);
        assert_eq!(
            app.screen(),
            before,
            "the page under the panel is left as it was"
        );
        assert_eq!(app.shade_open(2_000), 1.0, "and the panel stays out");
    }

    /// A notice that comes over a page stands there a moment, so a person
    /// in the middle of something still learns of it, and then slips back
    /// into the list it also went into.
    #[test]
    fn a_notice_that_comes_stands_over_the_page_and_then_goes() {
        let f = crate::font::bundled();
        let info = crate::app::testing::info();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.notify(Notification::event(
            Origin::System,
            "battery",
            "电量偏低",
            "剩 15%",
        ));
        assert!(app.banner.is_some(), "it stands over the page");
        assert_eq!(app.notifier.len(), 1, "and is in the list all the same");
        let mut shot = Canvas::new(400, 800);
        app.compose(
            &mut shot,
            &f,
            &info,
            "21:40",
            "11月16日 周一",
            BANNER_MOVE_MS,
        );
        app.tick(BANNER_HOLD_MS + BANNER_MOVE_MS + 1);
        assert!(app.banner.is_none(), "and then it is gone");
        assert_eq!(app.notifier.len(), 1, "leaving the notice where it stands");
    }

    /// With the panel out the list is already under the finger: no card
    /// comes over it on top of that.
    #[test]
    fn a_notice_that_comes_with_the_panel_out_stands_only_in_the_list() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        pulled(&mut app, &f);
        app.notify(Notification::event(
            Origin::System,
            "battery",
            "电量偏低",
            "剩 15%",
        ));
        assert!(
            app.banner.is_none(),
            "nothing comes over a panel already out"
        );
        assert_eq!(app.notifier.len(), 1, "but it stands in the list");
    }

    /// Nor does one come over a locked phone, or a dark one, where the card
    /// would be seen by nobody and waken whoever walked away.
    #[test]
    fn a_notice_that_comes_to_a_locked_phone_waits_in_the_list() {
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.notify(Notification::event(
            Origin::System,
            "battery",
            "电量偏低",
            "剩 15%",
        ));
        assert!(
            app.banner.is_none(),
            "the locked phone keeps it to the list"
        );
        assert_eq!(app.notifier.len(), 1);
    }

    /// A finger on a panel already out scrolls the notices rather than
    /// putting the panel away, and the panel stays out when it lifts.
    #[test]
    fn a_finger_on_a_full_panel_scrolls_its_notices() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        for i in 0..12u32 {
            let mut notice = Notification::event(Origin::System, "row", "标题", "正文");
            notice.id = i;
            app.notifier.post(notice);
        }
        pulled(&mut app, &f);
        assert!(
            app.shade_max_scroll() > 0.0,
            "there is more than the panel holds"
        );
        let before = app.shade_scrolled();
        app.touch(Touch::Down(200.0, 700.0), &f, 1_100);
        app.touch(Touch::Move(200.0, 600.0), &f, 1_150);
        assert!(
            app.shade_scrolled() > before,
            "the notices moved up the panel"
        );
        app.touch(Touch::Up, &f, 1_200);
        app.tick(2_000);
        assert_eq!(app.shade_open(2_000), 1.0, "and the panel is still out");
    }

    /// Twelve notices, more than the panel holds, so the list has somewhere
    /// to scroll.
    fn crowded(app: &mut App) {
        for i in 0..12u32 {
            let mut notice = Notification::event(Origin::System, "row", "标题", "正文");
            notice.id = i;
            app.notifier.post(notice);
        }
    }

    /// A list with nowhere left to scroll does not hold the panel out: the
    /// pull goes on into the panel, and the panel comes away.
    #[test]
    fn a_pull_past_the_end_of_the_list_takes_the_panel_with_it() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        crowded(&mut app);
        pulled(&mut app, &f);
        // The list is already at its end; a finger pulling up from there has
        // only the panel left to move.
        let max = app.shade_max_scroll();
        app.shade.as_mut().expect("the panel is out").scroll = max;
        app.touch(Touch::Down(200.0, 700.0), &f, 1_100);
        app.touch(Touch::Move(200.0, 60.0), &f, 1_150);
        assert!(
            app.shade_open(1_150) < 1.0,
            "past the list's end the panel comes back"
        );
        assert_eq!(app.shade_scrolled(), max, "and the list stays where it was");
        app.touch(Touch::Up, &f, 1_200);
        app.tick(2_000);
        assert!(app.shade.is_none(), "a quick pull takes it away whole");
    }

    /// A finger on the status bar works the panel rather than the list,
    /// whichever way it goes: pulled out from the bar, pushed back to it.
    #[test]
    fn a_pull_from_the_bar_pushes_the_panel_back_over_any_list() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        crowded(&mut app);
        pulled(&mut app, &f);
        let max = app.shade_max_scroll();
        app.shade.as_mut().expect("the panel is out").scroll = max / 2.0;
        let top = app.shade_geom().top;
        app.touch(Touch::Down(200.0, top - 4.0), &f, 1_100);
        app.touch(Touch::Move(200.0, top - 44.0), &f, 1_150);
        assert!(app.shade_open(1_150) < 1.0, "the bar pushes the panel back");
        assert_eq!(app.shade_scrolled(), max / 2.0, "and leaves the list alone");
    }

    /// Short of out whole, the panel covers only the top of the screen: a
    /// finger on the page below it is meant for putting it away.
    #[test]
    fn a_finger_below_a_half_pulled_panel_puts_it_away() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.shade_down();
        app.shade_pull(0.5);
        let geom = app.shade_geom();
        let below = geom.top + geom.panel_h() * 0.5 + 40.0;
        app.touch(Touch::Down(200.0, below), &f, 10);
        app.touch(Touch::Up, &f, 20);
        app.tick(1_000);
        assert!(app.shade.is_none(), "the panel is put away whole");
    }

    /// The panel stands over the page, not on it: back puts it away first,
    /// rather than leaving the page under it.
    #[test]
    fn the_back_key_puts_the_panel_away_first() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        pulled(&mut app, &f);
        let before = app.screen();
        app.key(Key::Back, 1_100);
        app.tick(2_000);
        assert!(app.shade.is_none(), "the panel goes first");
        assert_eq!(
            app.screen(),
            before,
            "and the page under it is left where it was"
        );
    }

    /// The control centre is a card of the pages' own kind: the same room
    /// from the screen's edges as the notices below it, floating below the
    /// bar rather than hanging from it, with its own room inside.
    #[test]
    fn the_control_centre_is_a_card_of_the_pages_own_kind() {
        let app = App::new(Screen::Desktop, 400, 800, 2.0);
        let t = app.theme();
        let geom = app.shade_geom();
        assert_eq!(geom.card.x, t.m.margin, "inset from the left edge");
        assert_eq!(
            geom.card.x + geom.card.w,
            t.m.w as f32 - t.m.margin,
            "and the right"
        );
        assert!(geom.card.y > geom.top, "it floats below the bar");
        assert_eq!(
            geom.card.w,
            t.m.w as f32 - 2.0 * t.m.margin,
            "as wide as a notice"
        );
        let (first, _) = geom.tiles[0];
        assert!(first.x > geom.card.x, "the keys keep the card's own room");
        assert!(
            geom.card.y + geom.card.h + t.m.dp(HEAD_FOOT_DP) <= geom.top + geom.head_h,
            "the notices begin below its foot"
        );
    }
}
