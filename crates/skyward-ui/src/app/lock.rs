//! The lock screen: what the panel shows when it lights again after going
//! dark, with the page it came over kept small and soft behind it, and the
//! swipe up that puts it away. It lies over whatever is shown rather than
//! standing in the stack, so the page under it is never taken away and comes
//! back as it was.

use super::*;

/// Where the clock stands down the screen, as a share of its height.
const LOCK_CLOCK_AT: f32 = 0.21;
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
/// How faint the veil over the picture is: dark enough for the clock and the
/// words to read, light enough that the page behind is still seen.
const VEIL_DARK: u8 = 64;
const VEIL_LIGHT: u8 = 36;

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
    /// The picture, the veil, the clock and the words, drawn once and moved
    /// over the page whole.
    pub(super) sheet: Option<Canvas>,
}

impl Lock {
    fn new() -> Lock {
        Lock { reveal: 0.0, to: 0.0, spring: None, sheet: None }
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
        }
    }

    /// Whether the lock stands over the page, and takes every touch.
    pub(super) fn locked(&self) -> bool {
        self.lock.is_some()
    }

    /// The lock over what is shown: the page behind it kept soft, the veil
    /// over that, and the clock and words over both. It slides up whole as a
    /// finger drags it, showing the page again under it.
    pub(super) fn compose_lock(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str) {
        if self.lock.as_ref().is_some_and(|l| l.sheet.is_none()) {
            let sheet = self.lock_sheet(f, info, clock);
            if let Some(lock) = self.lock.as_mut() {
                lock.sheet = Some(sheet);
            }
        }
        let Some(lock) = self.lock.as_ref() else { return };
        let sheet = lock.sheet.as_ref().expect("the sheet made just above");
        let away = (lock.reveal * self.h as f32).round() as i32;
        out.put(sheet, -away);
    }

    /// The lock drawn whole: the page behind it, softened, the veil, and the
    /// words over them.
    fn lock_sheet(&self, f: &Fonts, info: &DeviceInfo, clock: &str) -> Canvas {
        let t = self.theme();
        let m = &t.m;
        let (w, h) = (m.w as f32, m.h as f32);
        let mut c = Canvas::new(m.w, m.h);
        match self.behind.as_ref().filter(|page| page.w > 0 && page.h > 0) {
            Some(page) => c.blit_scaled(page, w, h),
            None => c.clear(t.pal.bg),
        }
        // Only a veil, not a coat: the page behind is to be seen, dimmed just
        // enough for the clock and the words to read over it. It lightens a
        // light page and darkens a dark one.
        let veil = match t.dark {
            true => Rgba::rgb(0, 0, 0).with_alpha(VEIL_DARK),
            false => Rgba::rgb(250, 250, 252).with_alpha(VEIL_LIGHT),
        };
        c.shade_rows(0, m.h, |_, _| veil);
        c.text_center(&f.title, clock, w / 2.0, h * LOCK_CLOCK_AT, m.dp(78.0), t.pal.text);
        if let Some(battery) = info.battery.as_ref() {
            let said = format!("{}%", battery.level);
            c.text_center(&f.body, &said, w / 2.0, h * LOCK_CLOCK_AT + m.dp(66.0), m.sub_px, t.pal.subtitle);
        }
        let said = self.state.lang.pick(["上滑解锁", "上滑解鎖", "Swipe up to open"]);
        c.text_center(&f.label, said, w / 2.0, h - m.dp(88.0), m.sub_px, t.pal.subtitle);
        c
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

/// `c` with every pixel the mean of the `reach` pixels either side of it,
/// past an edge counting the edge pixel again, three times over, which is
/// close to a Gaussian of that spread.
fn soften(c: &mut Canvas, reach: usize) {
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
