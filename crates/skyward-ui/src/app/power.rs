//! The panel: lit, dimmed and off, what lights it again, and the charging
//! screen a cable going in brings up.

use super::*;

/// The power menu as it shows: the entry the volume keys are on, once they
/// have been pressed, and when it came up, for the soft ground and the card
/// to rise with. Its card is the common sheet's — the same shape, the same
/// keys — but its ground is softened, not darkened. That is the one thing
/// that tells it from the confirmations it opens.
pub(super) struct PowerMenu {
    pub(super) chosen: usize,
    /// Whether the volume keys have been pressed while it stands. A menu
    /// that comes up with one entry already lit looks like one already
    /// answered; the light waits for a key to ask for it.
    pub(super) lit: bool,
    pub(super) since: u64,
}

/// The four ways out of the power menu, in the order shown: the three that
/// ask for a word first, and cancel, which only puts the menu away. The
/// restart card holds recovery and bootloader as its own whole-row keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerChoice {
    PowerOff,
    Restart,
    Suspend,
    Cancel,
}

/// Leaving from the power menu's own confirmations: the card falls back as
/// the page behind it goes black, and the phone leaves only once the black
/// is whole. Every other way out — the recovery's power page, a restart to
/// the other slot — still leaves at once, on a screen that simply goes dark.
pub(super) struct PowerExit {
    pub(super) target: Target,
    pub(super) since: u64,
    /// Whether the phone has already been asked to leave, so the black-out
    /// standing after it asks only once.
    pub(super) fired: bool,
}

/// How long the page takes to go black, and the beat it stays black before
/// the phone leaves. The black takes exactly as long as the card's own fall,
/// so the card is out of sight just as the black is whole.
pub(super) const BLACK_MS: u64 = dialog::FALL_MS;
pub(super) const BLACK_HOLD_MS: u64 = 120;

/// What is left of the page `ms` after the black-out began, from the dim it
/// stood in under the card to nothing at all. The card is drawn after this,
/// so it alone keeps its colour on the way down.
pub(super) fn black_level(ms: u64, span: u64) -> f32 {
    dialog::DIM * (1.0 - dialog::eased(ms, span))
}

const POWER_TITLE: Text = tri("电源选项", "電源選項", "Power options");
const POWER_CHOICES: [(PowerChoice, Text); 4] = [
    (PowerChoice::PowerOff, tri("关机", "關機", "Power off")),
    (PowerChoice::Restart, tri("重新启动", "重新啟動", "Restart")),
    (PowerChoice::Suspend, tri("挂起", "掛起", "Suspend")),
    (PowerChoice::Cancel, tri("取消", "取消", "Cancel")),
];

/// The menu's keys as a sheet's rows: a whole row for each of the four.
fn power_rows() -> dialog::SheetRows {
    POWER_CHOICES.iter().map(|(choice, name)| vec![(*name, Action::PowerMenuChoice(*choice))]).collect()
}

/// The words of the sheet a choice opens, and the keys on it. Power off and
/// restart go on to leave the system once confirmed, through the black-out
/// `PowerExit` stands for; suspend is not wired up, so its 确定 does no more
/// than its 取消.
pub(super) fn power_confirm(choice: PowerChoice, lang: crate::lang::Lang) -> (Text, String, dialog::SheetRows) {
    let ok: Text = tri("确定", "確定", "OK");
    let cancel: Text = tri("取消", "取消", "Cancel");
    match choice {
        PowerChoice::PowerOff => (
            tri("关机", "關機", "Power off"),
            lang.among("您的手机将会关机。", "您的手機將會關機。", "Your phone will power off.").to_string(),
            vec![vec![(ok, Action::PowerExit(Target::PowerOff)), (cancel, Action::SheetCancel)]],
        ),
        PowerChoice::Restart => (
            tri("重新启动", "重新啟動", "Restart"),
            lang.among("您的手机将会重新启动。", "您的手機將會重新啟動。", "Your phone will restart.").to_string(),
            vec![
                vec![(ok, Action::PowerExit(Target::System)), (cancel, Action::SheetCancel)],
                vec![(tri("重启到Recovery", "重啟到Recovery", "Restart to recovery"), Action::PowerExit(Target::Recovery))],
                vec![(tri("重启到Bootloader", "重啟到Bootloader", "Restart to bootloader"), Action::PowerExit(Target::Bootloader))],
            ],
        ),
        PowerChoice::Suspend => (
            tri("确定要挂起吗？", "確定要掛起嗎？", "Suspend?"),
            lang.among("挂起后，Skyward-Extra当前状态将保留，随后您可以启动至其他操作系统，只要不关机再启动回Skyward-Extra，就会恢复挂起之前的状态。", "掛起後，Skyward-Extra當前狀態將保留，隨後您可以啟動至其他操作系統，只要不關機再啟動回Skyward-Extra，就會恢復掛起之前的狀態。", "Once suspended, Skyward-Extra keeps its current state. Start another system, then return to Skyward-Extra without powering off and the state before suspending is restored.").to_string(),
            vec![vec![(ok, Action::SheetCancel), (cancel, Action::SheetCancel)]],
        ),
        PowerChoice::Cancel => unreachable!("cancel never opens a sheet"),
    }
}

impl App {
    pub fn display_on(&self) -> bool {
        self.display_on
    }

    /// The backlight at `now`: off, or this share of the brightness at the
    /// slider's top, which holds as the content goes dark after the panel
    /// is told to go off.
    pub fn panel(&self, now: u64) -> Option<f32> {
        if !self.display_on {
            return self.darkening.filter(|_| self.curtain.level(now) > 0.0);
        }
        let (full, level) = (self.state.brightness, self.bright.level(now));
        Some(full * level + dim_level(full) * (1.0 - level))
    }

    /// Something was touched or pressed: the panel stays at full
    /// brightness for the screen-off time from now.
    pub(super) fn stir(&mut self, now: u64) {
        self.active = Some(now);
        self.bright.set(true, now);
    }

    /// Light the panel, as if just touched: the content comes up from as
    /// dark as it is.
    pub(super) fn wake(&mut self, now: u64) {
        if !self.display_on {
            self.display_on = true;
            self.dirty = true;
            self.darkening = None;
            self.curtain.set(true, now);
            // A panel lit to tell something shows that thing, not the lock:
            // erasing, installing and backing up all light the panel when
            // they end, and what they end with is to be read and tapped at
            // once. The lock is for coming back to the phone, so the two
            // ways of coming back put it up again below.
            self.lock = None;
        }
        self.woke_at = Some(now);
        self.stir(now);
    }

