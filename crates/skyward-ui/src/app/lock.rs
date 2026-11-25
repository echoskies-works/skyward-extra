//! The lock screen: what the panel shows when it lights again after going
//! dark, and the swipe up that puts it away. It lies over whatever is shown
//! rather than standing in the stack, so the page under it is never taken
//! away and comes back as it was — and uncovered sharp, since the lock's own
//! wallpaper covers the screen until the finger drags it off.

use super::*;
use crate::canvas::soften;

/// Where the clock stands down the screen, as a share of its height. High
/// enough to leave the middle of the screen to the wallpaper, as the lock's
/// own system sets it.
const LOCK_CLOCK_AT: f32 = 0.181;
/// The clock's size in dp: the figures are the page, so they are set the way
/// a clock face's are rather than a title's that happens to be large. Set
/// semibold the figures are wider than a page's text, and this is the size
/// that keeps them to the middle of the screen with room either side, as the
/// lock's own system sets them.
const LOCK_CLOCK_PX: f32 = 88.0;
/// How far above the clock's middle the day stands, in dp: the day reads
/// first and the time under it, the way the lock's own system sets them.
const LOCK_DAY_AT: f32 = -62.0;
/// The day's size in dp: about a quarter of the clock's, so the time is what
/// the eye lands on and the day reads as the line standing over it.
const LOCK_DAY_PX: f32 = 24.0;
/// Where the words over the swipe stand, as a share of the screen's height:
/// low enough to sit under a thumb rather than floating above the foot of
/// the picture, and high enough to clear the strip the gesture bar keeps at
/// the foot (`desktop::FOOT_DP`). It stood at 0.872 until 2026-11-26, which
/// read as hanging in the middle of nothing.
const LOCK_WORDS_AT: f32 = 0.92;
/// How far up a finger drags the lock, as a share of the screen's height,
/// for letting go to put it away: short of it, the lock goes back where it
/// was. The lock travels one for one with the finger, so this is a distance
/// the finger itself covers — a short push up the screen, not a haul.
const LOCK_KEEP: f32 = 0.18;
/// A push this quick — screen heights a second — puts the lock away even
/// when the finger never got that far: a flick reads as meant.
const LOCK_FLING: f32 = 1.2;
/// How long the lock takes to go away once the finger lifts: slow enough
/// that the clock and the words are seen leaving the screen, rather than
/// gone before the eye has followed them.
const LOCK_AWAY: f32 = 0.52;
/// How long it takes to come back down when the finger let go short of the
/// mark. Quicker than going away: coming back reads as undoing a move, not
/// as a move of its own.
const LOCK_BACK: f32 = 0.34;
/// A pixel of every this many of the page is kept for the lock's ground
/// when no wallpaper is set, and for the soft copy the power menu stands
/// on.
const LOCK_STEP: u32 = 10;
/// How many of that small copy's pixels either side of a point are averaged
/// into the blur, three times over: the power menu's ground is soft rather
/// than the coarse mosaic of a copy kept too small. Two is about twenty
/// pixels of the page itself — enough to read it as a picture, not a shape.
/// The lock's own ground is kept sharp; only the menu uses this.
const LOCK_BLUR: usize = 2;
/// The lock's ink, fixed rather than the theme's: white and a step softer
/// over a dark wallpaper, near-black and a step softer over a light one.
const LOCK_INK: Rgba = Rgba::rgb(255, 255, 255);
const LOCK_SOFT: Rgba = Rgba::rgb(224, 224, 224);
const LOCK_INK_DARK: Rgba = Rgba::rgb(24, 24, 24);
const LOCK_SOFT_DARK: Rgba = Rgba::rgb(96, 96, 96);

/// The lock's words about the battery, kept beside its words about the
/// swipe. Nothing draws either yet — the lock shows no charge state — so
/// they are written here and left unused on purpose, for the screen that
/// will show them: the wording gets settled before the wiring does.
///
/// They go through `among`, as the swipe's words do, and move into the
/// locale files with the rest of the lock's text when that text moves.
#[allow(dead_code)]
pub(crate) fn charging(lang: Lang, level: u8) -> String {
    lang.among("正在充电 {level}%", "正在充電 {level}%", "Charging {level}%").replace("{level}", &level.to_string())
}

