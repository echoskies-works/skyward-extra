//! The lock screen: what the panel shows when it lights again after going
//! dark, with the page it came over kept small and soft behind it, and the
//! swipe up that puts it away. It lies over whatever is shown rather than
//! standing in the stack, so the page under it is never taken away and comes
//! back as it was.

use super::*;

/// Where the clock stands down the screen, as a share of its height. High
/// enough to leave the middle of the screen to the wallpaper, as the lock's
/// own system sets it.
const LOCK_CLOCK_AT: f32 = 0.181;
/// The clock's size in dp: the figures are the page, so they are set the way
/// a clock face's are rather than a title's that happens to be large.
const LOCK_CLOCK_PX: f32 = 104.0;
/// How far under the clock's middle the day stands, in dp.
const LOCK_DAY_AT: f32 = 66.0;
/// Where the words over the swipe stand, as a share of the screen's height:
/// on the line the lock's own system puts its torch and its camera on, near
/// enough the foot to sit under a thumb and clear of it.
const LOCK_WORDS_AT: f32 = 0.872;
/// How far up a finger drags the lock, as a share of the screen's height,
/// to put it away: let go short of it, the lock goes back where it was.
const LOCK_SWIPE: f32 = 0.34;
/// How long the lock takes to go back, or away, once the finger lifts.
const LOCK_PERIOD: f32 = 0.36;
/// How far up the lock is dragged, of the share [`LOCK_SWIPE`] names, for
/// letting go to put it away rather than back down.
const LOCK_KEEP: f32 = 0.4;
/// A pixel of every this many of the page is kept for the picture the lock
/// shows behind itself.
const LOCK_STEP: u32 = 10;
/// How many of that small copy's pixels either side of a point are averaged
/// into the blur, three times over: the lock's picture is soft rather than
/// the coarse mosaic of a copy kept too small. Two is about twenty pixels
/// of the page itself — enough to read it as a picture, not as a shape.
const LOCK_BLUR: usize = 2;
/// The lock's ink, fixed rather than the theme's: white and a step softer
/// over a dark wallpaper, near-black and a step softer over a light one.
const LOCK_INK: Rgba = Rgba::rgb(255, 255, 255);
const LOCK_SOFT: Rgba = Rgba::rgb(224, 224, 224);
const LOCK_INK_DARK: Rgba = Rgba::rgb(24, 24, 24);
const LOCK_SOFT_DARK: Rgba = Rgba::rgb(96, 96, 96);

/// The lock, once it is up: how far up it has been dragged, and where it is
/// going now that the finger has lifted.
pub(super) struct Lock {
    /// How far the lock has been pulled up, nought to one of the share of
    /// the screen [`LOCK_SWIPE`] names.
    pub(super) reveal: f32,
    /// Where it comes to rest: nought back down, one away, the page under it
    /// left as it was.
    pub(super) to: f32,
    /// The spring carrying it once the finger has gone.
    pub(super) spring: Option<Spring>,
    /// The lock drawn whole — the wallpaper, the veil, the clock and the
    /// words — moved up over the page as the finger drags it.
    pub(super) sheet: Option<Canvas>,
    /// The page under the lock, blurred: shown for as long as the lock is up
    /// and cleared toward the page itself as the lock is pulled away.
    pub(super) blurred: Option<Canvas>,
}

impl Lock {
    fn new() -> Lock {
        Lock { reveal: 0.0, to: 0.0, spring: None, sheet: None, blurred: None }
    }
}

impl App {
    /// Take the lock up: the panel has gone dark, and what it shows next
    /// waits behind this. The preview takes it up this way too, the picture
    /// behind it being whatever was composed last.
    pub fn lock_now(&mut self) {
        self.lock = Some(Lock::new());
        self.lock_sheet_gone();
    }

    /// The picture behind the lock is to be drawn again: the clock it shows
    /// has turned, or the page under it has changed.
    pub(super) fn lock_sheet_gone(&mut self) {
        if let Some(lock) = self.lock.as_mut() {
            lock.sheet = None;
            lock.blurred = None;
        }
    }

    /// Whether the lock stands over the page, and takes every touch.
    pub(super) fn locked(&self) -> bool {
        self.lock.is_some()
    }

