//! Drawing: the page rendered a band at a time, and what the panel shows
//! composed from it, with the bar, the titles and what slides or fades
//! over the page.

use super::*;

impl App {
    /// What the status bar shows, its clock reading `clock`: the battery,
    /// the network joined, the data partition's lock, the slot the system
    /// runs from, and the battery's temperature.
    pub fn bar<'a>(&self, clock: &'a str, info: &DeviceInfo) -> Bar<'a> {
        let battery = info.battery.as_ref();
        Bar {
            clock,
            battery: battery.map(|b| (b.level, b.charging().then_some(if b.quick { Bolt::Quick } else { Bolt::Charging }))),
            wlan: self.wlan_mark(),
            data: match self.state.data {
                Data::Erased => None,
                Data::Open => Some(true),
                Data::Closed | Data::Opening | Data::Failed(_) | Data::Locked { .. } => Some(false),
            },
            slot: info.slot.map(Slot::letter),
            temp: battery.and_then(|b| b.temp),
        }
    }

    /// Render the current page if what it shows has changed: the rows of
    /// it on screen, the rest as they come into view; or, where only some
    /// rows changed, those rows.
    pub fn frame(&mut self, f: &Fonts, info: &DeviceInfo) -> &Frame {
        self.render(f, info, self.w)
    }

    /// [`App::frame`], drawing of a page rendered anew only its columns
    /// from the left edge to `wide`.
    fn render(&mut self, f: &Fonts, info: &DeviceInfo, wide: u32) -> &Frame {
        if self.frame.is_some() && !self.stale.is_empty() {
            let t = self.theme();
            let screen = self.screen();
            for rows in std::mem::take(&mut self.stale) {
                let band = Band { rows, cols: (0, self.cols) };
                let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: Some(band), now: self.now, wallpaper: self.wallpaper.as_ref() };
                let fresh = pages::render(screen, &cx);
                let mut canvas = self.frame.take().expect("checked above").canvas;
                assert_eq!(canvas.h, fresh.canvas.h, "a page drawn again in part is as tall as it was");
                canvas.replace(&fresh.canvas);
                self.frame = Some(Frame { canvas, ..fresh });
            }
        }
        if self.frame.is_none() {
            let t = self.theme();
            let screen = self.screen();
            let first = self.top().1.max(0.0) as u32;
            let band = Band { rows: (first, first + self.h), cols: (0, wide.min(self.w)) };
            let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: Some(band), now: self.now, wallpaper: self.wallpaper.as_ref() };
            let mut frame = pages::render(screen, &cx);
            // A page drawn in part across holds its rows whole, the columns
            // of them drawn on as they are needed.
            self.cols = frame.canvas.cols().end;
            if self.cols < self.w {
                let mut whole = Canvas::unfilled(self.w, frame.canvas.h, frame.canvas.held());
                whole.replace(&frame.canvas);
                frame.canvas = whole;
            }
            self.height = frame.height(&t);
            self.frame = Some(frame);
            self.clamp_scroll();
            if std::mem::take(&mut self.snap) {
                let (title, bar) = self.bar_wanted().expect("rendered above");
                self.title.jump(title);
                self.bar.jump(bar);
            }
        }
        self.frame.as_ref().expect("rendered above")
    }

    /// The current page rendered, all of it drawn.
    pub fn whole_frame(&mut self, f: &Fonts, info: &DeviceInfo) -> &Frame {
        let h = self.frame(f, info).canvas.h;
        self.draw_cols(f, info, self.w);
        self.draw_rows(f, info, 0.0, h as f32);
        self.frame.as_ref().expect("rendered above")
    }

    /// Draw a part of the current page not drawn yet: the rest of the
    /// columns of its rows drawn, then a band of rows below those or else
    /// above them, for a page at rest to have all of it drawn by the time
    /// it scrolls into view; that done, make the About page field's dither,
    /// for that page to open without making it. Whether any is left to do.
    pub fn draw_ahead(&mut self, f: &Fonts, info: &DeviceInfo) -> bool {
        let Some(frame) = &self.frame else { return false };
        let (held, h) = (frame.canvas.held(), frame.canvas.h);
        if self.cols < self.w {
            self.draw_cols(f, info, self.w);
        } else if held.end < h {
            self.draw_rows(f, info, held.end as f32, (held.end + 1) as f32);
        } else if held.start > 0 {
            self.draw_rows(f, info, (held.start - 1) as f32, held.start as f32);
        } else if self.dither.is_none() {
            self.dither = Some(glow::Dither::new(self.w as usize, self.h as usize));
        }
        self.cols < self.w || self.frame.as_ref().is_some_and(|fr| !fr.canvas.whole()) || self.dither.is_none()
    }

    /// Draw the columns of the current page's rows drawn from where they
    /// were drawn to, on to `wide`.
    fn draw_cols(&mut self, f: &Fonts, info: &DeviceInfo, wide: u32) {
        let wide = wide.min(self.w);
        let Some(frame) = &self.frame else { return };
        if self.cols >= wide {
            return;
        }
        let held = frame.canvas.held();
        let t = self.theme();
        let band = Band { rows: (held.start, held.end), cols: (self.cols, wide) };
        let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: Some(band), now: self.now, wallpaper: self.wallpaper.as_ref() };
        let part = pages::render(self.screen(), &cx).canvas;
        self.frame.as_mut().expect("drawn above").canvas.replace(&part);
        self.cols = wide;
        self.layer = None;
    }

    /// Draw the rows `from..to` of the current page it has not drawn yet,
    /// a band at least a quarter of a screen tall at a time; the rows drawn
    /// are drawn across first.
    fn draw_rows(&mut self, f: &Fonts, info: &DeviceInfo, from: f32, to: f32) {
        let Some(frame) = &self.frame else { return };
        let h = frame.canvas.h;
        let (from, to) = ((from.max(0.0) as u32).min(h), (to.max(0.0).ceil() as u32).min(h));
        let held = frame.canvas.held();
        let least = self.h / 4;
        let mut bands = Vec::new();
        if from < held.start {
            bands.push((from.min(held.start.saturating_sub(least)), held.start));
        }
        if to > held.end {
            bands.push((held.end, to.max(held.end + least).min(h)));
        }
        if bands.is_empty() {
            return;
        }
        self.draw_cols(f, info, self.w);
        let t = self.theme();
        let screen = self.screen();
        for rows in bands {
            let band = Band { rows, cols: (0, self.w) };
            let cx = Ctx { t: &t, f, info, state: &self.state, term: self.term.as_ref(), band: Some(band), now: self.now, wallpaper: self.wallpaper.as_ref() };
            let part = pages::render(screen, &cx).canvas;
            self.frame.as_mut().expect("drawn above").canvas.join(&part);
        }
        self.layer = None;
    }

    pub(super) fn max_scroll(&self) -> f32 {
        (self.height - self.h as f32).max(0.0)
    }

    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        let top = self.top_mut();
        top.1 = top.1.clamp(0.0, max);
    }

    /// Draw what the panel shows now: the page, the charging screen over it
    /// while a cable has just gone in, both as dark as the content stands
    /// as the panel goes off or comes up.
    pub fn compose(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, today: &str, now: u64) {
        self.now = now;
        let charge = self.charging.as_ref().map_or(0.0, |c| c.shown.level(now));
        if charge < 1.0 {
            self.compose_page(out, f, info, clock, now);
        }
        // The notice that has just come stands over the page, under the
        // panel: with the panel out the list is the better place to read it.
        self.compose_banner(out, f, now);
        // The panel pulled down from the status bar comes over the page and
        // the tasks, and under the lock: with the phone locked it is not out
        // at all, so a finger cannot reach what it would not see.
        self.compose_shade(out, f, now);
        // The lock comes over the page, the charging screen and the tasks:
        // nothing any of them puts over another comes over the lock, and the
        // page under it is left as it was.
        self.compose_lock(out, f, info, clock, today);
        // The charging screen comes over the lock: a cable in lights the
        // panel to show the level, and the lock is there under it for once
        // that is put away, as the phone's own system does.
        if charge > 0.0 {
            self.compose_charging(out, f, info, charge, now);
        }
        // The power menu comes over that too: the power key answers while the
        // lock stands, so its menu must be the thing seen when it does.
        self.compose_power_menu(out, f, now);
        // Seen off from that menu, the page under the card goes black as the
        // card falls; the card itself is drawn after it and keeps its colour.
        self.compose_power_exit(out, now);
        // A sheet - the timer that has run out, and whatever else asks for
        // a word back - speaks over all of them until its key is pressed.
        self.compose_sheet(out, f, now);
        // What the panel shows is kept sharp for the lock's ground when no
        // wallpaper is set, while no lock is over it.
        if self.display_on && self.lock.is_none() {
            self.keep(out);
        }
        out.dim(self.curtain.level(now));
    }

    /// The charging screen, `level` of the way over the page drawn on `out`.
    fn compose_charging(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, level: f32, now: u64) {
        let battery = info.battery.as_ref().expect("a cable went in on a phone whose battery is read");
        let (t, lang) = (self.theme(), self.state.lang);
        if !self.charge_art.as_ref().is_some_and(|a| a.shows(lang, battery)) {
            self.charge_art = Some(charge::Art::new(f, &t.m, lang, battery).expect("the screens' face takes the charging screen's weight"));
        }
        let since = self.charging.as_ref().map_or(now, |c| c.since);
        let dither = self.dither.get_or_insert_with(|| glow::Dither::new(out.w as usize, out.h as usize));
        let art = self.charge_art.as_ref().expect("drawn above");
        if level >= 1.0 {
            charge::draw(out, art, &t.m, battery.level, now.saturating_sub(since), dither);
        } else {
            let screen = self.charge_canvas.get_or_insert_with(|| Canvas::new(out.w, out.h));
            charge::draw(screen, art, &t.m, battery.level, now.saturating_sub(since), dither);
            out.lay(screen, 0, 0, level);
        }
    }

    /// The visible part of the page, pulled past an end if it is, its large
    /// title fading as it scrolls, the bar and its title over a scrolled
    /// page, the `<`, the page sliding in or out over or under the page
    /// left, the status bar, any message, and the back cue while a swipe
    /// from an edge is under way.
    fn compose_page(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, now: u64) {
        // A page opened is drawn from its left edge as it slides in.
        let opening = self.turn.as_ref().is_some_and(|t| !t.back);
        self.render(f, info, if opening { 0 } else { self.w });
        let screen = self.screen();
        let scroll = self.top().1;
        let offset = (scroll + self.overscroll(now)).round();
        let caret_on = match self.frame.as_ref().is_some_and(|fr| fr.caret.is_some()) {
            true => self.caret_up(screen, now),
            false => {
                self.blink = None;
                self.caret_shown = false;
                false
            }
        };
        if let Some(m) = &self.morph {
            if !m.at(now).1 {
                return self.compose_morph(out, f, info, clock, now, caret_on);
            }
            self.morph = None;
        }
        let t = self.theme();
        if let Some((which, track)) = self.knob.take() {
            self.paint_knob(&t, f, which, track);
        }
        if self.toast.as_ref().is_some_and(|m| now >= m.1) {
            self.toast = None;
        }
        let title = self.frame.as_ref().expect("rendered above").title;
        let key = (title, self.state.dark);
        if self.titles.as_ref().is_none_or(|(k, _, _)| *k != key) {
            self.titles = Some((key, Blurs::new(widgets::big_title(&t, f, title)), Blurs::new(widgets::small_title(&t, f, title))));
        }
        if let Some(fold) = self.frame.as_ref().and_then(|fr| fr.fold) {
            let key = (fold.hint, self.state.dark);
            if self.search_words.as_ref().is_none_or(|(k, _)| *k != key) {
                self.search_words = Some((key, widgets::search_words(&t, f, fold.bar, fold.hint)));
            }
        }
        if self.chip.as_ref().is_none_or(|c| c.0 != t.dark) {
            let (art, x, y) = widgets::back_chip(&t);
            self.chip = Some((t.dark, art, x, y));
        }
        let button = self.frame.as_ref().is_some_and(|fr| fr.button.is_some());
        if button && self.button_chip.as_ref().is_none_or(|c| c.0 != t.dark) {
            let (art, x, y) = widgets::button_chip(&t);
            self.button_chip = Some((t.dark, art, x, y));
        }
        let slide = self.slid(now);
        if self.frame.as_ref().is_some_and(|fr| fr.hero.is_none()) {
            // What of the page is on screen, drawn if it has not been yet.
            // While it slides in, that is its columns come in, and a
            // quarter of the screen more each frame, so it is drawn across
            // early in the slide; all of it once a row of it is held.
            // The columns a page is drawn across: all of them but the ones a
            // page sliding in from the right has yet to reach. A page rising
            // is drawn across whole, since it comes up rather than in.
            let wide = match (slide, &self.turn) {
                (Motion::Move { at, .. }, Some(turn)) if !turn.back && self.row_press.is_none() => {
                    (self.w - at.clamp(0, self.w as i32) as u32).max(self.cols + self.w / 4)
                }
                _ => self.w,
            };
            self.draw_cols(f, info, wide);
            self.draw_rows(f, info, offset, offset + self.h as f32);
        }
        // The row a finger holds, drawn once more as held down when it
        // starts darkening, and only that row: a finger that moves on
        // before then never waits for it.
        if let (Some(press), None) = (self.row_press.filter(|p| now >= p.rise), &self.row_art) {
            let page = &self.frame.as_ref().expect("rendered above").canvas;
            let (x, y, w, h) = row_part(press.rect, page);
            let pressed = t.pressed();
            let band = Band { rows: (y, y + h), cols: (x, x + w) };
            let cx = Ctx { t: &pressed, f, info, state: &self.state, term: self.term.as_ref(), band: Some(band), now: self.now, wallpaper: self.wallpaper.as_ref() };
            let down = pages::render(screen, &cx).canvas;
            assert_eq!((down.w, down.h), (page.w, page.h), "a page held down is laid out as it was");
            self.row_art = Some((page.crop(x, y, w, h), down.crop(x, y, w, h)));
        }
        let view = View {
            scroll,
            offset,
            bar: self.bar.level(now),
            title: self.title.level(now),
            light: self.light(offset),
            caret: caret_on,
            row: self.row_press.map_or(0.0, |p| p.level(now)),
            pressing: self.pressing(),
        };
        // A finger pulling the page off the screen by its foot: the page
        // and the desktop under it, both drawn whole, moved against each
        // other, the page's foot standing on the row the pull has left it.
        if matches!(slide, Motion::None)
            && let Some(off) = self.foot.as_ref().map(|foot| foot.pull)
        {
            let mut page = Canvas::new(out.w, out.h);
            self.paint_page(&mut page, &t, Some((f, info)), &view);
            let bar = self.bar(clock, info);
            widgets::status_bar(&mut page, &t, f, &bar, t.pal.status, false);
            if let Some((text, _)) = &self.toast {
                widgets::toast(&mut page, &t, f, text);
            }
            let desk = self.desktop_canvas(f, info, clock);
            let foot = (self.h as f32 - off).round().clamp(0.0, self.h as f32) as i32;
            out.lift(&page, &desk, foot, 0.0, self.corners);
            self.shown = Some(view);
            self.layer = None;
            return;
        }
        match slide {
            Motion::Move { at, shift, dim } => {
                // The page shown is drawn on its own canvas, once for as
                // long as it shows the same, and moved from there with the
                // page left.
                let (back, rise) = {
                    let turn = self.turn.as_ref().expect("sliding");
                    (turn.back, turn.rise)
                };
                // A page opened shows nothing of itself until its edge comes
                // in from the right, or its top from the foot of the screen:
                // the page left alone shows.
                let off = if rise { self.h as i32 } else { self.w as i32 };
                if !back && at >= off {
                    let left = &self.turn.as_ref().expect("sliding").left;
                    if rise {
                        out.put(left, 0);
                        out.dim(dim);
                    } else {
                        out.slide(left, left, at, shift, dim, self.corners);
                    }
                } else {
                    let still = self.frame.as_ref().is_some_and(|fr| fr.hero.is_none());
                    let page = match self.layer.take() {
                        Some((shown, c)) if still && shown == view => c,
                        kept => {
                            let mut c = kept.map(|(_, c)| c).unwrap_or_else(|| Canvas::new(out.w, out.h));
                            self.paint_page(&mut c, &t, Some((f, info)), &view);
                            c
                        }
                    };
                    let turn = self.turn.as_ref().expect("sliding");
                    let (front, behind) = if back { (&turn.left, &page) } else { (&page, &turn.left) };
                    if rise {
                        out.rise(front, behind, at, dim, self.corners);
                    } else {
                        out.slide(front, behind, at, shift, dim, self.corners);
                    }
                    self.layer = Some((view, page));
                }
            }
            Motion::Zoom { rect, alpha } => {
                // The page comes out of the icon, or goes back into it: the
                // desktop underneath stays whole and sharp, the page over
                // it grows and shows at the same rate, its shape kept.
                let back = self.turn.as_ref().expect("zooming").back;
                let still = self.frame.as_ref().is_some_and(|fr| fr.hero.is_none());
                let page = match self.layer.take() {
                    Some((shown, c)) if still && shown == view => c,
                    kept => {
                        let mut c = kept.map(|(_, c)| c).unwrap_or_else(|| Canvas::new(out.w, out.h));
                        self.paint_page(&mut c, &t, Some((f, info)), &view);
                        c
                    }
                };
                let left = &self.turn.as_ref().expect("zooming").left;
                let (front, behind) = if back { (left, &page) } else { (&page, left) };
                out.put(behind, 0);
                out.draw_scaled_alpha(front, rect.x, rect.y, rect.w, rect.h, alpha);
                self.layer = Some((view, page));
            }
            Motion::None => {
                self.layer = None;
                self.paint_face(out, &t, f, info, &view, now);
            }
        }
        self.shown = Some(view);

        // The desktop's wallpaper is black whichever theme is on, so its bar
        // takes the light ink; every other page's bar follows its own page.
        let desk = self.screen() == Screen::Desktop;
        let bar_ink = if desk { pages::DESKTOP_STATUS } else { t.pal.status };
        widgets::status_bar(out, &t, f, &self.bar(clock, info), bar_ink, desk);
        if let Some((text, _)) = &self.toast {
            widgets::toast(out, &t, f, text);
        }
        self.compose_volume_panel(out, f);
        match self.drag {
            Drag::Edge { start, from_left, armed, x, cy } => {
                let travel = if from_left { x - start } else { start - x };
                widgets::back_cue(out, &t, from_left, cy, widgets::back_cue_depth(&t, travel), armed);
            }
            _ => {
                if let Some((from_left, cy, s)) = self.cue {
                    widgets::back_cue(out, &t, from_left, cy, s.at(now).0.max(0.0), false);
                }
            }
        }
    }

    /// Draw the page shown as `view` has it: all of it but the status bar,
    /// any message and the back cue, which lie over every page. Rows of its
    /// long list come into view are drawn with `fresh`; a page drawn again
    /// as it last showed has them drawn already.
    /// The page where it stands — or, while one clock face gives way to the
    /// other, the face coming in over the one shown: the new face drawn on a
    /// canvas of its own, the page as it was laid down under it, and the new
    /// one laid over that a little to the right of where it comes to rest,
    /// fading up as it goes.
    /// Draw the face `at` as a whole page on its own canvas, without
    /// touching the page being shown. A copy of the state stands in for the
    /// one on screen with just this page's face changed, so the strip can
    /// hold two faces at once while the finger moves them.
    fn face_canvas(&self, at: usize, f: &Fonts, info: &DeviceInfo, now: u64) -> Canvas {
        let (w, h) = (self.w, self.h);
        let t = self.theme();
        let mut state = self.state.clone();
        let screen = self.screen();
        match screen {
            Screen::Clock => state.clock.face = if at == 0 { ClockFace::Stopwatch } else { ClockFace::Timer },
            Screen::Calculator => state.calc_face = if at == 0 { CalcFace::Sum } else { CalcFace::Convert },
            Screen::Music => state.music_face = if at == 0 { MusicFace::List } else { MusicFace::Play },
            Screen::Tasks => state.tasks_face = TasksFace::ALL[at.min(TasksFace::ALL.len() - 1)],
            Screen::Store => state.store_face = StoreFace::ALL[at.min(StoreFace::ALL.len() - 1)],
            _ => {}
        }
        let cx = Ctx { t: &t, f, info, state: &state, term: self.term.as_ref(), band: None, now, wallpaper: self.wallpaper.as_ref() };
        pages::render(screen, &cx).canvas.crop(0, 0, w, h)
    }

    /// The two faces a finger pulls against each other: the one shown, and
    /// the one beside it. At either end there is no neighbour, and the
    /// empty canvas lets the page's ground show through.
    fn face_pair(&self, f: &Fonts, info: &DeviceInfo, from: usize, to: usize, now: u64) -> (Canvas, Canvas) {
        let left = self.face_canvas(from, f, info, now);
        let right = if to == from { Canvas::new(self.w, self.h) } else { self.face_canvas(to, f, info, now) };
        (left, right)
    }

    fn paint_face(&mut self, out: &mut Canvas, t: &Theme, f: &Fonts, info: &DeviceInfo, view: &View, now: u64) {
        // Faces under a finger move as one strip: the one shown and its
        // neighbour side by side, following the finger's shift. No new page
        // is drawn here — both were drawn whole when the drag began — so a
        // frame is only two canvases laid down.
        // The strip is drawn only once: the two faces were rendered whole
        // when the finger first went over the slop, and every frame after
        // that lays the same two canvases at a new shift.
        let undrawn = match self.face_swipe.as_ref() {
            Some(swipe) if !swipe.drawn => Some((swipe.from, swipe.to)),
            _ => None,
        };
        if let Some((from, to)) = undrawn {
            let (left, right) = self.face_pair(f, info, from, to, now);
            if let Some(swipe) = self.face_swipe.as_mut() {
                swipe.left = left;
                swipe.right = right;
                swipe.drawn = true;
            }
        }
        if let Some(swipe) = self.face_swipe.as_ref() {
            let w = self.w as i32;
            let shift = swipe.shift.round() as i32;
            // The face shown always goes with the finger. Its neighbour
            // comes in from the side the finger pulled away from: from the
            // right on a leftward pull, from the left on a rightward one.
            // At the ends the neighbour is an empty canvas, so the page's
            // ground shows where a face is not.
            let neighbor_at = if swipe.onward { shift + w } else { shift - w };
            out.clear(t.pal.bg);
            out.lay(&swipe.left, shift, 0, 1.0);
            out.lay(&swipe.right, neighbor_at, 0, 1.0);
        }
        if self.face_swipe.is_some() {
            // The controls at the top and the bar at the foot keep their
            // place while the faces move. The strip was drawn from the pages
            // alone — neither the `<`, the round button and the titles nor
            // the bar are in a page's canvas — so all of them are laid over
            // both halves here.
            self.paint_chrome(out, t, view);
            self.paint_foot(out, t, f, now);
            return;
        }
        let along = match &self.face_turn {
            Some(turn) => dialog::eased(now.saturating_sub(turn.since), FACE_MS),
            None => 1.0,
        };
        if along >= 1.0 {
            self.face_turn = None;
            self.paint_page(out, t, Some((f, info)), view);
            self.paint_foot(out, t, f, now);
            return;
        }
        let (w, h) = (out.w, out.h);
        let mut page = match self.face_page.take() {
            Some(c) if (c.w, c.h) == (w, h) => c,
            _ => Canvas::new(w, h),
        };
        self.paint_page(&mut page, t, Some((f, info)), view);
        if let Some(turn) = &self.face_turn {
            out.put(&turn.left, 0);
        }
        let dx = ((1.0 - along) * t.m.dp(FACE_SLIDE_DP)).round() as i32;
        out.lay(&page, dx, 0, along);
        self.face_page = Some(page);
        // The foot stays put while the face slides in over it.
        self.paint_foot(out, t, f, now);
    }

    pub(super) fn paint_page(&mut self, out: &mut Canvas, t: &Theme, fresh: Option<(&Fonts, &DeviceInfo)>, view: &View) {
        let m = t.m;
        let screen = self.screen();
        let (scroll, offset, pressing) = (view.scroll, view.offset, view.pressing);
        // The light goes over its control, in the part of the screen the
        // control is drawn in, under whatever is laid over that part.
        let shine = |out: &mut Canvas, part: Group| {
            if let Some((g, r, radius, tint)) = view.light
                && g == part
            {
                out.fill_round_rect(r.x, r.y, r.w, r.h, radius, tint);
            }
        };
        let frame = self.frame.as_ref().expect("a page is drawn from its frame");
        // The row a finger holds darkens as part of the page, under what is
        // laid over the page.
        let press_row = |out: &mut Canvas| {
            if let (Some(press), Some((was, is))) = (self.row_press, &self.row_art) {
                let (x, y, _, _) = row_part(press.rect, &frame.canvas);
                out.relay(was, is, x as i32, y as i32 - offset as i32, view.row);
            }
        };

        if let Some(art) = &frame.hero {
            // The field fills the screen and fades as the page scrolls; the
            // cards scroll over it. The version and the author's line fade
            // and shrink away first, then the wordmark.
            let glow = self.glow.as_ref().expect("the About page keeps its field");
            let fade = (1.0 - scroll / m.dp(ABOUT_FIELD_DP)).clamp(0.0, 1.0);
            let top = (widgets::bar_bottom(t) + m.dp(ABOUT_FIELD_DP)) / m.h as f32;
            let field = glow::sample(glow, out.w as usize, out.h as usize, FIELD_CELL, top, t.pal.bg, fade);
            let dither = self.dither.get_or_insert_with(|| glow::Dither::new(out.w as usize, out.h as usize));
            glow::paint(out, &field, fade, &glow::Layer { canvas: &frame.canvas, runs: &art.runs, offset: offset as isize }, dither);
            press_row(out);
            let range = m.dp(ABOUT_LOGO_DP) - widgets::bar_bottom(t);
            let v = (scroll / range).clamp(0.0, 1.0);
            art.version.draw_scaled(out, 0.0, 0.0, 1.0 - 0.1 * v, t.hero.version, 1.0 - v);
            art.author.draw_scaled(out, 0.0, 0.0, 1.0 - 0.1 * v, t.hero.version, 1.0 - v);
            let l = ((scroll - range) / m.dp(ABOUT_LOGO_FADE_DP)).clamp(0.0, 1.0);
            glow::ink_wordmark(out, &field, &glow.look().ink, &art.wordmark, 1.0 - 0.1 * l, 1.0 - l, fade, dither);
        } else {
            // The page rows on screen above a long list's rows, those rows,
            // and the page colour past the ends; every pixel is written
            // once.
            let h = out.h as f32;
            let span = frame.rows.as_ref().map(|r| ((r.edges[0] - offset).clamp(0.0, h), (r.bottom() - offset).clamp(0.0, h)));
            let (list_top, list_end) = span.unwrap_or((h, h));
            let first = (-offset).clamp(0.0, h) as usize;
            let last = (frame.canvas.h as f32 - offset).clamp(first as f32, list_top) as usize;
            if first < last {
                let from = offset as i32 + first as i32;
                out.put_rows(&frame.canvas, from, from + (last - first) as i32, first as i32);
            }
            out.fill_rect(0.0, 0.0, out.w as f32, first as f32, frame.bg);
            out.fill_rect(0.0, last as f32, out.w as f32, list_top - last as f32, frame.bg);
            let below = list_end.max(last as f32);
            out.fill_rect(0.0, below, out.w as f32, h - below, frame.bg);
            if let Some(rows) = &frame.rows {
                // A long list's rows on screen, each drawn when it first
                // shows. Those more than half a screen away are let go, and
                // their canvases drawn over again for the rows coming in, as
                // a list view recycles its rows.
                let near = rows.within(offset - h * NEAR_SCREENS, offset + h * (1.0 + NEAR_SCREENS));
                self.spare.extend(self.drawn.extract_if(.., |(i, _)| !near.contains(i)).map(|(_, c)| c));
                for i in rows.within(offset, offset + h) {
                    let k = match self.drawn.iter().position(|(j, _)| *j == i) {
                        Some(k) => k,
                        None => {
                            let (f, info) = fresh.expect("a page drawn again as it last showed has its rows in view drawn");
                            let mut c = self.spare.pop().unwrap_or_else(|| Canvas::new(out.w, 0));
                            c.resize((rows.edges[i + 1] - rows.edges[i]) as u32);
                            let cx = Ctx { t, f, info, state: &self.state, term: self.term.as_ref(), band: None, now: self.now, wallpaper: self.wallpaper.as_ref() };
                            pages::row(screen, &cx, i, &mut c);
                            self.drawn.push((i, c));
                            self.drawn.len() - 1
                        }
                    };
                    out.put(&self.drawn[k].1, (rows.edges[i] - offset) as i32);
                }
            } else {
                // Off a long list, its rows' canvases are not kept.
                self.spare.clear();
            }
            press_row(out);
            // The desktop's icon under the finger darkens where it stands,
            // inside its own square: nothing on the page moves with it, and
            // the glyph and the name under it are left as they are.
            if let Some(press) = self.icon_press.filter(|_| screen == Screen::Desktop) {
                let level = press.level(self.now);
                if level > 0.0 {
                    icons::tile_dim(out, press.rect.x, press.rect.y - offset, press.rect.w, ICON_PRESS_DIM * level);
                }
            }
            if let Some(fold) = &frame.fold {
                let (_, words) = self.search_words.as_ref().expect("a folding search bar's words are made with its page");
                fold.draw(out, t, words, offset);
            }
        }
        // The search page's round button and the home page's search bar
        // darken at once under a finger.
        if let Some(a) = pressing.filter(|a| matches!(a, Action::CloseSearch | Action::Open(Screen::Search))) {
            let r = frame.hits.iter().find(|h| h.action == a).expect("a button held down is on its page").rect;
            match frame.shown(r, offset) {
                Some(r) if a == Action::CloseSearch => widgets::cancel_held(out, t, r),
                Some(r) => widgets::search_bar_held(out, t, r),
                None => {}
            }
        }
        if let Some(caret) = frame.caret.as_ref().filter(|_| view.caret) {
            out.lay(&caret.art, caret.x, caret.y - offset as i32, 1.0);
        }
        let bar = view.bar;
        let strip = frame.pinned.as_ref().map(|p| (p, pinned_top(t, p, offset)));
        shine(out, Group::Page);
        if bar > 0.0 {
            match strip {
                // The strip under the bar is veiled with it, and the list
                // thins out below the strip.
                Some((p, top)) => widgets::strip_mask(out, t, frame.bg, bar, top.round(), top.round() + p.canvas.h as f32),
                None => {
                    let stops = if frame.hero.is_some() { &widgets::ABOUT_BAR_MASK[..] } else { &widgets::BAR_MASK[..] };
                    widgets::bar_mask(out, t, stops, frame.bg, bar);
                }
            }
        }
        if let Some((p, top)) = strip {
            out.put(&p.canvas, top.round() as i32);
            shine(out, Group::Strip);
        }
        if let Some(d) = &frame.dock {
            out.put(&d.canvas, (out.h - d.canvas.h) as i32);
            shine(out, Group::Dock);
        }
        // A page's `<`, its round button and its titles are laid over the
        // page rather than drawn into it, so they keep their place while
        // anything on the page moves.
        self.paint_chrome(out, t, view);
    }

    /// The controls a page keeps at its top — the `<`, the round button at
    /// the right end — and the titles that scroll away over them: the large
    /// one while it stands, and the small one that takes its place.
    ///
    /// They lie over the page rather than in it, which is what keeps them in
    /// place while a finger pulls a face aside: the strip is drawn from the
    /// pages alone, and this is laid over it after.
    fn paint_chrome(&mut self, out: &mut Canvas, t: &Theme, view: &View) {
        let m = t.m;
        let (scroll, offset, bar, pressing) = (view.scroll, view.offset, view.bar, view.pressing);
        let (_, big, small) = self.titles.as_mut().expect("a page's titles are drawn with it");
        let frame = self.frame.as_ref().expect("a page is drawn from its frame");
        if frame.header {
            let level = (1.0 - frame.title_lift(scroll) / m.dp(TITLE_FADE_DP)).clamp(0.0, 1.0);
            draw_faded(out, big, 0.0, -frame.title_lift(offset), t.pal.text, level, (1.0 - level) * m.dp(TITLE_BLUR_DP));
        }
        if frame.back {
            let (_, chip, x, y) = self.chip.as_ref().expect("a page's `<` is drawn with it");
            out.lay(chip, *x, *y, bar);
            if pressing == Some(Action::Back) {
                widgets::back_held(out, t);
            } else {
                widgets::back_arrow(out, t, t.pal.text);
            }
        }
        if let Some((glyph, action)) = frame.button {
            let (_, chip, x, y) = self.button_chip.as_ref().expect("a page's round button is drawn with its chip");
            // A page that stands at its top for good is given the chip
            // whole: no scroll would ever bring it up.
            let level = if frame.button_still { 1.0 } else { bar };
            out.lay(chip, *x, *y, level);
            if pressing == Some(action) {
                widgets::button_held(out, t, glyph);
            } else {
                widgets::button_glyph(out, t, glyph, t.pal.text);
            }
        }
        if (frame.back || frame.button.is_some())
            && let Some((g, r, radius, tint)) = view.light
            && g == Group::Fixed
        {
            out.fill_round_rect(r.x, r.y, r.w, r.h, radius, tint);
        }
        let level = view.title;
        let blur = if frame.hero.is_some() { 0.0 } else { (1.0 - level) * m.dp(TITLE_BLUR_DP) };
        draw_faded(out, small, 0.0, 0.0, t.pal.text, level, blur);
    }

    /// The rect a page starts from when it grows out of a desktop icon: the
    /// icon's left and right edges give the width, and the height follows
    /// the screen's own shape so the page is never stretched. Where the
    /// icon sits high enough the top edge is the icon's top and the page
    /// grows down past it; where it sits low the bottom edge is the icon's
    /// bottom and the page grows up, so the small page stays on screen.
    fn zoom_start(&self, icon: Rect) -> Rect {
        let w = self.w as f32;
        let h = icon.w * (self.h as f32 / w);
        let top = if icon.y + h <= self.h as f32 {
            icon.y
        } else {
            icon.y + icon.h - h
        };
        Rect::new(icon.x, top, icon.w, h)
    }

    /// The rect a page starts from when there is no icon to come out of, or
    /// to go back into: a square a tile's width at the middle of the
    /// screen, its height following the screen's shape as one out of an
    /// icon does, so the page never changes shape while it grows or folds.
    fn zoom_start_middle(&self) -> Rect {
        let (w, h) = (self.w as f32, self.h as f32);
        let side = self.theme().m.dp(pages::DESKTOP_TILE_DP);
        let height = side * (h / w);
        Rect::new((w - side) / 2.0, (h - height) / 2.0, side, height)
    }

    /// Where a page sliding in over the page left, or the page left
    /// sliding off, stands at `now`: across the screen, up it, or growing
    /// out of a desktop icon. Nothing once the move is over. See [`Turn`].
    fn slid(&mut self, now: u64) -> Motion {
        if self.turn.is_none() {
            return Motion::None;
        }
        // The move goes up and down — a page opened from the desktop, and
        // one going back to it — or across the screen between pages.
        let up = self.turn.as_ref().is_some_and(|t| t.rise);
        let back = self.turn.as_ref().is_some_and(|t| t.back);
        if self.turn.as_ref().is_some_and(|t| t.spring.is_none()) {
            let made = if up {
                // Both ways run from 1000 down to 0. Opening reads it as
                // how much of the page shows (0→1, growing out of the
                // icon); going back as how much is left (1→0, the page
                // shrinking into the icon while the desktop shows through).
                Spring::new(now, 1000.0, 0.0, ZOOM_PERIOD)
            } else if back {
                Spring::new(now, -1000.0, 0.0, BACK_PERIOD)
            } else {
                Spring::new(now, 1000.0, -OPEN_SPEED * 1000.0, OPEN_PERIOD)
            };
            self.turn.as_mut().expect("turn above").spring = Some(made);
        }
        let spring = self.turn.as_ref().and_then(|t| t.spring).expect("set above");
        let (x, done) = spring.at(now);
        if done {
            self.backdrop = self.turn.take().map(|t| t.left);
            return Motion::None;
        }
        if up {
            // How much of the page shows, 0 at the icon and 1 at the whole
            // screen. Opening goes 0→1 as the spring falls 1000→0; going
            // back the spring falls the same way, so read it 1→0 and the
            // page folds back into the icon instead of jumping there. Both
            // the rect and the fade use this one measure.
            let q = if back { x / 1000.0 } else { (1000.0 - x) / 1000.0 };
            let q = q.clamp(0.0, 1.0);
            // The icon it came out of, or a square at the middle of the
            // screen where there is none: a page opened some other way has
            // no icon to come from, and going back, the desktop may no
            // longer stand where it did.
            let start = match self.turn.as_ref().and_then(|t| t.icon) {
                Some(icon) => self.zoom_start(icon),
                None => self.zoom_start_middle(),
            };
            let (sw, sh) = (self.w as f32, self.h as f32);
            let lerp = |from: f32, to: f32| from + (to - from) * q;
            let rect = Rect::new(lerp(start.x, 0.0), lerp(start.y, 0.0), lerp(start.w, sw), lerp(start.h, sh));
            return Motion::Zoom { rect, alpha: q };
        }
        // Where the page in front stands, in screen widths from the left
        // edge.
        let along = if back { 1.0 + x / 1000.0 } else { x / 1000.0 };
        let w = self.w as f32;
        Motion::Move { at: (along * w).round() as i32, shift: (BEHIND_SHIFT * w * (1.0 - along)).round() as i32, dim: BEHIND_DIM * (1.0 - along) }
    }

    /// Whether the cursor of the text field on `screen` shows at `now`: it
    /// comes up whole on each page it shows on and at each edit, then
    /// blinks.
    fn caret_up(&mut self, screen: Screen, now: u64) -> bool {
        let key = (screen, self.edits);
        if self.blink.is_none_or(|(s, e, _)| (s, e) != key) {
            self.blink = Some((screen, self.edits, now));
        }
        self.caret_shown = self.blink_at(now).is_some_and(|(on, _)| on);
        self.caret_shown
    }

    /// Draw the search bar on its way between the home page and the search
    /// page; see [`Morph`].
    fn compose_morph(&mut self, out: &mut Canvas, f: &Fonts, info: &DeviceInfo, clock: &str, now: u64, caret_on: bool) {
        let t = self.theme();
        let m = t.m;
        let mut morph = self.morph.take().expect("composing it");
        let (q, _) = morph.at(now);
        let (field, _) = widgets::search_layout(&t);
        let home = Rect::new(m.margin, m.content_top - morph.scroll, m.w as f32 - m.margin * 2.0, m.search_h);
        let along = |a: f32, b: f32| a + (b - a) * q;
        let bar = Rect::new(m.margin, along(home.y, field.y), along(home.w, field.w), m.search_h);
        let rise = bar.y - home.y;
        if !morph.into {
            // The home page's rows coming down with its bar.
            self.draw_rows(f, info, morph.scroll - rise, morph.scroll - rise + m.h as f32);
        }
        let frame = self.frame.as_ref().expect("rendered above");
        out.fill_rect(0.0, 0.0, out.w as f32, out.h as f32, frame.bg);
        if morph.into {
            // The search page's keyboard and what it shows under its
            // field, from its frame; that comes in once the bar is nearly
            // there.
            let (_, keys_top) = *morph.keyboard.get_or_insert_with(|| keyboard_strip(&t, frame));
            let (body, top) = morph.body.get_or_insert_with(|| {
                let top = (field.y + field.h).ceil() as i32;
                let mut body = Canvas::new(frame.canvas.w, (keys_top - top).max(0) as u32);
                body.put_rows(&frame.canvas, top, keys_top, 0);
                (body, top)
            });
            let (delay, span) = MORPH_BODY_MS;
            out.lay(body, 0, *top, (now.saturating_sub(morph.start + delay) as f32 / span as f32).min(1.0));
        } else {
            // The home page under its bar comes down with it.
            let below = m.content_top + m.search_h;
            out.put_rows(&frame.canvas, below.floor() as i32, frame.canvas.h as i32, (below - morph.scroll + rise).floor() as i32);
        }
        let title = morph.title.get_or_insert_with(|| Blurs::new(widgets::big_title(&t, f, self.state.lang.pick(pages::HOME_TITLE))));
        let level = (1.0 - q / MORPH_TITLE_GONE).clamp(0.0, 1.0);
        draw_faded(out, title, 0.0, rise - morph.scroll, t.pal.text, level, (1.0 - level) * m.dp(TITLE_BLUR_DP));
        if let Some((keys, top)) = &morph.keyboard {
            out.put(keys, (*top as f32 + (1.0 - q) * keys.h as f32).round() as i32);
        }
        widgets::search_capsule(out, &t, f, bar, self.state.lang.pick(texts::search::HINT));
        // The cursor shows from the start, going up with the field.
        if let Some(caret) = frame.caret.as_ref().filter(|_| morph.into && caret_on) {
            out.lay(&caret.art, caret.x, caret.y + (bar.y - field.y).round() as i32, 1.0);
        }
        let (disc, cross) = morph.cancel.get_or_insert_with(|| {
            let (disc, cross) = widgets::cancel_art(&t);
            (Blurs::new(disc), Blurs::new(cross))
        });
        let shown = ((q - MORPH_CANCEL.0) / (MORPH_CANCEL.1 - MORPH_CANCEL.0)).clamp(0.0, 1.0);
        let blur = (1.0 - shown) * m.dp(MORPH_CANCEL_BLUR_DP);
        draw_faded(out, disc, 0.0, 0.0, t.pal.search, shown, blur);
        draw_faded(out, cross, 0.0, 0.0, t.pal.text, shown, blur);
        self.morph = Some(morph);
        widgets::status_bar(out, &t, f, &self.bar(clock, info), t.pal.status, false);
        if let Some((text, _)) = &self.toast {
            widgets::toast(out, &t, f, text);
        }
        self.compose_volume_panel(out, f);
    }

    /// Put the page back where it rests, for a page just opened or returned
    /// to; its first frame shows the bar as its scroll has it.
    pub(super) fn settle_page(&mut self) {
        self.height = 0.0;
        // A face turning belongs to the page it turned on: the next page
        // shows its own face as it stands.
        self.face_turn = None;
        self.fling = None;
        self.glide = None;
        self.pull = 0.0;
        self.bounce = None;
        self.seek = None;
        self.adjusting = false;
        self.titles = None;
        self.glow = None;
        self.row_press = None;
        self.row_art = None;
        let hero = self.screen() == Screen::About;
        self.title = title_fade(hero);
        self.bar = bar_fade(hero);
        self.snap = true;
    }

    /// Show the user what the runtime has to say about work it was asked
    /// to do.
    pub fn report(&mut self, text: &str, now: u64) {
        self.say(text, now);
    }

    /// The screenshot [`Effect::Screenshot`] asked for went where `saved`
    /// says, or did not, as the system says why.
    pub fn screenshot_saved(&mut self, saved: Result<Shot, String>, now: u64) {
        let l = self.state.lang;
        let text = match saved {
            Ok(Shot::Album) => l.pick(fmt::SCREENSHOT_SAVED).to_string(),
            Ok(Shot::Held) => l.pick(fmt::SCREENSHOT_HELD).to_string(),
            Err(why) => fmt::screenshot_failed(l, &why),
        };
        self.say(&text, now);
    }

    /// The screenshots held while the owner's storage was closed did not go
    /// into the gallery's once it opened, as the system says why.
    pub fn screenshots_not_kept(&mut self, why: &str, now: u64) {
        self.say(&fmt::screenshots_not_kept(self.state.lang, why), now);
    }

    /// File transfer could not be turned on or off as the switch says: it
    /// shows how it stands, and why.
    pub fn file_transfer_failed(&mut self, on: bool, why: &str, now: u64) {
        self.state.file_transfer = on;
        self.refresh();
        self.say(why, now);
    }

    pub(super) fn say(&mut self, text: &str, now: u64) {
        self.toast = Some((text.to_string(), now + TOAST_MS));
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use skyward_info::Charge;

    #[test]
    fn a_page_drawn_a_band_at_a_time_is_the_page_drawn_whole() {
        let (f, i) = (fonts(), info());
        let state = State::default();
        // The About page lies over its field and is drawn whole.
        let screens = [
            Screen::Decrypt,
            Screen::Home,
            Screen::Settings,
            Screen::Reboot,
            Screen::Wipe,
            Screen::Mount,
            Screen::Install,
            Screen::Choose(Place::Storage),
            Screen::Specs,
            Screen::ScreenOff,
            Screen::Search,
            Screen::Backup,
            Screen::Restore,
            Screen::Advanced,
            Screen::Files,
            Screen::Terminal,
            Screen::Language,
            Screen::TimeZone,
            Screen::TimeZones(Region::Asia),
            Screen::Logs,
            Screen::Update,
            Screen::SkywardUpdate,
            Screen::License,
            Screen::FontLicense,
            Screen::Slot,
            Screen::Fastboot,
            Screen::Erase,
            Screen::CantStart,
            Screen::Unhandled,
        ];
        for t in [Theme::light(W, H, PPD), Theme::dark(W, H, PPD).pressed()] {
            for screen in screens {
                let cx = |band| Ctx { t: &t, f: &f, info: &i, state: &state, term: None, band, now: 0, wallpaper: None };
                let whole = pages::render(screen, &cx(None)).canvas;
                let row = whole.w as usize * 4;
                let differs = |c: &Canvas| c.data.chunks(row).zip(whole.data.chunks(row)).position(|(a, b)| a != b);
                // Cuts through cards, text and icons, and a band one row
                // tall, the bands joined one under another.
                let mut cuts: Vec<u32> = [0, 1, 377, 1201, 3000, 3001, 4444, u32::MAX].map(|k| k.min(whole.h)).to_vec();
                cuts.dedup();
                let mut joined: Option<Canvas> = None;
                for rows in cuts.windows(2) {
                    let part = pages::render(screen, &cx(Some(Band { rows: (rows[0], rows[1]), cols: (0, W) }))).canvas;
                    assert_eq!((part.h, part.held()), (whole.h, rows[0]..rows[1]), "{screen:?}: a band is laid out as the whole");
                    match &mut joined {
                        Some(c) => c.join(&part),
                        None => joined = Some(part),
                    }
                }
                let joined = joined.expect("a page has rows");
                assert!(joined.whole(), "{screen:?}: the bands make the whole");
                assert_eq!(differs(&joined), None, "{screen:?}: the first row drawn otherwise in bands");
                // The page drawn across a part at a time, as one sliding
                // in, into a canvas holding all of it: cuts through cards
                // and text, and a part one column wide.
                let mut across = Canvas::unfilled(W, whole.h, 0..whole.h);
                for cols in [0, 1, 333, 720, W - 1, W].windows(2) {
                    let part = pages::render(screen, &cx(Some(Band { rows: (0, whole.h), cols: (cols[0], cols[1]) }))).canvas;
                    assert_eq!(part.cols(), cols[0]..cols[1], "{screen:?}: a part across holds its columns");
                    across.replace(&part);
                }
                assert_eq!(differs(&across), None, "{screen:?}: the first row drawn otherwise across");
            }
        }
    }

    #[test]
    fn a_page_at_rest_draws_the_rows_below_the_screen_a_band_at_a_time() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Settings, W, H, PPD);
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", 0);
        assert_eq!(app.frame.as_ref().unwrap().canvas.held(), 0..H, "the page shown has the rows on screen drawn");
        let mut bands = 1;
        while app.draw_ahead(&f, &i) {
            bands += 1;
        }
        let t = app.theme();
        let whole = pages::render(Screen::Settings, &Ctx { t: &t, f: &f, info: &i, state: &app.state, term: None, band: None, now: 0, wallpaper: app.wallpaper.as_ref() }).canvas;
        let page = &app.frame.as_ref().unwrap().canvas;
        assert!(whole.h > H + H / 4, "a page well over a screen long, drawn in {bands} more bands");
        assert!(page.whole() && page.data == whole.data, "the page drawn a band at a time is the page drawn whole");
    }

    #[test]
    fn a_page_typed_on_draws_again_only_what_changed_and_shows_what_it_would_drawn_anew() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Search, W, H, PPD);
        let mut out = Canvas::new(W, H);
        app.compose(&mut out, &f, &i, "12:00", "11月16日 周一", 0);
        let anew = |app: &App| {
            let t = app.theme();
            pages::render(Screen::Search, &Ctx { t: &t, f: &f, info: &i, state: &app.state, term: None, band: None, now: 0, wallpaper: app.wallpaper.as_ref() })
        };
        for cap in [Cap::Char('b'), Cap::Char('f'), Cap::Space, Cap::Backspace, Cap::Shift, Cap::Char('q')] {
            let (x, y) = spot(&mut app, &f, &i, Action::Type(cap));
            app.touch(Touch::Down(x, y), &f, 0);
            assert!(app.frame.is_some() && !app.stale.is_empty() || cap == Cap::Shift, "{cap:?}: the key held is drawn again on its own");
            app.compose(&mut out, &f, &i, "12:00", "11月16日 周一", 0);
            assert!(app.frame.as_ref().unwrap().canvas.data == anew(&app).canvas.data, "{cap:?} held down");
            app.touch(Touch::Up, &f, 0);
            app.compose(&mut out, &f, &i, "12:00", "11月16日 周一", 0);
            let (drawn, fresh) = (app.frame.as_ref().unwrap(), anew(&app));
            assert!(drawn.canvas.data == fresh.canvas.data, "{cap:?} typed");
            assert_eq!(drawn.hits.iter().map(|h| h.action).collect::<Vec<_>>(), fresh.hits.iter().map(|h| h.action).collect::<Vec<_>>(), "{cap:?} typed");
        }
        assert_eq!(app.state.search, "bfQ");
    }

    #[test]
    fn the_home_card_tells_of_a_battery_too_low_to_install_on() {
        let f = fonts();
        // Where the first row under the card starts: the card is that much
        // taller with a notice in it.
        let under = |i: &DeviceInfo| {
            let mut app = App::new(Screen::Home, W, H, PPD);
            app.frame(&f, i).hits.iter().find(|h| h.action == Action::Open(Screen::Install)).expect("an install row").rect.y
        };
        let plain = under(&info());
        assert!(under(&DeviceInfo { battery: Some(battery_at(19, Charge::Discharging)), ..info() }) > plain, "short of 20% unplugged");
        assert_eq!(under(&DeviceInfo { battery: Some(battery_at(20, Charge::Discharging)), ..info() }), plain);
        assert!(under(&DeviceInfo { battery: Some(battery_at(14, Charge::Charging)), ..info() }) > plain, "short of 15% charging");
        assert_eq!(under(&DeviceInfo { battery: Some(battery_at(16, Charge::Charging)), ..info() }), plain);
        let (title, note) = fmt::battery_low(Lang::Hans, 14, true);
        assert_eq!((title.as_str(), note.as_str()), ("电量仅剩 14%", "充电至 15% 以上才能安装"));
    }

    #[test]
    fn the_home_card_tells_of_a_restart_into_the_other_slot_and_opens_the_slot_page() {
        let f = fonts();
        let i = DeviceInfo { slot: Some(Slot::B), ..info() };
        let mut app = App::new(Screen::Home, W, H, PPD);
        let slots = |active| BootSlots { active, bootable: [true, true], successful: [true, true] };
        app.set_boot_slots(Ok(slots(Slot::B)));
        assert!(app.frame(&f, &i).hits.iter().all(|h| h.action != Action::Open(Screen::Slot)), "the slot it runs from starts next");
        app.set_boot_slots(Ok(slots(Slot::A)));
        assert_eq!(tap(&mut app, &f, &i, Action::Open(Screen::Slot)), vec![Effect::ReadSlots]);
        assert_eq!(app.screen(), Screen::Slot);
        assert_eq!(fmt::next_boot(Lang::Hans, Slot::A, Slot::B), ("重启后从 A 槽位启动".into(), "当前运行的是 B 槽位".into()));
    }

    #[test]
    fn the_status_bar_carries_the_slot_the_data_partitions_lock_and_the_temperature() {
        let i = DeviceInfo { slot: Some(Slot::B), battery: Some(battery_at(64, Charge::Charging)), ..info() };
        let mut app = App::new(Screen::Home, W, H, PPD);
        let bar = app.bar("12:00", &i);
        assert_eq!((bar.slot, bar.temp, bar.battery), (Some("B"), Some(352), Some((64, Some(Bolt::Charging)))), "the temperature to the tenth");
        let quick = DeviceInfo { battery: Some(skyward_info::Battery { quick: true, ..battery_at(64, Charge::Charging) }), ..info() };
        assert_eq!(app.bar("12:00", &quick).battery, Some((64, Some(Bolt::Quick))), "a quick charger draws the stock bar's quick bolt");
        let out = DeviceInfo { battery: Some(battery_at(64, Charge::Discharging)), ..info() };
        assert_eq!(app.bar("12:00", &out).battery, Some((64, None)), "no bolt with the cable out");
        let lock = |app: &mut App, data: Data| {
            app.state.data = data;
            app.bar("12:00", &i).data
        };
        assert_eq!(lock(&mut app, Data::Locked { user: 0, keys: Keys::Digits, checking: false }), Some(false), "shut while its files are out of reach");
        assert_eq!(lock(&mut app, Data::Open), Some(true));
        assert_eq!(lock(&mut app, Data::Erased), None, "nothing to open once wiped");
        let none = app.bar("12:00", &info());
        assert_eq!((none.slot, none.temp), (None, None), "a phone without slots or a fuel gauge");
    }

    #[test]
    fn the_cursor_blinks_and_comes_up_whole_at_each_edit() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Search, &f, &i);
        let mut screen = Canvas::new(W, H);
        let mut shows = |app: &mut App, now: u64| {
            app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", now);
            app.caret_shown
        };
        assert!(shows(&mut app, 1000), "it comes up whole on the page");
        assert_eq!(app.deadline(1200), Some(1500), "the loop wakes when it turns");
        assert!(!shows(&mut app, 1500), "half a second on it hides");
        assert!(shows(&mut app, 2000));
        assert!(!shows(&mut app, 2600));
        tap(&mut app, &f, &i, Action::Type(Cap::Char('b')));
        assert!(shows(&mut app, 2700), "an edit brings it back whole at once");
        assert!(!shows(&mut app, 3200));
        tap(&mut app, &f, &i, Action::CloseSearch);
        assert_eq!(app.screen(), Screen::Home, "the round button leaves the search");
        assert!(!shows(&mut app, 3300), "no cursor off the page");
        assert!(app.deadline(3300).is_none_or(|d| d > 10_000), "nor a wake for it, only for the screen going off");
    }

    #[test]
    fn the_search_bar_moves_up_into_the_search_page_and_back_down() {
        let (f, i) = (fonts(), info());
        let t = Theme::light(W, H, PPD);
        let m = t.m;
        let mut app = App::new(Screen::Home, W, H, PPD);
        let mut screen = Canvas::new(W, H);
        // The bar's top along a column through it, clear of the title and
        // the placeholder: the first row in the field's grey.
        let x = m.dp(300.0) as u32;
        let top = |screen: &Canvas| {
            (m.dp(40.0) as u32..H).find(|&y| screen.data[((y * W + x) * 4) as usize] == t.pal.search.r).map(|y| y as f32)
        };
        let grey = |screen: &Canvas, y: f32| screen.data[((y as u32 * W + W / 2) * 4) as usize];
        let (home, field) = (m.content_top, widgets::search_layout(&t).0.y);
        // The device card under the home page's bar.
        let card = m.content_top + m.search_h + m.after_search + m.dp(20.0);
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 0);
        assert_eq!(grey(&screen, card), t.pal.card.r);

        tap_at(&mut app, &f, &i, Action::Open(Screen::Search), 1000);
        assert_eq!(app.screen(), Screen::Search);
        assert_eq!(app.deadline(1000), Some(0), "it moves frame by frame");
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 1067);
        let along = crate::motion::settle(67, MORPH_PERIOD);
        let expected = home + (field - home) * along;
        let at = top(&screen).expect("the bar is on screen");
        assert!((at - expected).abs() <= 2.0, "up {along} of the way after 67 ms: at {at}, not {expected}");
        assert_eq!(grey(&screen, card), t.pal.bg.r, "the home page's cards go at once");
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 2500);
        assert!(app.morph.is_none(), "settled, the search page shows as itself");
        assert_eq!(top(&screen), Some(field.ceil()));

        tap_at(&mut app, &f, &i, Action::CloseSearch, 3000);
        assert_eq!(app.screen(), Screen::Home);
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 3067);
        let expected = field + (home - field) * along;
        let at = top(&screen).expect("the bar is on screen");
        assert!((at - expected).abs() <= 2.0, "down {along} of the way after 67 ms: at {at}, not {expected}");
        assert_eq!(grey(&screen, card + (at - home)), t.pal.card.r, "the cards come down under the bar");
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 4500);
        assert!(app.morph.is_none());
        assert_eq!(top(&screen), Some(home.ceil()));
    }

    #[test]
    fn a_desktop_icon_grows_into_its_page_and_shrinks_back() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        // The clock is one of the icons on the desktop.
        let fr = app.frame(&f, &i);
        let icon = fr.launches.iter().find(|(s, _)| *s == Screen::Clock).map(|(_, r)| *r).expect("the clock icon");
        assert!(icon.w > 0.0 && icon.h > 0.0, "the icon is a real square");
        // Show the desktop first: a page leaves whatever was last drawn, so
        // the icon has to have been on screen before the tap opens it.
        let mut screen = Canvas::new(W, H);
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 0);

        tap_at(&mut app, &f, &i, Action::Open(Screen::Clock), 1000);
        assert_eq!(app.screen(), Screen::Clock);
        assert!(app.turn.as_ref().is_some_and(|t| t.icon == Some(icon)), "the page came out of that icon");
        assert_eq!(app.deadline(1000), Some(0), "it grows a frame at a time");

        // The first frame starts at the icon: small and almost clear.
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 1000);
        assert!(app.turn.is_some(), "still growing on the first frame");
        // Mid growth still asks for frames: it is drawn a frame at a time.
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 1120);
        assert_eq!(app.deadline(1120), Some(0));

        // After it settles the page shows as itself, no turn left.
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 3000);
        assert!(app.turn.is_none(), "settled, the page is whole");

        // Going back shrinks into the same icon.
        press(&mut app, Key::Back, 4000);
        assert_eq!(app.screen(), Screen::Desktop);
        assert!(app.turn.as_ref().is_some_and(|t| t.icon == Some(icon)), "it goes back into the same icon");
        assert_eq!(app.deadline(4000), Some(0), "it shrinks a frame at a time");
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 4120);
        assert!(app.turn.is_some(), "still shrinking at 4120");
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 6000);
        assert!(app.turn.is_none(), "back on the desktop, the move is let go");
    }

    #[test]
    fn a_zoom_grows_monotonically_without_a_jump() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        // The desktop must have shown before a page can leave it.
        app.compose(&mut Canvas::new(W, H), &f, &i, "12:00", "11月16日 周一", 0);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Clock), 1000);
        // Read the rect straight from the motion, a few frames apart.
        let mut last_w = 0.0;
        let mut last_a = -1.0;
        for &now in &[1000, 1020, 1060, 1120, 1200, 1400] {
            match app.slid(now) {
                Motion::Zoom { rect, alpha } => {
                    assert!(rect.w >= last_w, "the page never shrinks while growing: {} < {}", rect.w, last_w);
                    assert!(alpha >= last_a, "and never fades back: {alpha} < {last_a}");
                    assert!((rect.h / rect.w - H as f32 / W as f32).abs() < 0.01, "shape held at {now}");
                    last_w = rect.w;
                    last_a = alpha;
                }
                other => panic!("expected a zoom at {now}, got {other:?}"),
            }
        }
        assert!(last_w > 0.0 && last_a > 0.0, "it really grew and showed");
        assert!(last_w <= W as f32 + 1.0, "never grows past the screen");
    }

    #[test]
    fn a_zoom_back_folds_the_page_into_the_icon_without_a_jump() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let mut screen = Canvas::new(W, H);
        app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", 0);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Clock), 1000);
        for now in (1000..=3000).step_by(16) {
            app.compose(&mut screen, &f, &i, "12:00", "11月16日 周一", now);
        }
        assert!(app.turn.is_none(), "opened and settled");

        press(&mut app, Key::Back, 4000);
        // Going back reads whole-screen first, then folds down to the icon:
        // the page never jumps small at the start, and is never still whole
        // at the end only to disappear in one frame.
        let mut last_w = W as f32 + 1.0;
        let mut last_a = 1.1;
        for &now in &[4000, 4020, 4060, 4120, 4200] {
            match app.slid(now) {
                Motion::Zoom { rect, alpha } => {
                    assert!(rect.w <= last_w, "the page never grows while folding: {} > {}", rect.w, last_w);
                    assert!(alpha <= last_a, "and never shows more: {alpha} > {last_a}");
                    last_w = rect.w;
                    last_a = alpha;
                }
                other => panic!("expected a folding zoom at {now}, got {other:?}"),
            }
        }
        // The first frame is still nearly the whole screen, not the icon.
        match app.slid(4000) {
            Motion::Zoom { rect, .. } => assert!(rect.w > W as f32 * 0.9, "starts whole: {}", rect.w),
            other => panic!("expected a zoom, got {other:?}"),
        }
    }

    #[test]
    fn a_zoom_keeps_the_pages_own_shape() {
        let app = App::new(Screen::Desktop, W, H, PPD);
        let icon = Rect::new(300.0, 200.0, 100.0, 100.0);
        let r = app.zoom_start(icon);
        // The width is the icon's left and right edges; the height follows
        // the screen, so a tall page is never squeezed into a square.
        assert_eq!(r.x, icon.x, "left edge is the icon's");
        assert_eq!(r.w, icon.w, "width is the icon's");
        assert!((r.h / r.w - H as f32 / W as f32).abs() < 0.001, "the page keeps the screen's shape");
        assert_eq!(r.y, icon.y, "a high icon grows down from its top");
    }

    #[test]
    fn a_low_icon_grows_up_from_its_bottom() {
        let app = App::new(Screen::Desktop, W, H, PPD);
        let icon = Rect::new(300.0, 3000.0, 100.0, 100.0);
        let r = app.zoom_start(icon);
        assert_eq!(r.y + r.h, icon.y + icon.h, "a low icon grows up from its bottom");
        assert!(r.y >= 0.0, "the small page stays on screen");
    }
}