/// What the lock says when the battery is nearly out: the other half of the
/// pair above, and unused for the same reason.
#[allow(dead_code)]
pub(crate) fn battery_low(lang: Lang) -> &'static str {
    lang.among("电池电量不足", "電池電量不足", "Battery low")
}

/// The lock, once it is up: how far up it has been pushed, and where it is
/// going now that the finger has lifted.
pub(super) struct Lock {
    /// How far up the lock has been pushed, in pixels of this screen. The
    /// sheet is drawn whole and moved up by this much, so it travels exactly
    /// as far as the finger did.
    pub(super) away: f32,
    /// Where it comes to rest: nought back down, the screen's height away,
    /// the page under it left as it was.
    pub(super) to: f32,
    /// The spring carrying it once the finger has gone.
    pub(super) spring: Option<Spring>,
    /// The lock drawn whole — the wallpaper, the veil, the clock and the
    /// words — moved up over the page as the finger drags it.
    pub(super) sheet: Option<Canvas>,
}

impl Lock {
    fn new() -> Lock {
        Lock { away: 0.0, to: 0.0, spring: None, sheet: None }
    }
}

impl App {
    /// Take the lock up: the panel has gone dark, and what it shows next
    /// waits behind this. The preview takes it up this way too, the picture
    /// behind it being whatever was composed last.
    pub fn lock_now(&mut self) {
        self.lock = Some(Lock::new());
        // The volume panel belongs to the page below the lock, not to the
        // lock: it goes as the panel comes down.
        self.volume_bar = None;
        self.lock_sheet_gone();
    }

    /// The picture behind the lock is to be drawn again: the clock it shows
    /// has turned, or the page under it has changed.
    pub(super) fn lock_sheet_gone(&mut self) {
        if let Some(lock) = self.lock.as_mut() {
            lock.sheet = None;
        }
    }

    /// Whether the lock stands over the page, and takes every touch.
    pub(super) fn locked(&self) -> bool {
        self.lock.is_some()
    }

    /// How far up a finger can push the lock, in pixels of this screen: the
    /// screen's height, since the lock travels one for one with the finger.
    /// A caller dragging it wants this to know where the end is — the
    /// preview does, so its frames stand where the phone's would.
    pub fn lock_swipe_span(&self) -> f32 {
        self.h as f32
    }

    /// The lock over what is shown: its own wallpaper, veil, clock and words,
    /// slid up whole as a finger drags it. What it uncovers at the foot is
    /// the page itself, as it was left — sharp rather than blurred, which is
    /// also a whole screen's worth of mixing saved on every frame of the
    /// swipe.
    pub(super) fn compose_lock(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, today: &str) {
        // While the panel is going dark, the page is what dims: the lock is
        // what lighting it again shows, not something that comes up under
        // the dark. Drawn here it flashed over the page on the way out and
        // went black with it — the lock arriving, then the screen going,
        // which is the wrong way round.
        if !self.display_on {
            return;
        }
        if self.lock.as_ref().is_some_and(|l| l.sheet.is_none()) {
            let sheet = self.lock_sheet(f, info, clock, today);
            if let Some(lock) = self.lock.as_mut() {
                lock.sheet = Some(sheet);
            }
        }
        let Some(lock) = self.lock.as_ref() else { return };
        let away = lock.away.round() as i32;
        let sheet = lock.sheet.as_ref().expect("the sheet made just above");
        // Pushed the whole screen up, nothing of the lock is left to lay
        // down: the page under it is the whole picture now.
        if away >= self.h as i32 {
            return;
        }
        out.put(sheet, -away);
    }

