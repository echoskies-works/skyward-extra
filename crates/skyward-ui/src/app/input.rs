//! Fingers and keys: what a touch on the glass and a key going down or
//! coming up do.

use super::*;

impl App {
    /// A finger has gone over the slop level across a page whose top words
    /// are tabs. Draw both faces whole and hand them to the finger: the one
    /// shown and the one beside it in the direction it pulled. At either
    /// end there is no neighbour, so the second canvas is left empty and
    /// the ground of the page shows through as the finger drags past it.
    /// The face itself changes only when the finger settles on it.
    pub(super) fn start_face_swipe(&mut self, now: u64, onward: bool, x0: f32, x: f32) {
        let Some((from, count)) = self.face_slots() else { return };
        let at = if onward { (from + 1).min(count - 1) } else { from.saturating_sub(1) };
        let to = if at == from { from } else { at };
        // The strip runs sideways, so the speed measured from here on is
        // along x: the y samples the touch began with are of no use to it.
        // Rebase them rather than clear them, so the first sideways move is
        // still measured against the moment the finger went down. This very
        // move is still in `Drag::Press`, so add its x here: the `Drag::Face`
        // branch only sees the moves that come after it.
        self.tracker.rebase(x0);
        self.tracker.add(now, x);
        // The two faces are drawn where they are painted, where the fonts
        // and device are at hand; here only the finger's travel is known.
        let (w, h) = (self.w, self.h);
        self.face_swipe = Some(FaceSwipe { from, to, onward, shift: x - x0, spring: None, target: 0.0, drawn: false, left: Canvas::new(w, h), right: Canvas::new(w, h) });
        self.face_turn = None;
        self.face_page = None;
        self.dirty = true;
    }

    /// Carry a face strip along with the finger while it is down. The two
    /// faces were drawn when it began; only their shift changes, so no page
    /// needs drawing again on the way.
    pub(super) fn drag_face_swipe(&mut self, x0: f32, x: f32) {
        if let Some(swipe) = self.face_swipe.as_mut() {
            let w = self.w as f32;
            swipe.shift = (x - x0).clamp(-w, w);
            self.dirty = true;
        }
    }

    /// The finger has lifted off a face strip: send it to the face it pulled
    /// far enough towards, or back to the one it came from, with a spring
    /// that carries it there a frame at a time.
    pub(super) fn release_face_swipe(&mut self, now: u64, velocity: f32) {
        let w = self.w as f32;
        let flick = w * FACE_FLING;
        let Some(swipe) = self.face_swipe.as_mut() else { return };
        let (onward, to) = (swipe.onward, swipe.to);
        let settled = if to == swipe.from {
            // Past the last face there is nothing to settle on.
            swipe.from
        } else if swipe.shift.abs() > w * FACE_COMMIT || velocity.abs() > flick && (velocity < 0.0) == onward {
            to
        } else {
            swipe.from
        };
        let target = if settled == swipe.from { 0.0 } else if onward { -w } else { w };
        let spring = Spring::new(now, swipe.shift - target, velocity, FACE_PERIOD);
        swipe.spring = Some(spring);
        swipe.target = target;
        swipe.to = settled;
        self.dirty = true;
    }