    /// A charger or a computer was attached, `cable_in`, or taken away: the
    /// dark panel lights, and a lit one stays at full brightness for the
    /// screen-off time from now, as on the phone's own system. A cable
    /// going in brings up the charging screen, over the page or on the
    /// panel lit for it, which then goes dark again.
    pub fn plugged(&mut self, now: u64, cable_in: bool) {
        if cable_in {
            let dark = !self.display_on;
            let mut shown = self.charging.take().map_or(Fade::hidden(CHARGE_SHOW, CHARGE_HIDE), |c| c.shown);
            if dark {
                shown.jump(true);
            } else {
                shown.set(true, now);
            }
            self.charging = Some(Charging { since: now, shown, dark_after: dark });
            self.dirty = true;
        }
        if self.display_on {
            self.stir(now);
        } else {
            // A cable coming in lights the dark panel the way the power key
            // does, behind the lock as well: the charging screen stands over
            // the lock, and the page waits behind both.
            self.wake(now);
            self.lock_now();
        }
    }

    /// The charging screen has gone, and what was drawn for it with it.
    pub(super) fn end_charging(&mut self) {
        self.charging = None;
        self.charge_art = None;
        self.charge_canvas = None;
    }

    /// Whether the charging screen shows, or is coming up.
    pub(super) fn charge_shown(&self) -> bool {
        self.charging.as_ref().is_some_and(|c| c.shown.on)
    }

    /// Send the charging screen away for the page, and keep the panel lit.
    pub(super) fn dismiss_charging(&mut self, now: u64) {
        if let Some(c) = &mut self.charging {
            c.shown.set(false, now);
            c.dark_after = false;
        }
        self.stir(now);
    }

    /// Whether the panel lit so recently that a touch or key should only
    /// keep it awake.
    pub(super) fn just_woke(&self, now: u64) -> bool {
        self.woke_at.is_some_and(|w| now.saturating_sub(w) < WAKE_GUARD_MS)
    }

    /// Turn the panel off: the content goes dark under the backlight it has
    /// now, then the panel goes off. A finger on the glass lets go of what
    /// it held, and the page stops where it is.
    pub(super) fn sleep(&mut self, now: u64) {
        self.darkening = self.panel(now);
        self.curtain.set(false, now);
        self.display_on = false;
        self.power_menu = None;
        self.power_blur = None;
        // The panel comes back on behind the lock, whatever it showed: the
        // page under it is kept as it was left, and lit again as it was.
        self.lock_now();
        self.bright = full_brightness();
        match std::mem::replace(&mut self.drag, Drag::Idle) {
            Drag::Confirm { .. } => {
                self.state.confirm = 0.0;
                self.refresh();
            }
            Drag::Pattern => {
                self.state.pattern.clear();
                self.refresh();
            }
            _ => {}
        }
        self.drop_button();
        self.adjusting = false;
        self.fling = None;
        self.bounce = None;
        self.seek = None;
        self.pull = 0.0;
        self.cue = None;
        self.morph = None;
        self.row_press = None;
        self.row_art = None;
        self.foot = None;
        // The panel pulled down goes with the light, whole: it is not out
        // under the lock, and nothing of it is kept for when it lights.
        self.shade = None;
        self.banner = None;
        self.end_turn();
    }

    /// A page sliding over another comes to rest at once.
    pub(super) fn end_turn(&mut self) {
        if let Some(turn) = self.turn.take() {
            self.backdrop = Some(turn.left);
            self.dirty = true;
        }
    }

    /// The power key and volume up went down together, or the power key was
    /// held with nothing lit: the power menu comes up over the page, with a
    /// tap of the motor.
    pub(super) fn show_power_menu(&mut self, now: u64) -> Vec<Effect> {
        self.chord = None;
        self.power_menu = Some(PowerMenu { chosen: 0, lit: false, since: now });
        self.power_blur = None;
        // The volume panel goes with it: the menu's ground is a blurred copy
        // of the page, and a second card left on it would be blurred too.
        self.volume_bar = None;
        self.dirty = true;
        self.stir(now);
        if self.state.vibrate { vec![Effect::Haptic] } else { Vec::new() }
    }

    /// The power menu goes, and the phone goes on as it was.
    pub(super) fn hide_power_menu(&mut self) {
        self.power_menu = None;
        self.power_blur = None;
        self.dirty = true;
    }

    /// The black-out a confirmed 关机 or 重新启动 leaves behind: the page,
    /// the menu under the card and everything else goes black as the card
    /// falls. It is laid down before the card is drawn, so the card alone
    /// keeps its colour while the rest goes.
    pub(super) fn compose_power_exit(&self, out: &mut Canvas, now: u64) {
        if let Some(exit) = &self.power_exit {
            out.dim(black_level(now.saturating_sub(exit.since), BLACK_MS));
        }
    }