    /// The lock drawn whole: its own wallpaper, the status bar, the day, the
    /// clock under it and the words. The lock's wallpaper read at startup
    /// is the ground it stands on whatever page it came over; where the
    /// ramdisk carries none, the desktop's wallpaper stands in, and where
    /// neither does, the page last shown. The wallpaper is shown as it is,
    /// with no veil laid over it, and the ink of the status bar, the clock,
    /// the day and the words follows the wallpaper: light on a dark one, dark
    /// on a light one.
    fn lock_sheet(&self, f: &Fonts, info: &DeviceInfo, clock: &str, today: &str) -> Canvas {
        let t = self.theme();
        let m = &t.m;
        let (w, h) = (m.w as f32, m.h as f32);
        let mut sheet = Canvas::new(m.w, m.h);
        let dark = match self.lock_wallpaper.as_ref().or(self.wallpaper.as_ref()) {
            Some(picture) => {
                sheet.blit_scaled(picture, w, h);
                is_dark(picture)
            }
            None => match self.behind.as_ref().filter(|page| page.w > 0 && page.h > 0) {
                Some(page) => {
                    sheet.blit_scaled(page, w, h);
                    is_dark(page)
                }
                None => {
                    sheet.clear(t.pal.bg);
                    t.dark
                }
            },
        };
        let (ink, soft) = if dark { (LOCK_INK, LOCK_SOFT) } else { (LOCK_INK_DARK, LOCK_SOFT_DARK) };
        // The status bar the lock carries of its own, in the lock's ink over
        // the wallpaper rather than the theme's over a page.
        widgets::status_bar(&mut sheet, &t, f, &self.bar(clock, info), ink, dark);
        // The day over the time, and the time larger than anything a page
        // sets: the lock states where in the month it is and what hour it is,
        // in that order, and nothing else of the phone. The clock is set a
        // step heavier than the pages' own text, as the stock lock sets it.
        sheet.text_center(&f.regular, today, w / 2.0, h * LOCK_CLOCK_AT + m.dp(LOCK_DAY_AT), m.dp(LOCK_DAY_PX), soft);
        sheet.text_center(&f.lock_clock, clock, w / 2.0, h * LOCK_CLOCK_AT, m.dp(LOCK_CLOCK_PX), ink);
        let said = self.state.lang.among("上滑解锁", "上滑解鎖", "Swipe up to open");
        sheet.text_center(&f.label, said, w / 2.0, h * LOCK_WORDS_AT, m.sub_px, soft);
        sheet
    }

    /// A finger on the lock: pushed up it goes away, let go short of the
    /// mark it goes back down. Nothing under it is touched, whatever the
    /// finger falls on.
    ///
    /// The lock travels one for one with the finger. It used to cover the
    /// whole screen's height in the time the finger took to cover a third of
    /// that, so the lock ran out from under the thumb about three pixels for
    /// every one of the finger's — which read as the lock bolting the moment
    /// it was touched. What the finger covers is now what the lock covers,
    /// and the push that puts it away is a short one.
    pub(super) fn lock_touch(&mut self, ev: Touch, now: u64) -> Vec<Effect> {
        let reach = self.h as f32;
        match ev {
            Touch::Down(_, y) => {
                // A measure of this finger alone: what it was doing before
                // the lock came up is not part of this push.
                self.tracker.clear();
                self.tracker.add(now, y);
                self.drag = Drag::Lock { from: y };
                if let Some(lock) = self.lock.as_mut() {
                    // A lock still springing back is taken hold of where it
                    // is, rather than jumping to the finger.
                    lock.spring = None;
                }
            }
            Touch::Move(_, y) => {
                let Drag::Lock { from } = self.drag else { return Vec::new() };
                // The finger's travel, kept the way every other drag keeps
                // it: letting go then reads a flick by the same measure the
                // foot of the screen does, rather than a second one written
                // here.
                self.tracker.add(now, y);
                let Some(lock) = self.lock.as_mut() else { return Vec::new() };
                lock.away = (from - y).clamp(0.0, reach);
                self.dirty = true;
            }
            Touch::Up => {
                let Drag::Lock { .. } = self.drag else { return Vec::new() };
                // The tracker reads y, which grows down the screen; up is the
                // way the lock goes, so the sign turns over.
                let speed = -self.tracker.velocity(now);
                let Some(lock) = self.lock.as_mut() else { return Vec::new() };
                let away = lock.away >= reach * LOCK_KEEP || speed >= reach * LOCK_FLING;
                let to = if away { reach } else { 0.0 };
                lock.to = to;
                let period = if away { LOCK_AWAY } else { LOCK_BACK };
                lock.spring = Some(Spring::new(now, lock.away - to, speed, period));
                self.drag = Drag::Idle;
                self.dirty = true;
            }
        }
        Vec::new()
    }