    pub fn touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> Vec<Effect> {
        self.now = now;
        // Seen off from the power menu, the panel takes nothing: the card is
        // already on its way back and the page under it is going black.
        if self.power_exit.is_some() {
            return Vec::new();
        }
        // A finger on the dark panel lights nothing, as on the phone's own
        // system, where the power key lights it.
        if !self.display_on {
            return Vec::new();
        }
        // A sheet takes the finger while it stands: only its own keys
        // answer, and the page under it is left alone.
        if self.sheet.is_some() {
            return self.sheet_touch(ev, f, now);
        }
        // The power menu takes the finger while it shows, over everything
        // else on the panel.
        if self.power_menu.is_some() {
            return self.power_menu_touch(ev, f, now);
        }
        // The volume panel takes a finger landing on it: a track or the
        // switch answers, and a tap on the glass elsewhere puts it away.
        // What a finger drags along a track is its own from here on, as any
        // slider is.
        if let Touch::Down(x, y) = ev
            && self.volume_bar.is_some()
        {
            return self.volume_panel_touch(x, y, now);
        }
        // A finger on the charging screen sends it away for the page, and
        // does nothing else until it lifts. That screen stands over the lock,
        // so it is the one the finger reaches first.
        if self.charge_shown() {
            if matches!(ev, Touch::Down(..)) {
                self.dismiss_charging(now);
                self.drag = Drag::Waking;
            }
            return Vec::new();
        }
        // The lock takes the finger while it shows: it is swiped up away,
        // and nothing under it is touched.
        if self.locked() {
            return self.lock_touch(ev, now);
        }
        // A finger landing just after the panel lit only keeps it awake.
        if matches!(ev, Touch::Down(..)) && self.just_woke(now) {
            self.stir(now);
            self.drag = Drag::Waking;
            return Vec::new();
        }
        // A finger on the glass puts the keys' light out, whatever a key
        // held down was doing comes to nothing, and a page sliding comes to
        // rest.
        if matches!(ev, Touch::Down(..)) {
            if self.keys {
                self.keys = false;
                self.adjusting = false;
                self.dirty = true;
            }
            self.drop_button();
            self.end_turn();
        }
        self.stir(now);
        let t = self.theme();
        let m = t.m;
        let (slop, edge) = (m.dp(8.0), m.dp(20.0));
        // The panel pulled down takes the finger while it stands: its keys,
        // its brightness and its notices answer, and a pull on it moves it
        // rather than the page under it.
        if self.shade_shows(now) {
            return self.shade_touch(ev, now);
        }
        // A finger landing on the status bar pulls the panel down from it.
        // The bar itself is the grab, whole, rather than a strip drawn for
        // the purpose: a finger that finds an icon has found it.
        if let Touch::Down(_, y) = ev
            && y < m.status_cy * 2.0
        {
            self.tracker.clear();
            self.tracker.add(now, y);
            self.stir(now);
            self.shade_down();
            self.drag = Drag::Shade { from: y, last: y };
            return Vec::new();
        }
        let scroll = self.top().1;
        let mut fx = Vec::new();
        let pressing = self.pressing();
        match ev {
            Touch::Down(x, y) => {
                self.tracker.clear();
                self.tracker.add(now, y);
                let side = if x < edge {
                    Some(true)
                } else if x > m.w as f32 - edge {
                    Some(false)
                } else {
                    None
                };
                // The first page has nothing to go back to: its edges are
                // like the rest of it.
                let at_edge = side.filter(|_| self.stack.len() > 1);
                // A swipe in from an edge goes back over whatever the page is
                // doing: a finger landing there leaves a gliding or springing
                // page alone until it drags the page, and presses nothing.
                if at_edge.is_some() && self.in_motion(now) {
                    self.drag = Drag::Press { x, y, action: None, edge: at_edge, at: now };
                    return fx;
                }
                // A terminal's screen or the editor's text gliding stops
                // under a finger landing on it, which holds it to scroll on.
                let glided = self.glide.take().filter(|(glide, ..)| !glide.at(now).1);
                let history = self.frame.as_ref().and_then(|fr| fr.scrollback).filter(|(r, _)| r.contains(x, y + scroll));
                if let (Some(_), Some((_, pitch))) = (glided, history) {
                    self.drag = Drag::History { last: y, carry: 0.0, pitch };
                    return fx;
                }
                // Anywhere else it stops such a page and holds it; the
                // control under the finger is not pressed.
                if self.catch(now) {
                    self.drag = Drag::Scroll { last: y };
                    return fx;
                }
                // The `<` stays put while the page scrolls; under the bar
                // only it answers. A pinned strip answers where it shows,
                // over the rows beneath it, and so does a docked one.
                let back = widgets::back_rect(&t);
                let strip = self.frame.as_ref().and_then(|fr| fr.pinned.as_ref()).map(|p| (p, pinned_top(&t, p, scroll)));
                let on_strip = strip.filter(|&(p, top)| y >= top && y < top + p.canvas.h as f32);
                let dock = self.frame.as_ref().and_then(|fr| fr.dock.as_ref()).map(|d| (d, self.h as f32 - d.canvas.h as f32));
                let on_dock = dock.filter(|&(_, top)| y >= top);
                let button = self.frame.as_ref().and_then(|fr| fr.button).map(|(_, a)| (a, widgets::button_rect(&t)));
                let hit = if self.frame.as_ref().is_some_and(|fr| fr.back) && back.contains(x, y) {
                    Some((Action::Back, back))
                } else if let Some(button) = button.filter(|(_, r)| r.contains(x, y)) {
                    Some(button)
                } else if let Some((d, top)) = on_dock {
                    d.hits.iter().find(|h| h.rect.contains(x, y - top)).map(|h| (h.action, Rect { y: h.rect.y + top, ..h.rect }))
                } else if let Some((p, top)) = on_strip {
                    p.hits.iter().find(|h| h.rect.contains(x, y - top)).map(|h| (h.action, Rect { y: h.rect.y + top, ..h.rect }))
                } else if self.bar.on && y < widgets::bar_bottom(&t) {
                    None
                } else {
                    // A search bar folding away answers only on what is
                    // left of it.
                    let fr = self.frame.as_ref();
                    fr.and_then(|fr| fr.hit(x, y + scroll).filter(|h| fr.shown(h.rect, scroll).is_some_and(|r| r.contains(x, y))))
                        .map(|h| (h.action, h.rect))
                };
                // The timer's ruler takes the finger before the clock's
                // sideways face swipe does: pulling it sets a length, not
                // a face. It is only offered when a count is not running.
                let ruler = self
                    .frame
                    .as_ref()
                    .and_then(|fr| fr.clock_ruler)
                    .filter(|r| r.contains(x, y + scroll));
                if ruler.is_some() {
                    self.drag = Drag::Ruler { last_x: x, direction: 0 };
                    return fx;
                }
                let action = hit.map(|h| h.0);
                // A row of the page darkens behind its words a moment after
                // the finger lands on it, if the finger stays.
                let row = hit
                    .filter(|&(a, r)| self.frame.as_ref().is_some_and(|fr| fr.hits.contains(&Hit { rect: r, action: a })))
                    .filter(|&(a, r)| row_wide(&t, r) && !matches!(a, Action::Open(Screen::Search) | Action::Confirm(_) | Action::TypeInto(_)))
                    .map(|(_, r)| r);
                // A desktop icon darkens in its own square the same way,
                // whether it opens a page or has nothing behind it yet.
                let icon = hit.and_then(|(a, _)| self.frame.as_ref().and_then(|fr| fr.tiles.iter().find(|(t, _)| *t == a).map(|(_, r)| *r)));
                self.drag = match hit {
                    Some((Action::Slider(which), track)) => {
                        self.set_slider(&t, which, track, x);
                        Drag::Slider(which, track)
                    }
                    Some((Action::Confirm(which), track)) if self.on_knob(track, x) => {
                        let knob_x = track.x + (track.w - track.h) * self.state.confirm;
                        Drag::Confirm { which, track, grab: x - knob_x }
                    }
                    Some((Action::Pattern, _)) => {
                        self.state.pattern.clear();
                        self.trace(&t, x, y + scroll);
                        Drag::Pattern
                    }
                    _ => {
                        if let Some(rect) = icon {
                            self.icon_press = Some(Darken { rect, rise: now + ROW_PRESS_DELAY_MS, fall: None });
                        }
                        if let Some(rect) = row {
                            self.row_press = Some(Darken { rect, rise: now + ROW_PRESS_DELAY_MS, fall: None });
                            self.row_art = None;
                        }
                        Drag::Press { x, y, action, edge: at_edge, at: now }
                    }
                };
            }
            Touch::Move(x, y) => match std::mem::replace(&mut self.drag, Drag::Idle) {
                Drag::Press { x: x0, y: y0, action, edge: at_edge, at } => {
                    self.tracker.add(now, y);
                    let (dx, dy) = (x - x0, y - y0);
                    self.drag = match at_edge {
                        Some(from_left) if dx.abs() > slop && dx.abs() > dy.abs() => {
                            self.dirty = true;
                            Drag::Edge { start: x0, from_left, armed: false, x, cy: y0 - m.dp(CUE_LIFT_DP) }
                        }
                        // A finger pulled up from the foot of the screen
                        // brings the tasks up over whatever is shown, and,
                        // thrown on past them, takes the page off for the
                        // desktop. The desktop is the home of every page, so
                        // this asks nothing of what lies under the one shown.
                        // A page at work is not pulled off, as the back key
                        // will not leave it: the finger scrolls it instead.
                        // A finger dragged level across the desktop pulls
                        // its icons aside, and where it lets go turns to
                        // the page beside the one showing. A page's worth
                        // of icons is what a screen holds, so the desktop
                        // turns sideways rather than running on downwards.
                        _ if self.screen() == Screen::Desktop && crate::pages::desk_pages() > 1 && dx.abs() > slop && dx.abs() > dy.abs() => {
                            // The pull starts where the finger already is,
                            // not at nought, so the icons do not jump the
                            // slop's width on the first move.
                            self.state.desk_drag = dx;
                            self.dirty = true;
                            Drag::Page { start: x0 }
                        }
                        _ if self.face_slots().is_some() && dx.abs() > slop && dx.abs() > dy.abs() => {
                            // Dragged level across a page whose top words are
                            // tabs, its faces come along with the finger —
                            // the way the words name them, a pull to the left
                            // showing the one to the right. Every such page
                            // answers this one way; a new one only has to
                            // name its faces in `face_slots`.
                            self.start_face_swipe(now, dx < 0.0, x0, x);
                            Drag::Face { from: x0 }
                        }
                        _ if dy < -slop
                            && dy.abs() > dx.abs()
                            && y0 > m.h as f32 - m.dp(FOOT_DP)
                            && self.screen() != Screen::Desktop
                            && !self.working() =>
                        {
                            // Whatever the page was doing under the finger
                            // stops with it.
                            self.glide = None;
                            self.fling = None;
                            self.bounce = None;
                            self.pull = 0.0;
                            self.foot = Some(Foot { pull: (-dy).min(self.h as f32), to: 0.0, spring: None });
                            self.dirty = true;
                            Drag::Foot { from: y0 }
                        }
                        _ if dy.abs() > slop => {
                            let history = self.frame.as_ref().and_then(|fr| fr.scrollback).filter(|(r, _)| r.contains(x0, y0 + scroll));
                            if let Some((_, pitch)) = history {
                                self.scroll_history(y0, y, 0.0, pitch)
                            } else {
                                // A page still gliding or springing back is
                                // caught where it has got to; one at rest
                                // follows the finger from where it came down.
                                if !self.catch(now) {
                                    self.drag_by(y0 - y);
                                }
                                Drag::Scroll { last: y }
                            }
                        }
                        _ => Drag::Press { x: x0, y: y0, action, edge: at_edge, at },
                    };
                    if !matches!(self.drag, Drag::Press { .. }) {
                        self.let_press_go(now);
                    }
                }
                Drag::Scroll { last } => {
                    self.tracker.add(now, y);
                    self.drag_by(last - y);
                    self.drag = Drag::Scroll { last: y };
                }
                // The icons come along with the finger, as far as it has
                // pulled them. Letting go does the turning, in `Up`.
                Drag::Page { start } => {
                    let w = m.w as f32;
                    self.state.desk_drag = (x - start).clamp(-w, w);
                    self.dirty = true;
                    self.drag = Drag::Page { start };
                }
                Drag::Ruler { last_x, direction } => {
                    // Pulling left rolls the scale towards more time, so a
                    // minute passes each time the finger travels one
                    // minute's width. Each mark passed taps the motor.
                    let delta = (-(x - last_x) / m.dp(RULER_MINUTE_DP) * RULER_UNITS as f32).round() as i32;
                    let crossed = self.state.clock.drag_ruler(delta);
                    if crossed > 0 && self.state.vibrate {
                        for _ in 0..crossed {
                            fx.push(Effect::Haptic);
                        }
                    }
                    let direction = match delta.cmp(&0) {
                        std::cmp::Ordering::Greater => 1,
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => direction,
                    };
                    self.drag = Drag::Ruler { last_x: x, direction };
                    self.refresh();
                }
                Drag::Edge { start, from_left, armed, cy, .. } => {
                    let travel = if from_left { x - start } else { start - x };
                    let ready = travel > m.dp(BACK_SWIPE_DP);
                    // A tap of the motor as the `<` appears, as the system
                    // gives.
                    if ready && !armed && self.state.vibrate {
                        fx.push(Effect::Haptic);
                    }
                    self.drag = Drag::Edge { start, from_left, armed: ready, x, cy };
                    self.dirty = true;
                }
                Drag::Foot { from } => {
                    self.tracker.add(now, y);
                    if let Some(foot) = self.foot.as_mut() {
                        foot.pull = (from - y).clamp(0.0, self.h as f32);
                    }
                    self.drag = Drag::Foot { from };
                    self.dirty = true;
                }
                Drag::Slider(which, track) => {
                    self.set_slider(&t, which, track, x);
                    self.drag = Drag::Slider(which, track);
                }
                // A track of the volume panel: the card stays where it is,
                // so the track kept with the finger is the one it goes on
                // standing on, and every move holds the panel up a while
                // longer.
                Drag::Panel(which, track) => {
                    self.set_slider(&t, which, track, x);
                    self.volume_bar = Some(now + VOLUME_BAR_MS);
                    self.drag = Drag::Panel(which, track);
                }
                Drag::Confirm { which, track, grab } => {
                    let travel = (track.w - track.h).max(1.0);
                    self.state.confirm = ((x - grab - track.x) / travel).clamp(0.0, 1.0);
                    self.repaint_confirm(&t, f, which, track);
                    self.drag = Drag::Confirm { which, track, grab };
                }
                Drag::Pattern => {
                    self.trace(&t, x, y + scroll);
                    self.drag = Drag::Pattern;
                }
                Drag::History { last, carry, pitch } => {
                    self.tracker.add(now, y);
                    self.drag = self.scroll_history(last, y, carry, pitch);
                }
                Drag::Waking => self.drag = Drag::Waking,
                Drag::Held => self.drag = Drag::Held,
                Drag::Face { from } => {
                    self.tracker.add(now, x);
                    self.drag_face_swipe(from, x);
                    self.drag = Drag::Face { from };
                }
                // A finger on the lock, which is swiped by its own path in
                // `lock_touch` and never reaches here.
                Drag::Lock { .. } => {}
                // A finger on the panel, which is moved by its own path in
                // `shade_touch`; here only for a pull that began on the bar
                // and has not yet shown the panel at all.
                Drag::Shade { from, .. } => {
                    let open = (y - from) / self.shade_geom().panel_h().max(1.0);
                    self.shade_pull(open);
                    self.tracker.add(now, y);
                    self.drag = Drag::Shade { from, last: y };
                }
                Drag::Idle => {}
            },
            Touch::Up => match std::mem::replace(&mut self.drag, Drag::Idle) {
                // A pull on the panel let go: it stays out or goes back, by
                // how far it came and how quickly.
                Drag::Shade { .. } => {
                    let speed = self.tracker.velocity(now);
                    self.shade_release(now, speed);
                }
                Drag::Press { action: Some(a), .. } => {
                    if let Some(p) = &mut self.row_press {
                        p.lift(now);
                    }
                    // An icon let go is clean at once: the page grows out of
                    // its square, and a darkening still fading would be drawn
                    // into the desktop painted behind that page, where it
                    // could never fade.
                    self.icon_press = None;
                    fx.extend(self.act(a, now));
                }
                Drag::Press { x, y, action: None, .. } if self.screen() == Screen::Editor => self.place_caret(f, x, y + scroll),
                Drag::Face { .. } => {
                    // The faces settle on the one the finger pulled towards,
                    // or spring back to the one it came from. A last quick
                    // flick decides it even when the travel was short.
                    let speed = self.tracker.velocity(now);
                    self.release_face_swipe(now, speed);
                }
                Drag::History { pitch, .. } => {
                    // A finger lifted while still moving sends the rows
                    // gliding on.
                    let speed = self.tracker.velocity(now);
                    if speed.abs() >= m.dp(MIN_FLING_DP) {
                        let speed = speed.clamp(-m.dp(MAX_FLING_DP), m.dp(MAX_FLING_DP));
                        self.glide = Some((Fling::new(now, 0.0, speed, self.px_per_dp * 160.0), pitch, 0));
                    }
                }
                Drag::Scroll { .. } if self.pull != 0.0 => {
                    // A page let go past its end springs back.
                    self.bounce = Some(Spring::new(now, motion::rubber(self.pull, self.h as f32), 0.0, 0.4));
                    self.pull = 0.0;
                    self.dirty = true;
                }
                Drag::Scroll { .. } => {
                    // A finger lifted while still moving sends the page
                    // gliding on. One lifted slower leaves it where it is,
                    // but for a search bar part folded, which settles
                    // folded or unfolded; so does one lifted in the fold
                    // whose glide would end in it too.
                    let speed = -self.tracker.velocity(now);
                    let folding = |s: f32| self.frame.as_ref().is_some_and(|fr| fr.settle(s) != s);
                    let glide = (speed.abs() >= m.dp(MIN_FLING_DP)).then(|| {
                        let speed = speed.clamp(-m.dp(MAX_FLING_DP), m.dp(MAX_FLING_DP));
                        Fling::new(now, scroll, speed, self.px_per_dp * 160.0)
                    });
                    match glide {
                        Some(glide) if !(folding(scroll) && folding(glide.at(u64::MAX).0)) => self.fling = Some(glide),
                        _ => self.settle_fold(now),
                    }
                }
                // The desktop turns: dragged a quarter of a screen or more
                // it goes to the page beside the one it was on, short of
                // that the icons come back where they were.
                Drag::Ruler { direction, .. } => {
                    // The minute the scale lands on is the one set. An unset
                    // or finished timer starts there; a paused one waits.
                    self.state.clock.settle_ruler(direction, now);
                    self.refresh();
                }
                Drag::Page { .. } => {
                    let pull = self.state.desk_drag;
                    let w = m.w as f32;
                    let pages = crate::pages::desk_pages();
                    // Dragged left (negative) is the page after this one.
                    let turn = if pull.abs() > w * 0.25 { -pull.signum() as i32 } else { 0 };
                    let page = self.state.desk_page as i32 + turn;
                    self.state.desk_page = page.clamp(0, pages as i32 - 1) as usize;
                    self.state.desk_drag = 0.0;
                    self.dirty = true;
                }
                Drag::Edge { armed: true, .. } => fx.extend(self.back(now)),
                Drag::Foot { .. } => {
                    // A pull let go: thrown on fast, it takes the page off
                    // for the desktop; let go slower, it sinks back.
                    let speed = -self.tracker.velocity(now);
                    let h = self.h as f32;
                    let fling = h * FOOT_FLING;
                    if let Some(foot) = self.foot.as_mut() {
                        let to = if speed > fling { h } else { 0.0 };
                        foot.to = to;
                        // Going off the screen takes the lock's own time;
                        // coming back takes its quicker one. A page and the
                        // lock leave and come back at one pace.
                        let period = if to >= h - 0.5 { FOOT_AWAY } else { FOOT_BACK };
                        foot.spring = Some(Spring::new(now, foot.pull - to, speed, period));
                        self.dirty = true;
                    }
                }
                Drag::Edge { start, from_left, x, cy, .. } => {
                    // Short of going back, the cue sinks back into the edge.
                    let travel = if from_left { x - start } else { start - x };
                    let depth = widgets::back_cue_depth(&t, travel);
                    self.cue = Some((from_left, cy, Spring::new(now, depth, 0.0, 0.15)));
                    self.dirty = true;
                }
                Drag::Confirm { which, track, .. } => {
                    let done = self.state.confirm > SLID;
                    self.state.confirm = 0.0;
                    self.repaint_confirm(&t, f, which, track);
                    if done {
                        fx.extend(self.confirmed(which, now));
                    }
                }
                Drag::Pattern => fx.extend(self.drawn(now)),
                // Let go, the strength slider taps the motor as strongly as
                // it was left at.
                Drag::Slider(Slider::Vibration, _) if self.state.vibrate => fx.push(Effect::Haptic),
                _ => {}
            },
        }
        if self.pressing() != pressing {
            self.dirty = true;
        }
        // A key of the on-screen keyboard, or a choice of a list open over
        // the page, shows held down while the finger rests on it.
        let held = match self.drag {
            Drag::Press { action: Some(a @ (Action::Type(_) | Action::PickSecurity(_))), .. } => Some(a),
            _ => None,
        };
        if held != self.state.held {
            let was = std::mem::replace(&mut self.state.held, held);
            // Only the rows of what is let go and what is held change.
            let rects: Option<Vec<Rect>> = self
                .frame
                .as_ref()
                .and_then(|fr| [was, held].into_iter().flatten().map(|a| fr.hits.iter().find(|h| h.action == a).map(|h| h.rect)).collect());
            match rects {
                Some(rects) => rects.into_iter().for_each(|r| self.refresh_rows(r.y.floor() as u32, (r.y + r.h).ceil() as u32)),
                None => self.refresh(),
            }
        }
        fx
    }