    /// The phone did not go after all — a property that would not take its
    /// write: the black-out is lifted and the page stands as it was, so the
    /// word about it can be read.
    pub fn leave_failed(&mut self) {
        if self.power_exit.take().is_some() {
            self.sheet = None;
            self.dirty = true;
        }
    }

/// A finger on the power menu: a tap on an entry takes it, a tap beside the
/// card puts the menu away and does nothing else. The card is the common
/// sheet's; a finger landing while it rises finds the keys where they were.
pub(super) fn power_menu_touch(&mut self, ev: Touch, f: &Fonts, now: u64) -> Vec<Effect> {
    let Touch::Down(x, y) = ev else { return Vec::new() };
    let t = self.theme();
    let shown = self.power_menu.as_ref().map_or(0.0, |m| dialog::shown_sheet(m.since, None, now));
    let rows = power_rows();
    if let Some(action) = dialog::sheet_hit(&t, f, &rows, "", shown, x, y) {
        return self.act(action, now);
    }
    if dialog::sheet_miss(&t, f, &rows, "", shown, x, y) {
        self.hide_power_menu();
    }
    Vec::new()
}

/// The keys while the power menu shows: the volume keys move the light
/// down the list and up it, the power key takes what is lit, back puts the
/// menu away. Taking a key hands the choice to `act`, which puts the menu
/// away and opens the sheet that asks for the word.
pub(super) fn power_menu_key(&mut self, key: Key, now: u64) -> Vec<Effect> {
    let volume = matches!(key, Key::VolumeUp | Key::VolumeDown);
    // Turned to the volume, the volume keys are the volume's: they neither
    // light an entry nor move the light on one, as they move nothing on a
    // page either.
    if volume && !self.state.volume_keys {
        return Vec::new();
    }
    let Some(menu) = self.power_menu.as_mut() else { return Vec::new() };
    if volume && !menu.lit {
        // The first press only puts the light on the entry the menu opens
        // with, as the first key on a page lights the first control in view.
        // Each after moves it on.
        menu.lit = true;
    } else if volume {
        let last = POWER_CHOICES.len() - 1;
        menu.chosen = if key == Key::VolumeUp {
            menu.chosen.checked_sub(1).unwrap_or(last)
        } else {
            (menu.chosen + 1) % POWER_CHOICES.len()
        };
    }
    let chosen = menu.chosen;
    self.dirty = true;
    match key {
        Key::Power => self.act(Action::PowerMenuChoice(POWER_CHOICES[chosen].0), now),
        Key::Back => {
            self.hide_power_menu();
            Vec::new()
        }
        _ => {
            self.stir(now);
            Vec::new()
        }
    }
}

/// The power menu over the page: the same card the sheets use, but the page
/// under it is softened, not darkened — the one thing that tells this menu
/// from the confirmations it opens. The soft copy is made once and laid over
/// the page a little more each frame as the card rises from the foot.
pub(super) fn compose_power_menu(&mut self, out: &mut Canvas, f: &Fonts, now: u64) {
    let Some(menu) = self.power_menu.as_ref() else { return };
    let since = menu.since;
    // The entry is ringed only once the volume keys have asked for a light:
    // a menu just come up is a menu nothing is chosen on yet.
    let chosen = menu.lit.then_some(menu.chosen);
    let t = self.theme();
    let lang = self.state.lang;
    let m = &t.m;
    let along = dialog::shown_sheet(since, None, now);
    if along <= 0.0 {
        return;
    }
    // Blurred once into a copy kept while the menu stands, then laid back
    // over the page as the card comes up: softened ground, not a dark one.
    if self.power_blur.is_none() {
        self.power_blur = Some(super::lock::soft_ground(out, m.w, m.h));
    }
    if let Some(blur) = self.power_blur.clone() {
        out.lay(&blur, 0, 0, along);
    }
    let rows = power_rows();
    let geom = dialog::sheet_geom(&t, f, "", &rows);
    let off = dialog::sheet_off(&t, along);
    dialog::paint_sheet(out, &t, f, lang, POWER_TITLE, &rows, &geom, off, chosen);
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use skyward_info::Charge;

    /// The centre of the `index`-th key of a sheet's rows, once it has risen.
    fn sheet_key_at(app: &App, f: &Fonts, rows: &dialog::SheetRows, body: &str, index: usize) -> (f32, f32) {
        let t = app.theme();
        let geom = dialog::sheet_geom(&t, f, body, rows);
        let off = dialog::sheet_off(&t, 1.0);
        let (rect, _) = geom.rows.iter().flatten().zip(rows.iter().flatten()).nth(index).unwrap();
        (rect.x + rect.w / 2.0, rect.y + off + rect.h / 2.0)
    }

    /// The power menu is four whole-row keys: the three that change the
    /// phone's state and cancel. Its card is a sheet's, but its ground is
    /// softened, which is what tells it from the confirmations to come.
    #[test]
    fn the_power_menu_lists_four_ways_out() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.show_power_menu(0);
        let rows = power_rows();
        assert_eq!(rows.len(), 4, "four rows");
        assert!(rows.iter().all(|r| r.len() == 1), "each a whole row");
        assert_eq!(rows[3][0].1, Action::PowerMenuChoice(PowerChoice::Cancel), "cancel is last");
        // A finger on a key hands its choice to act, which opens the card.
        let (x, y) = sheet_key_at(&app, &f, &rows, "", 0);
        assert!(app.touch(Touch::Down(x, y), &f, dialog::RISE_MS).is_empty(), "power off asks first");
        assert!(app.power_menu.is_none(), "the menu went");
        let sheet = app.sheet.as_ref().unwrap();
        assert_eq!(sheet.title, tri("关机", "關機", "Power off"));
        assert_eq!(sheet.body, "您的手机将会关机。");
        let _ = i;
    }