    /// How far up a finger has to drag, in pixels of this screen, to put the
    /// lock away. The lock follows the finger one for one up to that, so a
    /// caller dragging it wants this to know where the end is — the preview
    /// does, so its frames stand where the phone's would.
    pub fn lock_swipe_span(&self) -> f32 {
        self.h as f32 * LOCK_SWIPE
    }

    /// The lock over what is shown: the page under it kept soft, and the
    /// lock's own wallpaper, veil, clock and words over that. The lock
    /// slides up whole as a finger drags it, and as it goes the page under
    /// it clears — shown blurred while the lock stands, it comes back sharp
    /// as the lock is pulled away, rather than snapping from blur to focus
    /// at the last.
    pub(super) fn compose_lock(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, today: &str) {
        if self.lock.as_ref().is_some_and(|l| l.sheet.is_none()) {
            let sheet = self.lock_sheet(f, info, clock, today);
            // The page under the lock, kept soft while the lock stands over
            // it: blurred once here, from what the panel shows, and shown
            // until the lock is gone.
            let blurred = soft_ground(out, self.w, self.h);
            if let Some(lock) = self.lock.as_mut() {
                lock.sheet = Some(sheet);
                lock.blurred = Some(blurred);
            }
        }
        let Some(lock) = self.lock.as_ref() else { return };
        let away = (lock.reveal * self.h as f32).round() as i32;
        // The page under the lock stands blurred while the lock is over it,
        // and clears toward itself as the lock is pulled away. The blur is
        // kept for as long as the lock stands: the page is not touched while
        // it is covered.
        if let Some(blurred) = &lock.blurred {
            out.lay(blurred, 0, 0, 1.0 - lock.reveal);
        }
        let sheet = lock.sheet.as_ref().expect("the sheet made just above");
        out.put(sheet, -away);
    }

    /// The lock drawn whole: its own wallpaper, the status bar, the clock,
    /// the day under it and the words. The lock's wallpaper read at startup
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
        // The clock is the lock's one piece of lettering done large, so it is
        // set in the brand's own typeface rather than the UI's: the same face
        // the wordmark on the About page is cut from.
        sheet.text_center(&f.display, clock, w / 2.0, h * LOCK_CLOCK_AT, m.dp(LOCK_CLOCK_PX), ink);
        // The day under the clock, on the line the battery's level used to
        // take. The level is in the status bar already, and the day is what
        // the lock's own system puts under the time.
        sheet.text_center(&f.body, today, w / 2.0, h * LOCK_CLOCK_AT + m.dp(LOCK_DAY_AT), m.sub_px, soft);
        let said = self.state.lang.pick(["上滑解锁", "上滑解鎖", "Swipe up to open"]);
        sheet.text_center(&f.label, said, w / 2.0, h * LOCK_WORDS_AT, m.sub_px, soft);
        sheet
    }

    /// A finger on the lock: dragged up it goes away, let go short of the
    /// mark it goes back down. Nothing under it is touched, whatever the
    /// finger falls on.
    pub(super) fn lock_touch(&mut self, ev: Touch, now: u64) -> Vec<Effect> {
        let span = self.h as f32 * LOCK_SWIPE;
        match ev {
            Touch::Down(_, y) => {
                self.drag = Drag::Lock { from: y };
                if let Some(lock) = self.lock.as_mut() {
                    // A lock still springing back is taken hold of where it
                    // is, rather than jumping to the finger.
                    lock.spring = None;
                }
            }
            Touch::Move(_, y) => {
                let Drag::Lock { from } = self.drag else { return Vec::new() };
                let Some(lock) = self.lock.as_mut() else { return Vec::new() };
                lock.reveal = ((from - y) / span).clamp(0.0, 1.0);
                self.dirty = true;
            }
            Touch::Up => {
                let Some(lock) = self.lock.as_mut() else { return Vec::new() };
                let to = match lock.reveal >= LOCK_KEEP {
                    true => 1.0,
                    false => 0.0,
                };
                lock.to = to;
                lock.spring = Some(Spring::new(now, lock.reveal - to, 0.0, LOCK_PERIOD));
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
        let Some(lock) = self.lock.as_mut() else { return fx };
        let Some(spring) = lock.spring else { return fx };
        let (off, settled) = spring.at(now);
        lock.reveal = (lock.to + off).clamp(0.0, 1.0);
        self.dirty = true;
        if !settled {
            return fx;
        }
        if lock.to >= 0.5 {
            // Away: the page under it is what shows, as it was left.
            self.lock = None;
            self.behind = None;
            return fx;
        }
        let lock = self.lock.as_mut().expect("the lock is under way");
        lock.reveal = 0.0;
        lock.spring = None;
        fx
    }

    /// Keep a small copy of what the panel shows, a pixel of every
    /// [`LOCK_STEP`], softened: the picture the lock stretches behind itself
    /// once the panel comes back on.
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
        soften(&mut small, LOCK_BLUR);
        self.behind = Some(small);
    }
}