    /// The button a finger holds down, which shows held at once: `<`, the
    /// search page's round button, or the home page's search bar.
    pub(super) fn pressing(&self) -> Option<Action> {
        let button = self.frame.as_ref().and_then(|fr| fr.button).map(|(_, a)| a);
        match self.drag {
            Drag::Press { action: Some(a @ (Action::Back | Action::CloseSearch | Action::Open(Screen::Search))), .. } => Some(a),
            Drag::Press { action: Some(a), .. } if Some(a) == button => Some(a),
            _ => None,
        }
    }

    /// The finger left the control it held, scrolling the page or swiping:
    /// a row or an icon not darkening yet never does, and one darkening
    /// fades back. An icon goes at once instead, because the desktop may be
    /// turning under the finger: the tiles come along with it, and a
    /// darkening square left fading would soon stand off its tile.
    fn let_press_go(&mut self, now: u64) {
        if let Some(p) = self.row_press {
            if now < p.rise {
                self.row_press = None;
                self.row_art = None;
            } else if p.fall.is_none() {
                self.row_press = Some(Darken { fall: Some(now), ..p });
            }
            self.dirty = true;
        }
        if self.icon_press.take().is_some() {
            self.dirty = true;
        }
    }

    /// No touchscreen answers: the keys work the pages, their light shown
    /// from now on.
    pub fn touchscreen_missing(&mut self) {
        self.keys = true;
        self.dirty = true;
    }