    /// Picking cancel from the menu puts it away and asks nothing: the page
    /// under it carries on, and no sheet is left over.
    #[test]
    fn cancelling_the_menu_just_lets_the_page_go_on() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.show_power_menu(0);
        let rows = power_rows();
        let (x, y) = sheet_key_at(&app, &f, &rows, "", 3);
        assert!(app.touch(Touch::Down(x, y), &f, dialog::RISE_MS).is_empty());
        assert!(app.power_menu.is_none() && app.sheet.is_none(), "nothing stands over the page now");
        assert_eq!(app.screen(), Screen::Home, "and the page is where it was");
        let _ = i;
    }

    /// The card rises a frame at a time and the panel is told to draw each
    /// of them. Asked once for the end of the rise, it went from off the
    /// panel to where it was going between two frames, which reads as no
    /// animation at all; asked for every frame of a rise long done, it kept
    /// the loop turning at full speed with nothing to show.
    #[test]
    fn the_power_menu_moves_frame_by_frame_and_then_rests() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.show_power_menu(0);
        assert_eq!(app.deadline(0), Some(0), "the card is on its way up");
        assert_eq!(app.deadline(dialog::RISE_MS / 2), Some(0), "and still on its way");
        assert_ne!(app.deadline(dialog::RISE_MS + 1), Some(0), "come up, it asks for nothing more");
        let _ = app.take_dirty();
        app.tick(dialog::RISE_MS / 2);
        assert!(app.take_dirty(), "every frame of the rise is drawn");
    }

    /// Seen off, the card falls and the page goes black under it, a frame at
    /// a time: woken at the end of the fall alone, the card stood still and
    /// the black arrived whole, which is what the phone showed.
    #[test]
    fn the_black_out_is_drawn_frame_by_frame() {
        let (f, _) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.show_power_menu(0);
        let rows = power_rows();
        let (x, y) = sheet_key_at(&app, &f, &rows, "", 0);
        app.touch(Touch::Down(x, y), &f, dialog::RISE_MS);
        let body = app.sheet.as_ref().unwrap().body.clone();
        let rows = app.sheet.as_ref().unwrap().rows.clone();
        let (x, y) = sheet_key_at(&app, &f, &rows, &body, 0);
        app.touch(Touch::Down(x, y), &f, dialog::RISE_MS * 2);
        assert!(app.power_exit.is_some(), "确定 sends the phone off");
        let mid = dialog::RISE_MS * 2 + dialog::FALL_MS / 2;
        let _ = app.take_dirty();
        app.tick(mid);
        assert!(app.take_dirty(), "the fall is drawn as it goes");
        assert_eq!(app.deadline(mid), Some(0), "and asked for a frame at a time until it is gone");
    }

    /// A confirmation's 确定 does what it says and takes the sheet with it;
    /// its 取消 only takes the sheet, leaving the page as it was. 确定 asks
    /// for nothing at once: the black-out comes first.
    #[test]
    fn a_confirmation_goes_on_cancel_and_leaves_on_ok() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.show_power_menu(0);
        let rows = power_rows();
        let (x, y) = sheet_key_at(&app, &f, &rows, "", 0);
        app.touch(Touch::Down(x, y), &f, dialog::RISE_MS); // open the power-off confirmation

        let sheet = app.sheet.as_ref().unwrap();
        let rows = sheet.rows.clone();
        assert_eq!(rows[0].len(), 2, "确定 and 取消 share the first row");
        let (cx, cy) = sheet_key_at(&app, &f, &rows, &app.sheet.as_ref().unwrap().body, 1);
        assert!(app.touch(Touch::Down(cx, cy), &f, dialog::RISE_MS * 2).is_empty(), "cancel does nothing");
        assert!(app.sheet.as_ref().unwrap().closing.is_some(), "but the sheet goes");
        assert!(app.power_exit.is_none(), "and nothing goes black");
        app.tick(dialog::RISE_MS * 2 + dialog::FALL_MS);
        assert_eq!(app.screen(), Screen::Home, "back to the page that was under it");

        // Open it again and this time confirm.
        app.show_power_menu(dialog::RISE_MS * 3);
        let rows = power_rows();
        let (x, y) = sheet_key_at(&app, &f, &rows, "", 0);
        app.touch(Touch::Down(x, y), &f, dialog::RISE_MS * 4);
        let sheet = app.sheet.as_ref().unwrap();
        let rows = sheet.rows.clone();
        let body = sheet.body.clone();
        let (cx, cy) = sheet_key_at(&app, &f, &rows, &body, 0);
        let at = dialog::RISE_MS * 5;
        assert!(app.touch(Touch::Down(cx, cy), &f, at).is_empty(), "确定 asks for no word at once");
        let exit = app.power_exit.as_ref().expect("the black-out has begun");
        assert_eq!(exit.target, Target::PowerOff);
        assert_eq!(exit.since, at, "it began on the key");
        assert!(app.tick(at + BLACK_MS + BLACK_HOLD_MS - 1).is_empty(), "black first, then the beat");
        assert_eq!(app.tick(at + BLACK_MS + BLACK_HOLD_MS), vec![Effect::Reboot(Target::PowerOff)], "and the phone leaves");
        assert!(app.tick(at + BLACK_MS + BLACK_HOLD_MS + 1000).is_empty(), "asked for once, not again");
        let _ = i;
    }

    /// The restart card carries recovery and bootloader as whole rows below
    /// 确定 / 取消, and either leaves through the same black-out, the card
    /// falling as the page under it goes black.
    #[test]
    fn the_restart_card_carries_recovery_and_bootloader() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.act(Action::PowerMenuChoice(PowerChoice::Restart), 0);
        let sheet = app.sheet.as_ref().unwrap();
        assert_eq!(sheet.title, tri("重新启动", "重新啟動", "Restart"));
        assert_eq!(sheet.body, "您的手机将会重新启动。");
        let rows = sheet.rows.clone();
        assert_eq!(rows.len(), 3, "确定/取消, then two whole rows");
        assert_eq!(rows[1][0].1, Action::PowerExit(Target::Recovery));
        assert_eq!(rows[2][0].1, Action::PowerExit(Target::Bootloader));
        let body = sheet.body.clone();
        // Keys flatten as 确定, 取消, Recovery, Bootloader.
        let (x, y) = sheet_key_at(&app, &f, &rows, &body, 2);
        let at = dialog::RISE_MS;
        assert!(app.touch(Touch::Down(x, y), &f, at).is_empty());
        assert_eq!(app.power_exit.as_ref().map(|e| e.target), Some(Target::Recovery));
        assert_eq!(app.tick(at + BLACK_MS + BLACK_HOLD_MS), vec![Effect::Reboot(Target::Recovery)]);
        let _ = i;
    }

    /// The black-out is a ground under the card, not over it: the page
    /// darkens all the way to black while the card keeps the colour it had.
    #[test]
    fn the_page_goes_black_while_the_card_keeps_its_colour() {
        let (f, i) = (fonts(), info());
        let t = crate::Theme::light(W, H, PPD);
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.act(Action::PowerMenuChoice(PowerChoice::PowerOff), 0);
        let sheet = app.sheet.as_ref().unwrap();
        let (rows, body) = (sheet.rows.clone(), sheet.body.clone());
        let (x, y) = sheet_key_at(&app, &f, &rows, &body, 0);
        let at = dialog::RISE_MS;
        app.touch(Touch::Down(x, y), &f, at);

        // The card's place on its way down — early enough that it is still on
        // the panel, which the fall clears in its first few frames — and the
        // page's own corner, clear of it.
        let geom = dialog::sheet_geom(&t, &f, &body, &rows);
        let early = at + 40;
        let off = dialog::sheet_off(&t, dialog::shown_sheet(at, Some(at), early));
        let (card_x, card_y) = (geom.card.x + geom.card.w / 2.0, geom.card.y + off + t.m.dp(10.0));
        let ink = |app: &mut App, now| {
            let mut c = Canvas::new(W, H);
            app.compose(&mut c, &f, &i, "12:00", "11月18日 周二", now);
            (grey_at(&c, 8.0, 8.0), grey_at(&c, card_x, card_y))
        };
        let (page_before, card_before) = ink(&mut app, at);
        let (page_early, card_early) = ink(&mut app, early);
        assert!(page_early < page_before, "the page is going black: {page_early} of {page_before}");
        assert_eq!(card_early, card_before, "the card keeps its colour on the way down");
        let (page_black, _) = ink(&mut app, at + BLACK_MS + 1);
        assert_eq!(page_black, 0, "and the page is black once it has gone");
    }

    /// Only the power menu's own confirmations are seen off this way: a
    /// page's plain 重启到系统 still leaves at once, as it always has.
    #[test]
    fn a_plain_reboot_leaves_at_once_without_the_black_out() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        assert_eq!(
            app.act(Action::RebootTo(Target::System), 0),
            vec![Effect::Reboot(Target::System)],
            "no black-out, no wait"
        );
        assert!(app.power_exit.is_none());
        let _ = (f, i);
    }

    /// Suspend is not wired up, so both of its keys close the card and leave
    /// the phone as it was — the 确定 does no more than the 取消.
    #[test]
    fn suspend_confirms_to_nothing_because_it_is_not_built() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.act(Action::PowerMenuChoice(PowerChoice::Suspend), 0);
        let sheet = app.sheet.as_ref().unwrap();
        assert_eq!(sheet.title, tri("确定要挂起吗？", "確定要掛起嗎？", "Suspend?"));
        assert!(sheet.rows[0].iter().all(|(_, a)| *a == Action::SheetCancel), "both keys only close it");
        let rows = sheet.rows.clone();
        let body = sheet.body.clone();
        let (x, y) = sheet_key_at(&app, &f, &rows, &body, 0);
        assert!(app.touch(Touch::Down(x, y), &f, dialog::RISE_MS).is_empty());
        assert!(app.sheet.as_ref().unwrap().closing.is_some());
        assert_eq!(app.screen(), Screen::Home);
        let _ = i;
    }

    #[test]
    fn power_key_turns_the_panel_off_and_on() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        assert!(app.key(Key::Power, 0).is_empty());
        assert_eq!(app.panel(0), Some(SLIDER_START), "it waits for volume down first");
        assert_eq!(app.deadline(0), Some(CHORD_MS));
        // Past the wait for its partner nothing has come of it yet: let go,
        // the panel goes off; held the long press through, the power menu
        // comes up instead.
        assert!(app.tick(CHORD_MS).is_empty());
        assert!(app.display_on());
        assert_eq!(app.key_up(Key::Power, CHORD_MS + 50), vec![Effect::Ticks], "let go, it turns the panel off, ticking the motor");
        assert!(!app.display_on());
        assert!(app.key(Key::Power, CHORD_MS + 60).is_empty());
        assert!(app.display_on(), "and lights it again");
        assert!(app.key_up(Key::Power, CHORD_MS + 100).is_empty(), "a press that only lit the panel does not put it out");
        assert!(app.key(Key::Power, 1000).is_empty());
        assert_eq!(app.key_up(Key::Power, 1050), vec![Effect::Ticks]);
        assert!(!app.display_on(), "let go sooner, it turns the panel off then");
    }

    #[test]
    fn the_power_key_turning_the_screen_off_ticks_the_motor_whatever_the_settings() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        assert_eq!(press(&mut app, Key::Power, 1000), vec![Effect::Ticks]);
        assert!(app.tick(1400).is_empty(), "once");
        app.state.vibrate = false;
        press(&mut app, Key::Power, 2000);
        assert_eq!(press(&mut app, Key::Power, 3000), vec![Effect::Ticks], "with vibration off too");
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        assert!(app.tick(OFF).is_empty(), "but not the screen going off by itself");
        assert!(!app.display_on());
    }

    #[test]
    fn the_power_key_and_volume_down_together_take_a_screenshot() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.frame(&f, &i);
        app.state.vibrate = true;
        assert!(app.key(Key::Power, 0).is_empty());
        let shot = vec![Effect::Haptic, Effect::Screenshot { storage: false }];
        assert_eq!(app.key(Key::VolumeDown, CHORD_MS - 1), shot, "volume down follows in time");
        assert_eq!(app.chord, None, "nothing is left waiting");
        assert!(app.tick(1000).is_empty());
        assert!(app.key_up(Key::Power, 1000).is_empty());
        assert!(app.key_up(Key::VolumeDown, 1010).is_empty());
        assert!(app.display_on(), "the power key did not turn the panel off");
        assert!(!app.keys, "nor did volume down light a control");
        // Volume down first works the same, the other key coming up first.
        assert!(app.key(Key::VolumeDown, 2000).is_empty());
        assert_eq!(app.key(Key::Power, 2100), shot);
        assert!(app.key_up(Key::VolumeDown, 2150).is_empty());
        assert!(app.key_up(Key::Power, 2200).is_empty());
        assert!(app.display_on());
        assert!(!app.keys);
        // Without a tap of the motor where the settings turn it off, and into
        // the gallery once internal storage is open.
        app.state.vibrate = false;
        app.data_waiting(None);
        app.key(Key::Power, 3000);
        assert_eq!(app.key(Key::VolumeDown, 3010), vec![Effect::Screenshot { storage: true }]);
    }

    #[test]
    fn the_power_key_and_volume_down_too_far_apart_each_do_their_own() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.volume_keys = true;
        app.frame(&f, &i);
        assert!(app.key(Key::VolumeDown, 0).is_empty());
        assert!(!app.keys, "volume down waits for the power key first");
        assert!(app.tick(CHORD_MS).is_empty());
        assert!(app.keys, "then lights a control on its own");
        assert_eq!(lit(&mut app, &f, &i), Some(Stop::Hit(Action::Open(Screen::Search))));
        // The volume key is let go before power comes down: there is no pair
        // of keys held together any more, so power works the lit control.
        assert!(app.key_up(Key::VolumeDown, CHORD_MS + 20).is_empty());
        assert!(app.key(Key::Power, CHORD_MS + 30).is_empty(), "the power key came too late");
        assert!(app.tick(CHORD_MS * 2 + 10).is_empty());
        assert_eq!(app.screen(), Screen::Home, "the power key waits on the control lit until it comes up");
        app.key_up(Key::Power, CHORD_MS * 3);
        assert_eq!(app.screen(), Screen::Search, "and then works it");
    }

    /// With the keys steering the page, a volume key is often still held
    /// while the other hand reaches for power — long past the little window
    /// for two fresh presses. It is still the same two-key chord: volume
    /// down and power take a picture, volume up and power bring the menu.
    #[test]
    fn a_held_cursor_key_and_power_take_a_chord() {
        let (f, i) = (fonts(), info());
        // Volume down held, then power: a screenshot.
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.vibrate = false;
        app.state.volume_keys = true;
        app.frame(&f, &i);
        assert!(app.key(Key::VolumeDown, 0).is_empty());
        assert!(app.tick(CHORD_MS).is_empty(), "the held key lands on the light");
        assert!(app.keys);
        assert_eq!(
            app.key(Key::Power, 1000),
            vec![Effect::Screenshot { storage: false }],
            "power with the volume key still down takes a picture"
        );
        assert!(app.key_up(Key::Power, 1100).is_empty());
        assert!(app.key_up(Key::VolumeDown, 1110).is_empty());
        assert_eq!(app.screen(), Screen::Home, "neither key works a control");

        // Volume up held, then power: the menu.
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.vibrate = false;
        app.state.volume_keys = true;
        app.frame(&f, &i);
        assert!(app.key(Key::VolumeUp, 0).is_empty());
        assert!(app.tick(CHORD_MS).is_empty());
        assert_eq!(app.key(Key::Power, 1000), Vec::new());
        assert!(app.power_menu.is_some(), "power with volume up held brings the menu");
        assert!(app.key_up(Key::Power, 1100).is_empty());
        assert!(app.key_up(Key::VolumeUp, 1110).is_empty());
        assert!(app.power_menu.is_some(), "the keys let go do not close it");

        // Power held on a lit control, then volume down: the same picture,
        // and power coming up must not work the control it went down on.
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.vibrate = false;
        app.state.volume_keys = true;
        app.frame(&f, &i);
        press(&mut app, Key::VolumeDown, 0);
        assert!(app.keys && app.lit().is_some());
        assert!(app.key(Key::Power, 1000).is_empty());
        assert!(app.tick(1000 + CHORD_MS).is_empty(), "power lands on the lit control");
        assert_eq!(
            app.key(Key::VolumeDown, 1200),
            vec![Effect::Screenshot { storage: false }]
        );
        assert!(app.key_up(Key::Power, 1300).is_empty());
        assert!(app.key_up(Key::VolumeDown, 1310).is_empty());
        assert_eq!(app.screen(), Screen::Home, "power did not take the lit control");
    }

    #[test]
    fn a_screenshot_says_where_it_went() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.screenshot_saved(Ok(Shot::Album), 0);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("截图已保存到相册"));
        app.screenshot_saved(Ok(Shot::Held), 0);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("截图已暂存，解密后存入相册"));
        app.screenshot_saved(Err("No space left on device".into()), 0);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("截图保存失败：No space left on device"));
        app.screenshots_not_kept("Read-only file system", 0);
        assert_eq!(app.toast.as_ref().map(|t| t.0.as_str()), Some("暂存的截图未能存入相册：Read-only file system"));
    }

    /// With the default two minutes, when the panel dims and when it goes
    /// off.
    const DIMS: u64 = 113_000;
    const OFF: u64 = 120_000;

    #[test]
    fn left_alone_the_panel_dims_then_goes_off() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        assert_eq!(app.deadline(0), Some(DIMS));
        app.tick(DIMS - 1);
        assert_eq!(app.panel(DIMS - 1), Some(SLIDER_START));
        app.tick(DIMS);
        assert_eq!(app.deadline(DIMS), Some(DIMS + BACKLIGHT_STEP_MS), "the backlight steps down a frame at a time");
        let midway = app.panel(DIMS + 250).unwrap();
        assert!(midway < SLIDER_START && midway > dim_level(SLIDER_START), "{midway}");
        assert_eq!(app.panel(DIMS + 500), Some(dim_level(SLIDER_START)));
        assert_eq!(app.deadline(DIMS + 500), Some(OFF), "dim until it goes off");
        app.tick(OFF);
        assert!(!app.display_on());
        assert_eq!(app.panel(OFF), Some(dim_level(SLIDER_START)), "the content goes dark under the dim backlight");
        app.tick(OFF + 365);
        assert_eq!(app.panel(OFF + 365), None);
        assert_eq!(app.deadline(OFF + 365), None, "nothing to wait for on a dark panel");
    }

    #[test]
    fn time_before_the_first_tick_does_not_count_as_idle() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        // The UI started long after boot; the idle clock begins at the
        // first tick, not at zero.
        app.tick(5_000_000);
        assert_eq!(app.panel(5_000_000), Some(SLIDER_START));
        assert_eq!(app.deadline(5_000_000), Some(5_000_000 + DIMS));
        app.tick(5_000_000 + OFF);
        assert!(!app.display_on());
    }

    #[test]
    fn the_dim_level_sits_below_the_brightness_set() {
        assert_eq!(dim_level(1.0), DIM_CEILING);
        assert_eq!(dim_level(0.06), MIN_BRIGHTNESS);
        assert_eq!(dim_level(0.03), MIN_BRIGHTNESS);
        assert_eq!(dim_level(0.01), 0.01, "never brighter than the level set");
        assert_eq!(dim_span(15_000), 3000);
        assert_eq!(dim_span(120_000), DIM_MS);
    }

    #[test]
    fn a_touch_while_dim_brightens_and_still_counts() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        app.tick(DIMS);
        assert_eq!(app.panel(DIMS + 500), Some(dim_level(SLIDER_START)));
        let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
        app.touch(Touch::Down(x, y), &f, DIMS + 600);
        app.touch(Touch::Up, &f, DIMS + 650);
        assert_eq!(app.screen(), Screen::Settings, "the tap counts");
        assert_eq!(app.panel(DIMS + 800), Some(SLIDER_START), "back to full brightness");
        assert_eq!(app.deadline(DIMS + 800), Some(DIMS + 650 + DIMS), "the wait starts over");
    }

    #[test]
    fn a_touch_on_the_dark_panel_lights_nothing() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
        press(&mut app, Key::Power, 0);
        assert!(app.touch(Touch::Down(x, y), &f, 100).is_empty());
        assert!(app.touch(Touch::Move(x, y - 300.0), &f, 120).is_empty());
        assert!(app.touch(Touch::Up, &f, 140).is_empty());
        assert!(!app.display_on(), "the panel stays dark");
        assert_eq!(app.screen(), Screen::Home, "the tap pressed nothing");
        assert_eq!(app.top().1, 0.0, "nor scrolled the page");
        press(&mut app, Key::Power, 200);
        assert!(app.display_on());
        // The panel lit comes back behind the lock, which is swiped away
        // before the page takes a finger.
        unlock(&mut app, &f, 300);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Settings), 1400);
        assert_eq!(app.screen(), Screen::Settings, "a tap on the lit panel does");
    }

    #[test]
    fn a_cable_coming_out_lights_the_dark_panel_and_keeps_a_lit_one_lit() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        app.tick(OFF);
        app.tick(OFF + 365);
        assert_eq!(app.panel(OFF + 365), None);
        let pulled = OFF + 1000;
        app.plugged(pulled, false);
        assert!(app.display_on(), "the cable coming out lights the panel");
        assert!(!app.charge_shown(), "and shows the page");
        assert_eq!(app.panel(pulled), Some(SLIDER_START));
        assert_eq!(app.deadline(pulled + 520), Some(pulled + DIMS), "for the screen-off time, once the content is up");
        app.tick(pulled + DIMS);
        assert_eq!(app.panel(pulled + DIMS + 500), Some(dim_level(SLIDER_START)));
        let again = pulled + DIMS + 600;
        app.plugged(again, false);
        assert_eq!(app.panel(again + 150), Some(SLIDER_START), "a cable coming out brightens a lit panel");
        assert_eq!(app.deadline(again + 150), Some(again + DIMS), "and the wait starts over");
        press(&mut app, Key::Power, again + 200);
        assert!(!app.display_on(), "the power key turns it off at once, as ever");
    }

    #[test]
    fn a_cable_going_in_on_the_dark_panel_shows_the_charging_screen_and_goes_dark_again() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        press(&mut app, Key::Power, 0);
        app.tick(365);
        assert_eq!(app.panel(365), None);
        let plugged = 1000;
        app.plugged(plugged, true);
        assert!(app.display_on() && app.charge_shown(), "the charging screen lights the panel");
        assert_eq!(app.deadline(plugged + 1000), Some(0), "its tide moves a frame at a time");
        app.tick(plugged + CHARGE_MS - 1);
        assert!(app.display_on());
        app.tick(plugged + CHARGE_MS);
        assert!(!app.display_on(), "then the panel goes dark again");
        assert!(app.panel(plugged + CHARGE_MS + 100).is_some(), "as the charging screen goes dark");
        app.tick(plugged + CHARGE_MS + 365);
        assert_eq!(app.panel(plugged + CHARGE_MS + 365), None);
        assert!(app.charging.is_none(), "and goes with it");
    }

    #[test]
    fn a_cable_going_in_on_a_lit_panel_brings_the_charging_screen_over_the_page_and_away() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        app.plugged(1000, true);
        assert!(app.display_on() && app.charge_shown());
        app.tick(1000 + CHARGE_MS);
        assert!(app.display_on(), "the panel stays lit");
        assert!(!app.charge_shown(), "as the charging screen goes for the page");
        app.tick(1000 + CHARGE_MS + 300);
        assert!(app.charging.is_none());
    }

    #[test]
    fn a_touch_or_a_volume_key_sends_the_charging_screen_away_and_keeps_the_panel_lit() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let (x, y) = spot(&mut app, &f, &i, Action::Open(Screen::Settings));
        app.tick(0);
        press(&mut app, Key::Power, 0);
        app.tick(365);
        app.plugged(1000, true);
        app.touch(Touch::Down(x, y), &f, 1500);
        app.touch(Touch::Up, &f, 1550);
        assert!(!app.charge_shown(), "the touch sent it away");
        assert_eq!(app.screen(), Screen::Home, "and pressed nothing");
        app.tick(1000 + CHARGE_MS);
        assert!(app.display_on(), "the panel stays lit for the page");
        app.plugged(10_000, true);
        press(&mut app, Key::VolumeDown, 10_500);
        assert!(!app.charge_shown(), "a volume key sends it away too");
        assert!(app.display_on());
    }

    #[test]
    fn the_charging_screen_shows_the_level_over_a_tide_of_light() {
        let (f, i) = (fonts(), DeviceInfo { battery: Some(battery_at(64, Charge::Charging)), ..info() });
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        press(&mut app, Key::Power, 0);
        app.tick(365);
        app.plugged(1000, true);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);
        let px = |x: u32, y: u32| {
            let k = ((y * W + x) * 4) as usize;
            [c.data[k], c.data[k + 1], c.data[k + 2]]
        };
        assert_eq!(px(0, 0), [0, 0, 0], "on black");
        let foot = px(W / 2, H - 4);
        assert!(foot[2] > 40 && foot[2] > foot[0] * 4 && foot[2] > foot[1] * 2, "the tide's light at the foot is blue: {foot:?}");
    }

    #[test]
    fn the_content_goes_dark_under_the_backlight_it_had_before_the_panel_goes_off() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        press(&mut app, Key::Power, 1000);
        assert!(!app.display_on(), "touches and keys take the screen as off at once");
        assert_eq!(app.panel(1000), Some(SLIDER_START), "while its content goes dark under the backlight it had");
        assert_eq!(app.deadline(1000), Some(0), "a frame at a time");
        assert_eq!(app.panel(1300), Some(SLIDER_START));
        app.tick(1365);
        assert_eq!(app.panel(1365), None, "then the panel goes off");
        assert_eq!(app.deadline(1365), None);
    }

    #[test]
    fn the_power_key_as_the_content_goes_dark_brings_it_back() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        press(&mut app, Key::Power, 1000);
        app.tick(1100);
        press(&mut app, Key::Power, 1100);
        assert!(app.display_on(), "the power key lit it again");
        let level = app.curtain.level(1100);
        assert!(level > 0.0 && level < 1.0, "from as dark as it had gone: {level}");
        assert_eq!(app.curtain.level(1100 + 520), 1.0);
    }

    #[test]
    fn the_content_comes_up_from_black_as_the_panel_lights() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.tick(0);
        press(&mut app, Key::Power, 0);
        app.tick(365);
        press(&mut app, Key::Power, 1000);
        let mut c = Canvas::new(W, H);
        let light = |c: &Canvas| c.data.as_chunks::<4>().0.iter().map(|p| p[0] as u64 + p[1] as u64 + p[2] as u64).sum::<u64>();
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        let first = light(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1260);
        let halfway = light(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1520);
        let full = light(&c);
        assert_eq!(first, 0, "black as the panel lights");
        assert!(halfway > full / 2 && halfway < full, "{halfway} of {full} halfway through");
    }

    #[test]
    fn only_the_power_key_lights_the_dark_panel_and_does_nothing_else() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Settings, &f, &i);
        press(&mut app, Key::Power, 0);
        for (key, at) in [(Key::Back, 10), (Key::VolumeUp, 20), (Key::VolumeDown, 30)] {
            assert!(press(&mut app, key, at).is_empty());
            assert!(!app.display_on(), "{key:?} leaves the panel dark");
        }
        assert!(press(&mut app, Key::Power, 40).is_empty());
        assert!(app.display_on());
        assert_eq!(app.screen(), Screen::Settings, "back did not go back");
    }

    #[test]
    fn a_stray_press_just_after_the_panel_lights_only_keeps_it_awake() {
        let (f, i) = (fonts(), info());
        let mut app = opened(Screen::Reboot, &f, &i);
        press(&mut app, Key::Power, 0);
        // A key wakes the panel; the next, within the guard, does not sleep
        // it again or act.
        app.key(Key::Power, 100);
        assert!(app.display_on());
        assert!(app.key(Key::Power, 300).is_empty());
        assert!(app.display_on(), "a second power press did not turn it off");
        assert!(app.key(Key::Back, 350).is_empty());
        assert_eq!(app.screen(), Screen::Reboot, "back did not go back");
        // Past the guard, power turns it off as usual.
        press(&mut app, Key::Power, 600);
        assert!(!app.display_on());
    }

    #[test]
    fn a_stray_touch_just_after_an_install_lights_the_panel_presses_nothing() {
        let (f, i) = (fonts(), info());
        let mut app = sideloading(&f, &i);
        press(&mut app, Key::Power, 0);
        app.install(Event::Finished(Outcome::Installed { slot: Slot::A, wipe: false }), 100);
        assert!(app.display_on(), "the finished install lit the panel");
        // A finger reaching for the phone lands on 重启到系统; within the
        // guard it only keeps the panel awake.
        assert!(tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 200).is_empty());
        assert_eq!(app.screen(), Screen::Installing, "the stray tap did not reboot");
        // Past the guard the lock is swiped away, and the same tap reboots.
        unlock(&mut app, &f, 400);
        assert_eq!(
            tap_at(&mut app, &f, &i, Action::RebootTo(Target::System), 1500),
            vec![Effect::Reboot(Target::System)]
        );
    }

    #[test]
    fn a_finger_held_as_the_panel_goes_off_lets_go() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Reboot, W, H, PPD);
        let track = app.frame(&f, &i).hits.iter().find(|h| h.action == Action::Confirm(Confirm::Reboot)).unwrap().rect;
        let y = track.centre().1;
        app.touch(Touch::Down(track.x + track.h / 2.0, y), &f, 0);
        app.touch(Touch::Move(track.x + track.w * 0.6, y), &f, 10);
        assert!(app.state.confirm > 0.0);
        press(&mut app, Key::Power, 20);
        assert_eq!(app.state.confirm, 0.0, "the knob is back at the start");
        app.touch(Touch::Move(track.x + track.w, y), &f, 30);
        assert!(app.touch(Touch::Up, &f, 40).is_empty(), "no reboot from a dark panel");
        assert!(!app.display_on());
    }

    #[test]
    fn never_leaves_the_panel_on() {
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.screen_off = SCREEN_OFF.iter().position(Option::is_none).unwrap();
        app.tick(0);
        assert_eq!(app.deadline(0), None);
        let day = 24 * 3600 * 1000;
        app.tick(day);
        assert_eq!(app.panel(day), Some(SLIDER_START));
    }

    #[test]
    fn the_installer_lights_the_panel_when_it_needs_the_user() {
        let (f, i) = (fonts(), info());
        let mut app = sideloading(&f, &i);
        press(&mut app, Key::Power, 0);
        app.install(Event::Checking { size: 1 << 20 }, 10);
        assert!(!app.display_on(), "reading the package does not light it");
        app.install(Event::Image { kind: ImageKind::Dtbo, size: 1 << 20, partitions: slotted(&["dtbo"]) }, 20);
        assert!(app.display_on(), "an image waiting for its partition does");
        app.tick(20 + 520);
        assert_eq!(app.deadline(20 + 520), Some(20 + DIMS));
        app.state.install = Install::Flashing { partition: "dtbo".into(), stage: None };
        press(&mut app, Key::Power, 500);
        app.install(Event::Progress(Stage::Flashing { done: 1, total: 2 }), 510);
        assert!(!app.display_on(), "progress does not");
        app.install(Event::Finished(Outcome::Flashed { partition: "dtbo".into(), slots: vec![Slot::A] }), 520);
        assert!(app.display_on(), "the end does");
    }

    #[test]
    fn an_install_ending_after_its_page_was_left_keeps_the_panel_dark() {
        let (f, i) = (fonts(), info());
        let mut app = sideloading(&f, &i);
        assert_eq!(app.key(Key::Back, 0), vec![Effect::Cancel]);
        press(&mut app, Key::Power, 10);
        app.install(Event::Finished(Outcome::Cancelled), 20);
        assert!(!app.display_on());
        assert_eq!(app.state.install, Install::Idle);
    }

    /// The power key held on a dark panel lights it and goes on to the power
    /// menu, the hold timed from the press that lit it.
    /// Holding power always brings the menu up, even when the keys have
    /// left a control lit: a short press takes that control, a long one
    /// reaches past it to the menu.
    #[test]
    fn a_long_press_brings_the_menu_up_with_a_control_lit() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.state.volume_keys = true;
        app.frame(&f, &i);
        press(&mut app, Key::VolumeDown, 0);
        assert!(app.keys && app.lit().is_some(), "the light is on a control");
        assert!(app.key(Key::Power, 1000).is_empty());
        assert_eq!(app.deadline(1000), Some(1000 + CHORD_MS), "it waits for volume down first");
        assert!(app.tick(1000 + CHORD_MS).is_empty(), "the power key lands on the control");
        assert_eq!(app.deadline(1000 + CHORD_MS), Some(1000 + LONG_PRESS_MS), "the hold is timed");
        assert!(app.tick(1000 + LONG_PRESS_MS - 1).is_empty());
        assert_eq!(app.tick(1000 + LONG_PRESS_MS), vec![Effect::Haptic]);
        assert!(app.power_menu.is_some(), "held long enough, the menu comes up");
        assert!(app.key_up(Key::Power, 1500).is_empty(), "and the key does not take the control too");
        assert_eq!(app.screen(), Screen::Home);
    }

    #[test]
    fn the_power_key_held_on_a_dark_panel_brings_the_menu_up() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.frame(&f, &i);
        app.sleep(1000);
        assert!(!app.display_on());
        assert!(app.key(Key::Power, 2000).is_empty());
        assert!(app.display_on(), "the press lights the panel at once");
        assert!(app.power_menu.is_none(), "and the menu waits for the hold");
        app.tick(2000 + LONG_PRESS_MS);
        assert!(app.power_menu.is_some(), "held on, the menu comes up");
        assert!(app.key_up(Key::Power, 2600).is_empty());
        assert!(app.display_on(), "let go after the menu, the panel stays lit");
    }

    /// With the lock up, the power key and volume up still come together for
    /// the menu, whichever of the two goes down first; alone, the volume key
    /// does nothing there.
    #[test]
    fn the_menu_comes_up_from_the_lock_on_the_power_key_and_volume_up() {
        let (f, i) = (fonts(), info());
        for first in [Key::Power, Key::VolumeUp] {
            let mut app = App::new(Screen::Desktop, W, H, PPD);
            app.state.vibrate = false;
            app.frame(&f, &i);
            app.lock_now();
            assert!(app.locked());
            assert!(app.key(first, 1000).is_empty());
            let second = if first == Key::Power { Key::VolumeUp } else { Key::Power };
            assert!(app.key(second, 1100).is_empty());
            assert!(app.power_menu.is_some(), "the two together bring the menu up over the lock, {first:?} first");
            assert_eq!(app.chord, None, "and neither waits for the other any more");
        }
    }

    /// A volume key on the lock on its own moves nothing, even once the wait
    /// for its partner is over.
    #[test]
    fn a_volume_key_alone_on_the_lock_moves_nothing() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        app.state.volume_keys = false;
        let was = app.state.media_volume;
        app.frame(&f, &i);
        app.lock_now();
        assert!(app.key(Key::VolumeUp, 1000).is_empty());
        app.tick(1000 + CHORD_MS);
        assert_eq!(app.state.media_volume, was, "the volume is left as it was");
        assert!(app.volume_bar.is_none(), "and no panel comes up");
        assert!(app.key_up(Key::VolumeUp, 1200).is_empty());
        assert_eq!(app.state.media_volume, was);
    }
}