/// A soft copy of `picture` at `w` by `h`: a pixel of every [`LOCK_STEP`]
/// kept and blurred, stretched back over the screen. A lock or a menu that
/// stands over a page wants this as its ground — the picture behind it reads
/// as a picture, not as a shape, and blurring a tenth-scale copy costs a
/// fraction of blurring the page whole.
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

/// `c` with every pixel the mean of the `reach` pixels either side of it,
/// past an edge counting the edge pixel again, three times over, which is
/// close to a Gaussian of that spread.
pub(super) fn soften(c: &mut Canvas, reach: usize) {
    if reach == 0 || c.w == 0 || c.h == 0 {
        return;
    }
    let (w, h) = (c.w as usize, c.h as usize);
    let width = (2 * reach + 1) as u32;
    let mut a = c.data.clone();
    let mut b = vec![0u8; a.len()];
    for _ in 0..3 {
        // Along each row, then down each column: the running sum of the
        // window either side of the pixel.
        for y in 0..h {
            let row = y * w * 4;
            for ch in 0..4 {
                let mut sum = a[row + ch] as u32 * (reach as u32 + 1);
                for k in 1..=reach {
                    sum += a[row + k.min(w - 1) * 4 + ch] as u32;
                }
                for x in 0..w {
                    b[row + x * 4 + ch] = ((sum + width / 2) / width) as u8;
                    let gone = a[row + x.saturating_sub(reach) * 4 + ch] as u32;
                    let come = a[row + (x + reach + 1).min(w - 1) * 4 + ch] as u32;
                    sum = sum - gone + come;
                }
            }
        }
        for x in 0..w {
            for ch in 0..4 {
                let mut sum = b[x * 4 + ch] as u32 * (reach as u32 + 1);
                for k in 1..=reach {
                    sum += b[k.min(h - 1) * w * 4 + x * 4 + ch] as u32;
                }
                for y in 0..h {
                    a[y * w * 4 + x * 4 + ch] = ((sum + width / 2) / width) as u8;
                    let gone = b[y.saturating_sub(reach) * w * 4 + x * 4 + ch] as u32;
                    let come = b[(y + reach + 1).min(h - 1) * w * 4 + x * 4 + ch] as u32;
                    sum = sum - gone + come;
                }
            }
        }
    }
    c.data = a;
}

/// Whether `picture` is dark enough that light ink reads on it: the mean
/// brightness of its pixels, a pixel of every [`LOCK_STEP`] taken, against
/// the midpoint. The lock's clock and words take their ink from this rather
/// than the theme, so they follow the wallpaper instead of the page.
fn is_dark(picture: &Canvas) -> bool {
    let (w, h) = (picture.w, picture.h);
    if w == 0 || h == 0 {
        return true;
    }
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in (0..h).step_by(LOCK_STEP as usize) {
        let row = y as usize * w as usize;
        for x in (0..w).step_by(LOCK_STEP as usize) {
            let p = &picture.data[(row + x as usize) * 4..][..3];
            // The eye's weights: green counts most, blue least.
            sum += (p[0] as u64 * 299 + p[1] as u64 * 587 + p[2] as u64 * 114) / 1000;
            n += 1;
        }
    }
    sum / n.max(1) < 128
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
        app.touch(Touch::Move(200.0, 700.0 - 800.0 * LOCK_SWIPE * 0.6), &f, 0);
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
        app.touch(Touch::Move(200.0, 700.0 - 800.0 * LOCK_SWIPE * 0.2), &f, 0);
        app.touch(Touch::Up, &f, 0);
        app.tick(1_000);
        assert!(app.locked(), "let go short of the mark, the lock stays");
    }
}