    /// A button went down. With the keys working the page, the volume keys
    /// move the light from control to control and the power key works the
    /// one lit as it comes back up; otherwise the first volume press lights
    /// a control, and the power key turns the panel off. The power key and
    /// volume down together take a screenshot instead: whichever of the two
    /// goes down first waits [`CHORD_MS`] for the other, or until it is let
    /// go, and then does what it does alone, as pressed when it went down.
    pub fn key(&mut self, key: Key, now: u64) -> Vec<Effect> {
        self.now = now;
        // Seen off from the power menu, the keys take nothing either: the
        // phone is on its way out.
        if self.power_exit.is_some() {
            return Vec::new();
        }
        // Any button stops a gliding page, and brings a sliding one to rest.
        self.fling = None;
        self.end_turn();
        // A sheet takes back and power while it stands; its first key
        // answers for them, as the timer's 确定 does.
        if self.sheet.is_some() {
            return self.sheet_key(key, now);
        }
        // The power menu takes every key while it shows: the volume keys
        // move the light, the power key takes what is lit, back puts it away.
        if self.power_menu.is_some() {
            return self.power_menu_key(key, now);
        }
        // The lock takes every key but the power key, which goes on lighting
        // and darkening the panel here as it does anywhere. A charging screen
        // over the lock is what the keys work while it shows. A volume key
        // still waits a moment for the power key to go down with it, so the
        // two together bring the power menu up from the lock as they do from
        // a page; alone, it does nothing there.
        if self.locked() && !self.charge_shown() && key != Key::Power {
            // The power key's partner goes on down to the pair below.
            let partner = matches!(self.chord, Some((Key::Power, at)) if now < at + CHORD_MS)
                && matches!(key, Key::VolumeUp | Key::VolumeDown);
            if !partner {
                if matches!(key, Key::VolumeUp | Key::VolumeDown) && self.chord.is_none() {
                    self.chord = Some((key, now));
                }
                return Vec::new();
            }
        }
        if let Some(first) = self.chord_partner(key, now) {
            // The two together do what neither does alone, whichever went
            // down first.
            match (first, key) {
                (Key::Power, Key::VolumeDown) | (Key::VolumeDown, Key::Power) => return self.screenshot(key, now),
                (Key::Power, Key::VolumeUp) | (Key::VolumeUp, Key::Power) => {
                    // The key held from before comes to nothing now the menu
                    // has the screen: a power key must not work the control
                    // it went down on, and a volume key must not go on moving
                    // the light under the menu.
                    self.button = Some((key, Press::Spent));
                    return self.show_power_menu(now);
                }
                _ => {}
            }
        }
        let mut fx = self.let_chord_go();
        if !self.display_on {
            // The power key lights the dark panel, and does only that when
            // it is let go, as on the phone's own system; the other keys
            // leave it dark. Coming back to the phone, it comes back to the
            // lock. Held on, it goes further, as the key does over a lit
            // page: the power menu, timed from the press that lit it.
            if key == Key::Power {
                self.wake(now);
                self.lock_now();
                self.button = Some((key, Press::PowerWake { at: now }));
                return fx;
            }
            self.button = Some((key, Press::Spent));
            return fx;
        }
        // A key other than power sends the charging screen away for the
        // page, and does nothing else.
        if key != Key::Power && self.charge_shown() {
            self.dismiss_charging(now);
            self.button = Some((key, Press::Spent));
            return fx;
        }
        // A button pressed just after the panel lit only keeps it awake, so
        // a power press meant to wake it does not turn it back off.
        if self.just_woke(now) {
            self.stir(now);
            self.button = Some((key, Press::Spent));
            return fx;
        }
        self.stir(now);
        if matches!(key, Key::Power | Key::VolumeDown | Key::VolumeUp) {
            self.chord = Some((key, now));
            return fx;
        }
        fx.extend(self.key_down(key, now));
        fx
    }