    /// Carry a drag of the lock on to where it goes, and take it away once it
    /// is gone.
    pub(super) fn settle_lock(&mut self, now: u64) -> Vec<Effect> {
        let fx = Vec::new();
        let reach = self.h as f32;
        let Some(lock) = self.lock.as_mut() else { return fx };
        let Some(spring) = lock.spring else { return fx };
        let (off, settled) = spring.at(now);
        lock.away = (lock.to + off).clamp(0.0, reach);
        self.dirty = true;
        // A spring that overshot and was held at the end has nothing left to
        // show. It can still read as unsettled by a fraction of a pixel, and
        // a lock left standing at the screen's height goes on taking every
        // touch while the page under it stands whole: the screen looks open
        // and nothing answers. Where it has arrived, it has arrived.
        if !settled && (lock.away - lock.to).abs() >= 0.5 {
            return fx;
        }
        if lock.to >= reach * 0.5 {
            // Away: the page under it is what shows, as it was left.
            self.lock = None;
            self.behind = None;
            return fx;
        }
        let lock = self.lock.as_mut().expect("the lock is under way");
        lock.away = 0.0;
        lock.spring = None;
        fx
    }

    /// Keep a small copy of what the panel shows, a pixel of every
    /// [`LOCK_STEP`], for the lock's ground when there is no wallpaper. It is
    /// kept sharp: softening it left the page under the lock reading as a
    /// blur on the way out, which is just what the lock no longer does.
    pub(super) fn keep(&mut self, out: &Canvas) {
        let (w, h) = (out.w, out.h);
        if !out.whole() || w < LOCK_STEP || h < LOCK_STEP {
            return;
        }
        let (sw, sh) = (w.div_ceil(LOCK_STEP), h.div_ceil(LOCK_STEP));
        let mut small = match self.behind.take() {
            Some(s) if (s.w, s.h) == (sw, sh) => s,
            _ => Canvas::new(sw, sh),
        };
        for y in 0..sh {
            let row = (y * LOCK_STEP).min(h - 1) as usize * w as usize;
            for x in 0..sw {
                let from = (row + (x * LOCK_STEP).min(w - 1) as usize) * 4;
                let to = ((y * sw + x) * 4) as usize;
                small.data[to..to + 4].copy_from_slice(&out.data[from..from + 4]);
            }
        }
        self.behind = Some(small);
    }
}

/// A soft copy of `picture` at `w` by `h`: a pixel of every [`LOCK_STEP`]
/// kept and blurred, stretched back over the screen. The power menu standing
/// over a page wants this as its ground — the picture behind it reads as a
/// picture, not as a shape, and blurring a tenth-scale copy costs a fraction
/// of blurring the page whole.
pub(super) fn soft_ground(picture: &Canvas, w: u32, h: u32) -> Canvas {
    let (sw, sh) = (w.div_ceil(LOCK_STEP), h.div_ceil(LOCK_STEP));
    let mut small = Canvas::new(sw, sh);
    for y in 0..sh {
        let row = (y * LOCK_STEP).min(h - 1) as usize * w as usize;
        for x in 0..sw {
            let from = (row + (x * LOCK_STEP).min(w - 1) as usize) * 4;
            let to = ((y * sw + x) * 4) as usize;
            small.data[to..to + 4].copy_from_slice(&picture.data[from..from + 4]);
        }
    }
    soften(&mut small, LOCK_BLUR);
    let mut out = Canvas::new(w, h);
    out.blit_scaled(&small, w as f32, h as f32);
    out
}