    /// The key waiting to make a two-key chord with `key`: the one that went
    /// down inside [`CHORD_MS`], or one still held down while the light is
    /// moved with it. With the volume keys steering the page the first key
    /// is often kept down while reaching for power, well past the little
    /// window, and that must still take the picture or bring the menu up.
    fn chord_partner(&self, key: Key, now: u64) -> Option<Key> {
        if let Some((first, at)) = self.chord
            && now < at + CHORD_MS
        {
            return Some(first);
        }
        match (key, self.button?) {
            (Key::Power, (Key::VolumeDown, Press::Volume { .. })) => Some(Key::VolumeDown),
            (Key::Power, (Key::VolumeUp, Press::Volume { .. })) => Some(Key::VolumeUp),
            (Key::VolumeDown | Key::VolumeUp, (Key::Power, Press::Power { .. } | Press::PowerIdle { .. } | Press::PowerWake { .. })) => Some(Key::Power),
            _ => None,
        }
    }

    /// The power key and volume down went down together: what the panel
    /// shows is saved as a picture, with a tap of the motor, and neither key
    /// does anything more until let go.
    fn screenshot(&mut self, key: Key, now: u64) -> Vec<Effect> {
        self.chord = None;
        self.stir(now);
        self.button = Some((key, Press::Spent));
        let mut fx = Vec::new();
        if self.state.vibrate {
            fx.push(Effect::Haptic);
        }
        fx.push(Effect::Screenshot { storage: self.state.place_open(Place::Storage) });
        fx
    }

    /// The key waiting for its partner in a screenshot does what it does
    /// alone, as pressed when it went down.
    pub(super) fn let_chord_go(&mut self) -> Vec<Effect> {
        match self.chord.take() {
            Some((key, at)) => self.key_down(key, at),
            None => Vec::new(),
        }
    }

    /// What a button that went down `at` does on its own: a key held is
    /// timed from when it went down, and the panel goes off when it is let
    /// go, so a press meant to turn it off still does, at once.
    fn key_down(&mut self, key: Key, at: u64) -> Vec<Effect> {
        match key {
            Key::Back => {
                self.button = Some((key, Press::Spent));
                self.act(Action::Back, at)
            }
            Key::VolumeUp | Key::VolumeDown => {
                let down = key == Key::VolumeDown;
                self.button = Some((key, Press::Volume { down, next: at + REPEAT_DELAY_MS }));
                self.volume(down, at)
            }
            // A press before the page it lands on is drawn is lost, rather
            // than taken for one on nothing.
            Key::Power if self.keys && self.frame.is_none() => {
                self.button = Some((key, Press::Spent));
                Vec::new()
            }
            Key::Power => {
                self.dirty = true;
                match self.keys.then(|| self.lit()).flatten() {
                    // Held on a swipe-to-confirm control, the key slides
                    // its knob along as a finger would.
                    Some(Spot { stop: Stop::Hit(Action::Confirm(which)), rect, .. }) => {
                        self.button = Some((key, Press::Slide { which, track: rect, at }))
                    }
                    Some(spot) => self.button = Some((key, Press::Power { spot, at })),
                    // Nothing lit: the key waits to see whether it is held,
                    // so what comes of it — the panel going off, or the
                    // power menu — comes as it is let go or held long enough.
                    None => self.button = Some((key, Press::PowerIdle { at })),
                }
                Vec::new()
            }
        }
    }

    /// A button came back up: the power key works the control it went down
    /// on, or confirms with the knob it slid far enough. Let go before its
    /// partner in a screenshot came, the key waiting for it first does what
    /// it does alone.
    pub fn key_up(&mut self, key: Key, now: u64) -> Vec<Effect> {
        // A key let go of while the phone is on its way out does nothing.
        if self.power_exit.is_some() {
            return Vec::new();
        }
        let mut fx = match self.chord {
            Some((first, _)) if first == key => self.let_chord_go(),
            _ => Vec::new(),
        };
        let Some((down, press)) = self.button else { return fx };
        if down != key {
            return fx;
        }
        self.button = None;
        self.dirty = true;
        match press {
            Press::Power { spot, .. } => fx.extend(self.work(spot, now)),
            Press::Slide { which, track, .. } => {
                let done = self.state.confirm > SLID;
                self.let_knob_go(which, track);
                if done {
                    fx.extend(self.confirmed(which, now));
                } else {
                    self.say(self.state.lang.pick(texts::toast::HOLD_POWER), now);
                }
            }
            // Let go before it was held long enough: the panel goes off,
            // which is what a press of the key has always done, with the
            // motor's two ticks. Held long enough it is already spent, and
            // the power menu has taken its place.
            Press::PowerIdle { .. } => {
                self.sleep(now);
                fx.push(Effect::Ticks);
            }
            // The press that lit a dark panel only lights it: let go, it
            // does nothing more. Held on, the power menu has taken its place.
            Press::PowerWake { .. } => {}
            Press::Volume { .. } | Press::Spent => {}
        }
        fx
    }

    /// Whatever the button held down was doing comes to nothing until it is
    /// let go: a knob it slid goes back.
    pub(super) fn drop_button(&mut self) {
        let Some((key, press)) = self.button else { return };
        if let Press::Slide { which, track, .. } = press {
            self.let_knob_go(which, track);
        }
        self.button = Some((key, Press::Spent));
    }

    /// Let go of a swipe-to-confirm control's knob, which goes back to the
    /// start of its track.
    fn let_knob_go(&mut self, which: Confirm, track: Rect) {
        self.state.confirm = 0.0;
        self.knob = Some((which, track));
        self.dirty = true;
    }

    fn on_knob(&self, track: Rect, x: f32) -> bool {
        let knob_x = track.x + (track.w - track.h) * self.state.confirm;
        x >= knob_x && x <= knob_x + track.h
    }

    fn repaint_confirm(&mut self, t: &Theme, f: &Fonts, which: Confirm, track: Rect) {
        self.paint_knob(t, f, which, track);
        self.dirty = true;
    }

    /// Draw the swipe-to-confirm control at `track` on the page again, its
    /// knob where it has come to.
    pub(super) fn paint_knob(&mut self, t: &Theme, f: &Fonts, which: Confirm, track: Rect) {
        let (label, danger) = pages::confirm_label(which, &self.state);
        if let Some(fr) = self.frame.as_mut() {
            fr.canvas.fill_rect(track.x, track.y, track.w, track.h, t.pal.bg);
            widgets::confirm(&mut fr.canvas, t, f, track.y, label, danger, self.state.confirm);
        }
        self.layer = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::color::over;

    #[test]
    fn a_row_held_darkens_behind_its_words_a_moment_after_the_finger_lands() {
        let (f, i) = (fonts(), info());
        let t = Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Home, W, H, PPD);
        let mut c = Canvas::new(W, H);
        let row = app.frame(&f, &i).hits.iter().find(|h| h.action == Action::Open(Screen::Backup)).expect("on the home page").rect;
        // The card between the row's words and its arrow.
        let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        let ink = darkest(&c, row);
        assert_eq!(grey_at(&c, x, y), t.pal.card.r);

        app.touch(Touch::Down(x, y), &f, 1000);
        let rise = 1000 + ROW_PRESS_DELAY_MS;
        assert_eq!(app.deadline(1000), Some(rise), "nothing moves until the row starts darkening");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        assert!(app.row_art.is_none(), "a finger that may yet scroll does not wait for the row drawn held down");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", rise);
        assert_eq!(grey_at(&c, x, y), t.pal.card.r, "it has not started yet");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", rise + 30);
        let partway = grey_at(&c, x, y);
        assert!(partway < 255 && partway > 225, "darkening after 30 ms: {partway}");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 2000);
        assert_eq!(grey_at(&c, x, y), 225, "as dark as the stock settings' row held down");
        assert_eq!(darkest(&c, row), ink, "its words keep their ink");
        assert_ne!(app.deadline(2000), Some(0), "held still, nothing moves");