/// Brightness at or above which a pixel counts as paper rather than
/// picture: the lock only gives up its white ink for a ground that is
/// nearly white.
const LOCK_PAPER: u32 = 200;
/// How many pixels of paper make a picture paper: one in two. Below that
/// it is a picture with a bright patch in it, and white still reads.
const LOCK_PAPER_SHARE: u64 = 2;

/// Whether `picture` is dark enough that light ink reads on it.
///
/// Not the midpoint of the mean: the mean of a photograph sits near the
/// middle whatever the photograph is, so a midpoint test calls a blue sky
/// and a grey wall alike — and the lock's words sit at the top and the
/// foot of the screen, where a graded picture is brighter and darker than
/// its own mean. What the eye asks instead is whether this is a picture or
/// a sheet of paper: white ink reads on anything painted, and fails on
/// paper. So the lock counts the pixels that are nearly white and turns
/// its ink dark only when most of the picture is ([`LOCK_PAPER`],
/// [`LOCK_PAPER_SHARE`]).
///
/// This decides the ink of the clock, the day, the status bar and the
/// words, which follow the wallpaper rather than the theme.
fn is_dark(picture: &Canvas) -> bool {
    let (w, h) = (picture.w, picture.h);
    if w == 0 || h == 0 {
        return true;
    }
    let mut paper = 0u64;
    let mut n = 0u64;
    for y in (0..h).step_by(LOCK_STEP as usize) {
        let row = y as usize * w as usize;
        for x in (0..w).step_by(LOCK_STEP as usize) {
            let p = &picture.data[(row + x as usize) * 4..][..3];
            // The eye's weights: green counts most, blue least.
            let luma = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            if luma >= LOCK_PAPER {
                paper += 1;
            }
            n += 1;
        }
    }
    paper * LOCK_PAPER_SHARE <= n
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREY: Rgba = Rgba::rgb(0, 0, 0);

    fn at(c: &Canvas, x: u32, y: u32) -> u8 {
        c.data[((y * c.w + x) * 4) as usize]
    }

    #[test]
    fn softening_spreads_a_speck_no_further_than_its_reach() {
        let mut c = Canvas::filled(9, 9, GREY);
        let i = ((4 * 9 + 4) * 4) as usize;
        c.data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
        soften(&mut c, 1);
        assert!(at(&c, 4, 4) > 0 && at(&c, 4, 4) < 255, "the speck is spread over its own pixel");
        assert!(at(&c, 3, 4) > 0, "and reaches the pixel beside it");
        assert_eq!(at(&c, 0, 0), 0, "and no further than its reach");
    }

    #[test]
    fn softening_leaves_a_page_of_one_colour_as_it_was() {
        let mut c = Canvas::filled(6, 5, Rgba::rgb(120, 130, 140));
        let before = c.data.clone();
        soften(&mut c, 2);
        assert_eq!(c.data, before, "a page of one colour is still that colour");
    }

    #[test]
    fn the_page_is_kept_a_pixel_of_every_ten_and_no_bigger() {
        let mut app = App::new(Screen::Desktop, 100, 200, 3.0);
        app.keep(&Canvas::filled(100, 200, Rgba::rgb(10, 20, 30)));
        let kept = app.behind.as_ref().expect("a page shown is kept for the lock");
        assert_eq!((kept.w, kept.h), (10, 20), "a pixel of every ten, in either direction");
        assert_eq!(at(kept, 0, 0), 10, "and it is the page's own picture");
    }

    #[test]
    fn the_lock_comes_up_with_the_panel_going_dark_and_leaves_only_when_swiped() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        // The power key lighting the dark panel: it lights, and the lock is
        // what it lights onto.
        app.wake(0);
        app.lock_now();
        assert!(app.locked(), "the panel going dark leaves the lock behind it");
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 700.0 - 800.0 * 0.30), &f, 0);
        app.touch(Touch::Up, &f, 0);
        app.tick(1_000);
        assert!(!app.locked(), "swiped past the mark, the lock goes away");
        assert_eq!(app.screen(), Screen::Desktop, "and the page under it is where it was");
    }

    #[test]
    fn a_short_swipe_leaves_the_lock_where_it_was() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 700.0 - 800.0 * 0.08), &f, 0);
        app.touch(Touch::Up, &f, 0);
        app.tick(1_000);
        assert!(app.locked(), "let go short of the mark, the lock stays");
    }

    /// The panel going dark dims the page, and the lock stays out of sight
    /// until it lights again: drawn under the dark it flashed up over the
    /// page on the way out and went black with it, which read as the lock
    /// arriving and then the screen going, the wrong way round.
    #[test]
    fn the_lock_keeps_out_of_sight_while_the_panel_goes_dark() {
        let f = crate::font::bundled();
        let i = crate::app::testing::info();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        let mut out = Canvas::new(400, 800);
        app.compose(&mut out, &f, &i, "21:40", "11月18日 周二", 10);
        let lock = app.lock.as_ref().expect("the lock is what waits behind the dark");
        assert!(lock.sheet.is_none(), "and nothing of it is drawn under the dark");
    }

    /// Lighting the panel after a power-key sleep shows the lock coming up
    /// from black, never the page it went to sleep on. The page is under the
    /// lock's own wallpaper, which covers the screen from the first lit
    /// frame, so the brightening is of the lock and not a flash of the old
    /// page before the lock arrives.
    #[test]
    fn lighting_the_panel_shows_the_lock_not_the_page_it_left() {
        let f = crate::font::bundled();
        let i = crate::app::testing::info();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.set_lock_wallpaper(Canvas::filled(400, 800, Rgba::rgb(20, 20, 20)));
        app.sleep(0);
        // The power key down: light the panel and put the lock straight back.
        app.wake(10);
        app.lock_now();
        assert!(app.lock.is_some() && app.display_on, "the lock stands over the lit panel");
        // The first lit frame, and a later one while the content brightens:
        // the lock's sheet is drawn whole, covering the page under it.
        for now in [10, 100, 400] {
            let mut out = Canvas::new(400, 800);
            app.compose(&mut out, &f, &i, "21:40", "11月18日 周二", now);
            assert!(app.lock.as_ref().is_some_and(|l| l.sheet.is_some()), "the lock is drawn at {now}");
            // Its top-left is the lock wallpaper's grey, not the desktop's.
            let px = &out.data[0..4];
            assert!(px[0] < 60, "the lock wallpaper shows through at {now}, got {}", px[0]);
        }
    }

    /// The lock goes up exactly as far as the finger does. It used to run
    /// out from under the thumb three times as fast as the finger moved,
    /// which read as the lock bolting the moment it was touched.
    #[test]
    fn the_lock_travels_exactly_as_far_as_the_finger() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 500.0), &f, 16);
        let away = app.lock.as_ref().expect("the lock is up").away;
        assert_eq!(away, 200.0, "two hundred pixels of finger, two hundred of lock");
    }

    /// A quick flick puts the lock away even though the finger never got
    /// near the mark: the push was meant, and asking for a longer haul would
    /// read as the lock refusing.
    #[test]
    fn a_flick_puts_the_lock_away_short_of_the_mark() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        // Eighty pixels in sixteen milliseconds: a tenth of the screen, well
        // short of the mark, but travelling at five thousand pixels a second.
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 620.0), &f, 16);
        app.touch(Touch::Up, &f, 16);
        app.tick(1_000);
        assert!(!app.locked(), "a flick means it, short of the mark or not");
    }

    /// A slow push that stops short is not a flick, however far it came: the
    /// finger was at rest when it lifted, and the lock goes back down.
    #[test]
    fn a_slow_push_held_before_it_lifts_goes_back_down() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 620.0), &f, 16);
        // Held still for a fifth of a second before it let go.
        app.touch(Touch::Up, &f, 216);
        app.tick(1_000);
        assert!(app.locked(), "a still finger is not a flick");
    }

    /// A push thrown off in an instant overshoots and is held at the screen's
    /// height. The spring can go on reading as unsettled by a fraction of a
    /// pixel, and a lock left standing there takes every touch while the page
    /// under it stands whole: the screen looks open and nothing answers.
    #[test]
    fn a_quick_push_leaves_the_page_open_to_the_finger() {
        use crate::app::testing::{info, spot, unlock};
        let f = crate::font::bundled();
        let i = info();
        let mut app = App::new(Screen::Home, 1440, 3200, 3.5);
        app.sleep(0);
        // A finger on the dark panel, which lights nothing.
        app.touch(Touch::Down(700.0, 1600.0), &f, 100);
        app.touch(Touch::Move(700.0, 1300.0), &f, 120);
        app.touch(Touch::Up, &f, 140);
        app.wake(200);
        app.lock_now();
        // Sixteen hundred pixels in sixteen milliseconds.
        unlock(&mut app, &f, 300);
        assert!(!app.locked(), "the lock is away, not left standing at the top");
        let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
        app.touch(Touch::Down(x, y), &f, 1400);
        app.touch(Touch::Up, &f, 1400);
        assert_eq!(app.screen(), Screen::Settings, "and the page under it takes a finger");
    }

    /// Let go and the spring carries the lock where the swipe said, a frame
    /// at a time. Woken only when the spring was due to land, the panel drew
    /// the lock under the finger and then the page: the motion in between
    /// never reached the glass.
    #[test]
    fn the_lock_asks_for_frames_while_its_spring_carries_it() {
        let f = crate::font::bundled();
        let mut app = App::new(Screen::Desktop, 400, 800, 2.0);
        app.sleep(0);
        app.wake(0);
        app.lock_now();
        app.touch(Touch::Down(200.0, 700.0), &f, 0);
        app.touch(Touch::Move(200.0, 700.0 - 800.0 * 0.30), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.deadline(0), Some(0), "the spring is carried a frame at a time");
        let _ = app.take_dirty();
        app.tick(8);
        assert!(app.take_dirty(), "and each frame of it is drawn");
        app.tick(1_000);
        assert!(!app.locked(), "and it lands where the swipe said");
    }

    /// A picture of one colour, for the ink tests: what the lock reads is
    /// the colour of the pixels, not where they came from.
    fn filled(w: u32, h: u32, colour: Rgba) -> Canvas {
        let mut c = Canvas::new(w, h);
        c.clear(colour);
        c
    }

    /// The picture that started the argument: a mid-blue sky whose mean is
    /// 132 out of 255. The midpoint test called it light and drew the clock
    /// in dark grey on it; the eye calls it a sky.
    #[test]
    fn a_painted_sky_keeps_the_locks_white_ink() {
        let sky = filled(108, 192, Rgba::rgb(74, 143, 168));
        assert!(is_dark(&sky), "a painted sky is a picture, not paper");
    }

    /// Only a ground that is nearly white gives up the white ink: the lock
    /// asks how much of the picture is paper, not what it averages to.
    #[test]
    fn only_a_nearly_white_picture_turns_the_ink_dark() {
        assert!(!is_dark(&filled(108, 192, Rgba::rgb(255, 255, 255))), "white paper asks for dark ink");
        assert!(!is_dark(&filled(108, 192, Rgba::rgb(232, 232, 232))), "so does a pale wall");
        assert!(is_dark(&filled(108, 192, Rgba::rgb(0, 0, 0))), "and black keeps the white");
        assert!(is_dark(&filled(108, 192, Rgba::rgb(120, 120, 120))), "grey is still a picture");
    }

    /// The battery's words are written and not drawn yet. They have to be
    /// the three languages the lock speaks, with the level in the middle
    /// where each of them puts it.
    #[test]
    fn the_battery_words_are_written_in_the_locks_three_languages() {
        assert_eq!(charging(Lang::Hans, 47), "正在充电 47%");
        assert_eq!(charging(Lang::Hant, 47), "正在充電 47%");
        assert_eq!(charging(Lang::En, 47), "Charging 47%");
        assert_eq!(battery_low(Lang::Hans), "电池电量不足");
        assert_eq!(battery_low(Lang::Hant), "電池電量不足");
        assert_eq!(battery_low(Lang::En), "Battery low");
    }
}