        // The finger moves on to scroll: the row fades back.
        app.touch(Touch::Move(x, y + t.m.dp(30.0)), &f, 2100);
        assert_eq!(app.deadline(2100), Some(0));
        app.touch(Touch::Up, &f, 2200);
        app.tick(3000);
        assert!(app.row_press.is_none() && app.row_art.is_none(), "faded, it is let go");
        assert_eq!(app.screen(), Screen::Home, "a scroll opens nothing");
    }

    #[test]
    fn a_row_scrolled_away_from_before_it_darkens_never_shows_held() {
        let (f, i) = (fonts(), info());
        let t = Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Home, W, H, PPD);
        let row = app.frame(&f, &i).hits.iter().find(|h| h.action == Action::Open(Screen::Backup)).expect("on the home page").rect;
        let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
        app.touch(Touch::Down(x, y), &f, 1000);
        app.touch(Touch::Move(x, y + t.m.dp(30.0)), &f, 1050);
        assert!(app.row_press.is_none());
    }

    #[test]
    fn a_row_tapped_quickly_shows_held_from_the_lift_and_fades_back() {
        let (f, i) = (fonts(), info());
        let t = Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Settings, W, H, PPD);
        let mut c = Canvas::new(W, H);
        let row = app.frame(&f, &i).hits.iter().find(|h| h.action == Action::Toggle(Switch::Vibrate)).expect("on the settings page").rect;
        let (x, y) = (W as f32 * 0.6, row.y + row.h / 2.0);
        tap_at(&mut app, &f, &i, Action::Toggle(Switch::Vibrate), 1000);
        assert!(!app.state.vibrate, "the tap worked the switch");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1040);
        let held = grey_at(&c, x, y);
        assert!(held < t.pal.card.r, "held from the lift: {held}");
        app.tick(1040 + QUICK_TAP_MS + 1000);
        assert!(app.row_press.is_none(), "and let go once faded");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 2200);
        assert_eq!(grey_at(&c, x, y), t.pal.card.r);
    }

    #[test]
    fn the_back_control_and_the_search_buttons_darken_at_once_under_a_finger() {
        let (f, i) = (fonts(), info());
        let t = Theme::light(W, H, PPD);
        let mut c = Canvas::new(W, H);

        // `<`: a disc behind it, the `<` lighter on it.
        let mut app = opened(Screen::Mount, &f, &i);
        let (cx, cy, r) = widgets::back_disc(&t);
        let (bx, by) = widgets::back_rect(&t).centre();
        let around = Rect::new(cx - r, cy - r, r * 2.0, r * 2.0);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 5000);
        assert_eq!(grey_at(&c, cx + r * 0.6, cy), t.pal.bg.r);
        assert_eq!(darkest(&c, around), 0);
        app.touch(Touch::Down(bx, by), &f, 6000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 6000);
        let disc = over(t.pal.button_press, t.pal.bg).r;
        assert_eq!(grey_at(&c, cx + r * 0.6, cy), disc, "darker at once");
        let arrow = darkest(&c, around);
        assert!(arrow.abs_diff(85) <= 2, "the stock `<` held down is 85, not {arrow}");
        // A finger moving off lets it go.
        app.touch(Touch::Move(bx, by + t.m.dp(40.0)), &f, 6050);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 6050);
        assert_eq!(grey_at(&c, cx + r * 0.6, cy), t.pal.bg.r);
        app.touch(Touch::Up, &f, 6100);
        assert_eq!(app.screen(), Screen::Mount);

        // The search page's round button.
        let mut app = App::new(Screen::Search, W, H, PPD);
        app.frame(&f, &i);
        let (_, button) = widgets::search_layout(&t);
        let (x, y) = button.centre();
        app.touch(Touch::Down(x, y), &f, 1000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        assert_eq!(grey_at(&c, x, y + button.h * 0.35), over(t.pal.button_press, t.pal.search).r);
        let cross = darkest(&c, button);
        assert!(cross.abs_diff(70) <= 2, "the stock cross held down is 70, not {cross}");

        // The home page's search bar, clear of its words.
        let mut app = App::new(Screen::Home, W, H, PPD);
        let bar = app.frame(&f, &i).hits.iter().find(|h| h.action == Action::Open(Screen::Search)).expect("on the home page").rect;
        let (x, y) = (bar.x + bar.w * 0.8, bar.y + bar.h / 2.0);
        app.touch(Touch::Down(x, y), &f, 1000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        assert_eq!(grey_at(&c, x, y), over(t.pal.button_press, t.pal.search).r);
    }

    #[test]
    fn dragging_level_across_the_clock_turns_it_to_the_other_face() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        assert_eq!(app.state.clock.face, ClockFace::Stopwatch, "it opens on the watch");
        // A pull to the left shows the face to its right once the strip has
        // settled — it follows the finger until then, not changing at the
        // moment it passes the slop.
        app.touch(Touch::Down(900.0, 1600.0), &f, 0);
        app.touch(Touch::Move(100.0, 1605.0), &f, 10);
        assert!(app.face_swipe.is_some(), "the two faces are held under the finger");
        app.touch(Touch::Up, &f, 20);
        app.tick(10_000);
        assert_eq!(app.state.clock.face, ClockFace::Timer);
        // And a pull to the right brings the watch back.
        app.touch(Touch::Down(100.0, 1600.0), &f, 10_000);
        app.touch(Touch::Move(900.0, 1605.0), &f, 10_010);
        app.touch(Touch::Up, &f, 10_020);
        app.tick(20_000);
        assert_eq!(app.state.clock.face, ClockFace::Stopwatch);
    }

    /// A quick flick throws the strip to the next face even when it never
    /// travels far; the same travel with a pause before lifting does not.
    #[test]
    fn a_flick_commits_a_face_even_when_it_travels_little() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        // 40 px is past the 28 px slop at 3.5 px/dp but nowhere near the
        // 32%-of-1440 commit mark; only the speed can carry it over.
        app.touch(Touch::Down(900.0, 1600.0), &f, 0);
        app.touch(Touch::Move(860.0, 1600.0), &f, 10);
        app.touch(Touch::Up, &f, 10);
        app.tick(10_000);
        assert_eq!(app.state.clock.face, ClockFace::Timer, "the flick throws it onward");

        app.touch(Touch::Down(900.0, 1600.0), &f, 20_000);
        app.touch(Touch::Move(860.0, 1600.0), &f, 20_010);
        app.touch(Touch::Up, &f, 20_060);
        app.tick(30_000);
        assert_eq!(app.state.clock.face, ClockFace::Timer, "a rest before lifting springs back to where it was");
    }

    /// A short pull short of the mark springs back and leaves the face as it
    /// was, and a pull past the end shows the ground and comes back too.
    #[test]
    fn a_faces_strip_springs_back_when_it_does_not_commit() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        // Past the slop, and the finger rests before it lifts: not far
        // enough, and no flick to carry it over either.
        app.touch(Touch::Down(900.0, 1600.0), &f, 0);
        app.touch(Touch::Move(860.0, 1601.0), &f, 10);
        app.touch(Touch::Up, &f, 60);
        app.tick(10_000);
        assert_eq!(app.state.clock.face, ClockFace::Stopwatch, "a short pull springs back");
        // At the first face, a pull to the right has no neighbour: it shows
        // the ground and springs back rather than wrapping to the last.
        app.touch(Touch::Down(100.0, 1600.0), &f, 10_000);
        app.touch(Touch::Move(900.0, 1601.0), &f, 10_010);
        assert!(app.face_swipe.as_ref().is_some_and(|s| s.to == s.from), "there is no neighbour past the first face");
        app.touch(Touch::Up, &f, 10_020);
        app.tick(20_000);
        assert_eq!(app.state.clock.face, ClockFace::Stopwatch, "and it comes back to the one face");
    }

    #[test]
    fn a_drag_down_the_clock_leaves_the_face_shown() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        // Nothing to scroll on a watch that has taken no entry, so the
        // page stays as it is, whichever way the finger goes.
        app.touch(Touch::Down(720.0, 1000.0), &f, 0);
        app.touch(Touch::Move(720.0, 1400.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.state.clock.face, ClockFace::Stopwatch);
    }


    #[test]
    fn swipe_from_the_left_edge_goes_back() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        app.touch(Touch::Move(90.0, 1605.0), &f, 0);
        app.touch(Touch::Move(320.0, 1610.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.screen(), Screen::Home);
    }

    #[test]
    fn a_short_edge_swipe_does_not_go_back() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        app.touch(Touch::Move(80.0, 1600.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.screen(), Screen::Reboot);
    }

    #[test]
    fn the_first_page_has_no_edge_swipe_back() {
        let f = fonts();
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        app.touch(Touch::Move(80.0, 1600.0), &f, 8);
        assert!(app.touch(Touch::Move(400.0, 1600.0), &f, 16).is_empty(), "no tap of the motor");
        assert!(!matches!(app.drag, Drag::Edge { .. }), "no cue");
        app.touch(Touch::Up, &f, 24);
        assert_eq!(app.screen(), Screen::Home);
        assert_eq!(app.deadline(24), app.idle_due(), "nothing left on screen to settle");
    }

    #[test]
    fn the_back_cue_taps_once_as_it_arms_and_sinks_when_let_go() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        let ready = app.theme().m.dp(BACK_SWIPE_DP);
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        assert!(app.touch(Touch::Move(80.0, 1600.0), &f, 8).is_empty());
        assert_eq!(app.touch(Touch::Move(20.0 + ready, 1600.0), &f, 16), vec![Effect::Haptic]);
        assert!(app.touch(Touch::Move(60.0 + ready, 1600.0), &f, 24).is_empty(), "one tap while armed");
        app.touch(Touch::Move(40.0, 1600.0), &f, 32);
        app.touch(Touch::Up, &f, 40);
        assert_eq!(app.screen(), Screen::Reboot, "pulled back in, it does not go back");
        assert_eq!(app.deadline(40), Some(0), "the cue is sinking back");
        app.tick(1000);
        assert_eq!(app.deadline(1000), app.idle_due());
    }

    #[test]
    fn the_back_cue_stays_quiet_with_touch_vibration_off() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        app.state.vibrate = false;
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        app.touch(Touch::Move(80.0, 1600.0), &f, 8);
        assert!(app.touch(Touch::Move(400.0, 1600.0), &f, 16).is_empty());
    }

    #[test]
    fn the_back_cue_keeps_to_the_height_the_swipe_began_at() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        let at = 1600.0 - app.theme().m.dp(CUE_LIFT_DP);
        app.touch(Touch::Down(10.0, 1600.0), &f, 0);
        app.touch(Touch::Move(80.0, 1610.0), &f, 8);
        app.touch(Touch::Move(200.0, 1300.0), &f, 16);
        app.touch(Touch::Move(150.0, 1900.0), &f, 24);
        let Drag::Edge { cy, .. } = app.drag else { panic!("still swiping in from the edge") };
        assert_eq!(cy, at, "the finger wandered up and down");
        app.touch(Touch::Move(40.0, 2100.0), &f, 32);
        app.touch(Touch::Up, &f, 40);
        assert_eq!(app.cue.as_ref().map(|c| c.1), Some(at), "it sinks back where it was");
    }

    /// A desktop icon darkens in the square it is drawn on, a moment after
    /// the finger lands, and is gone the moment the finger moves on: the
    /// tiles come along with a turning desktop, and a square left fading
    /// would soon stand off its tile.
    #[test]
    fn a_desktop_icon_darkens_in_its_own_square_while_held() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let mut c = Canvas::new(W, H);
        let tile = app.frame(&f, &i).tiles.iter().find(|(a, _)| *a == Action::Open(Screen::Clock)).expect("the clock's tile").1;
        let (x, y) = tile.centre();
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        let plain = grey_at(&c, x, y);

        app.touch(Touch::Down(x, y), &f, 1000);
        assert!(app.icon_press.is_some(), "the square is kept from the moment the finger lands");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000 + ROW_PRESS_DELAY_MS);
        assert_eq!(grey_at(&c, x, y), plain, "nothing of it darkens yet");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 2000);
        let held = grey_at(&c, x, y);
        assert!(held < plain, "held down: {held} of {plain}");
        assert_ne!(app.deadline(2000), Some(0), "held still, nothing moves");

        app.touch(Touch::Move(x, y - 120.0), &f, 2100);
        assert!(app.icon_press.is_none(), "gone as the finger moves, not fading in place");
        assert_eq!(app.screen(), Screen::Desktop, "a scroll opens nothing");
    }

    /// A tile with nothing behind it answers the finger the same way: it
    /// darkens, and letting it go opens nothing.
    #[test]
    fn a_tile_with_nothing_behind_it_darkens_too() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let tile = app.frame(&f, &i).tiles.iter().find(|(a, _)| *a == Action::NotReady("weather")).expect("the weather's tile").1;
        let (x, y) = tile.centre();
        app.touch(Touch::Down(x, y), &f, 1000);
        assert!(app.icon_press.is_some(), "a tile with nothing behind it still answers");
        app.touch(Touch::Up, &f, 1100);
        assert!(app.icon_press.is_none(), "and is clean the moment it is let go");
        assert_eq!(app.screen(), Screen::Desktop, "opening nothing in its place");
    }

    /// A page opened from the desktop with no icon to come from grows out of
    /// a square at the middle of the screen, the way one out of an icon
    /// grows out of that icon, and covers the screen in the end.
    #[test]
    fn a_page_opened_without_an_icon_grows_out_of_the_middle() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        // No icon on the desktop opens this page.
        app.open(Screen::Logs);
        // The first frame sets the spring going, the page still a nothing at
        // the middle; a moment later it has grown out of it.
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        assert_eq!(grey_at(&c, 4.0, 4.0), 0, "nothing of it at the corner yet");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 150);
        assert_ne!(grey_at(&c, W as f32 / 2.0, H as f32 / 2.0), 0, "the page shows at the middle");
        assert_eq!(grey_at(&c, 4.0, 4.0), 0, "and has not reached the corner");

        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);
        assert_ne!(grey_at(&c, 4.0, 4.0), 0, "grown whole, it covers the screen");
    }

    /// Going back to the desktop with no icon to fold into shrinks to the
    /// middle of the screen, the desktop showing round it.
    #[test]
    fn a_page_going_back_without_an_icon_folds_to_the_middle() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        app.open(Screen::Logs);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);

        let _ = app.back(3000);
        // The frame the spring is set going on stands whole; a moment later
        // the page has folded in towards the middle.
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3150);
        assert_eq!(grey_at(&c, 4.0, 4.0), 0, "the desktop shows at the corner");
        assert_ne!(grey_at(&c, W as f32 / 2.0, H as f32 / 2.0), 0, "the page still stands at the middle");
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 6000);
        assert!(app.turn.is_none(), "come to rest, the desktop is left alone");
    }

    /// A slow pull from a page's foot has nowhere to stop any more: it sinks
    /// back, and the page it started on is still the one showing. There is
    /// no tasks' height to hover over.
    #[test]
    fn a_slow_pull_from_the_foot_has_no_tasks_to_stop_at() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        let x = W as f32 / 2.0;
        let from = H as f32 - 10.0;
        app.touch(Touch::Down(x, from), &f, 0);
        // Well past the old tasks' height, but slow: 40 px a frame at
        // 20 ms, under half a screen a second.
        for k in 1..=50 {
            app.touch(Touch::Move(x, from - k as f32 * 40.0), &f, k as u64 * 20);
        }
        app.touch(Touch::Up, &f, 1020);
        assert_eq!(app.foot.as_ref().map(|foot| foot.to), Some(0.0), "back down, not held at a tasks' height");
        app.tick(20_000);
        assert!(app.foot.is_none());
        assert_eq!(app.screen(), Screen::Clock, "the page was never taken off");
    }

    /// The desktop is not a page to leave, and has no tasks to raise: a pull
    /// at its foot does nothing at all.
    #[test]
    fn a_pull_at_the_desktop_foot_does_nothing() {
        let f = fonts();
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let x = W as f32 / 2.0;
        let from = H as f32 - 10.0;
        app.touch(Touch::Down(x, from), &f, 0);
        app.touch(Touch::Move(x, from - 600.0), &f, 20);
        app.touch(Touch::Up, &f, 40);
        assert!(app.foot.is_none(), "nothing stands up from the foot");
        assert_eq!(app.screen(), Screen::Desktop);
    }

    /// A flick still takes the page off for the desktop, at the lock's pace.
    #[test]
    fn a_flick_at_the_foot_still_leaves_for_the_desktop() {
        let f = fonts();
        let mut app = App::new(Screen::Clock, W, H, PPD);
        let x = W as f32 / 2.0;
        let from = H as f32 - 10.0;
        app.touch(Touch::Down(x, from), &f, 0);
        app.touch(Touch::Move(x, from - 500.0), &f, 10);
        app.touch(Touch::Move(x, from - 1500.0), &f, 20);
        app.touch(Touch::Move(x, from - 2500.0), &f, 30);
        app.touch(Touch::Up, &f, 40);
        assert_eq!(app.foot.as_ref().map(|foot| foot.to), Some(H as f32), "the page is on its way off");
        app.tick(10_000);
        assert_eq!(app.screen(), Screen::Desktop);
    }
}
