//! Going from page to page: one opened over another, gone back from, and
//! what each asks of the runtime as it opens.

use super::*;

/// What going back is told while work that can be stopped goes on.
const WAIT_OR_STOP: Text = texts::toast::back::WAIT_OR_STOP;

impl App {
    /// Open `screen` over the page shown, sliding in over it — or, from the
    /// desktop, growing out of the icon that was touched.
    pub(super) fn open(&mut self, screen: Screen) {
        // A touch on the frame happens before this; the icon it struck is
        // the one the page grows from and goes back into. Remember it now,
        // while the frame still holds the desktop that was shown. Pages
        // opened within an app leave it alone, so a page going back to the
        // desktop still finds the icon it first came out of.
        if self.screen() == Screen::Desktop {
            self.desk_icon = self.frame.as_ref().and_then(|fr| fr.launches.iter().find(|(s, _)| *s == screen).map(|(_, r)| *r));
        }
        self.leave(false);
        self.push(screen);
    }

    pub(super) fn push(&mut self, screen: Screen) {
        self.stack.push((screen, 0.0, Focus::Unset));
        self.settle_page();
        self.refresh();
    }

    /// The page shown gives way to another, opened over it or, `back`, gone
    /// back to under it: draw it as it last showed, to slide away. Changes
    /// made before the slide's first frame keep the page that showed; a
    /// page rendered anew since it last showed gives way at once.
    pub(super) fn leave(&mut self, back: bool) {
        let rise = self.rising(back);
        // A page that came out of a desktop icon grows out of it and goes
        // back into it; every other page slides. The icon is read while the
        // stack still says where the page goes back to.
        let icon = if rise { self.desk_icon } else { None };
        if let Some(turn) = self.turn.as_mut().filter(|t| t.spring.is_none()) {
            // The move already picked its way across the screen or up it:
            // turning back mid-move keeps to that way rather than jumping
            // from one axis to the other.
            turn.back = back;
            return;
        }
        self.end_turn();
        let Some(view) = self.shown.take() else { return };
        let mut left = self.backdrop.take().unwrap_or_else(|| Canvas::new(self.w, self.h));
        // The `<` comes up with the finger, before its page goes; an icon
        // held lets go too, since the desktop is drawn here once and kept as
        // the picture the page moves over, and anything still darkening
        // would be kept dark for good.
        self.icon_press = None;
        let view = View { pressing: None, ..view };
        let t = self.theme();
        self.paint_page(&mut left, &t, None, &view);
        // A page left within the first frames of sliding in, before it was
        // drawn across, shows its colour where it was not drawn yet.
        if let Some(fr) = self.frame.as_ref().filter(|_| self.cols < self.w) {
            left.fill_rect(self.cols as f32, 0.0, (self.w - self.cols) as f32, self.h as f32, fr.bg);
        }
        self.turn = Some(Turn { back, rise, icon, lifted: 0.0, left, spring: None });
        self.dirty = true;
    }

    /// Whether the move now under way goes up and down rather than across:
    /// the desktop opens the pages over it, and they sink back down to it,
    /// while pages opened one from another still go across. Either way the
    /// desktop pages grow out of their icons rather than slide.
    fn rising(&self, back: bool) -> bool {
        if back {
            // The page going is the one shown; the page it goes back to is
            // the one under it, or the desktop when there is none.
            self.stack.len() < 2 || self.stack[self.stack.len() - 2].0 == Screen::Desktop
        } else {
            self.screen() == Screen::Desktop
        }
    }

    /// Whether work that must not be cut short is under way: a partition
    /// being written, a package installing or flashing, a backup being
    /// restored, the filesystem or its labels made over, or Skyward going
    /// back into the recovery partitions. Going back waits for such work to
    /// end rather than leave it, so the pull from the foot of the screen,
    /// which leaves any page for the desktop at once without asking what it
    /// was doing, waits for it too.
    ///
    /// A package counts until it has ended, whatever its stage: left as
    /// going back leaves it, the installer is told to stop, which a pull has
    /// no way to do once the page is gone.
    pub(super) fn working(&self) -> bool {
        let s = &self.state;
        let installing = !matches!(&s.install, Install::Idle | Install::Ending | Install::Finished { .. } | Install::Wiped(_));
        s.systems_work.is_some()
            || s.job.as_ref().is_some_and(|j| j.outcome.is_none())
            || s.relabel.as_ref().is_some_and(|j| j.outcome.is_none())
            || s.part_work.as_ref().is_some_and(|w| w.outcome.is_none())
            || s.backup_job.as_ref().is_some_and(|j| j.outcome.is_none())
            || s.skyward_job.as_ref().is_some_and(|j| j.outcome.is_none())
            || installing
    }

    /// Fold the page shown back into its icon from where a pull at the foot
    /// of the screen carried it, `up` pixels past the top: the same fold a
    /// page goes back with, begun from the lift's last frame instead of
    /// from the whole screen, so the pull and the fold read as one motion.
    ///
    /// The icon is the app's own, wherever on the desktop it keeps it, not
    /// the one the page was opened from: a pull takes the page it holds off
    /// for the desktop from wherever it is, so a page opened from inside
    /// another app folds into its own square rather than its opener's.
    pub(super) fn leave_by_foot(&mut self, up: f32) {
        if self.working() {
            // A page at work stays where the pull left it: the pull cuts
            // nothing short that going back waits for.
            return;
        }
        self.foot = None;
        let Some(view) = self.shown.take() else {
            // Nothing was drawn to fold: the page goes at once.
            self.leave_for_desktop();
            return;
        };
        let mut left = self.backdrop.take().unwrap_or_else(|| Canvas::new(self.w, self.h));
        self.icon_press = None;
        let view = View { pressing: None, ..view };
        let t = self.theme();
        self.paint_page(&mut left, &t, None, &view);
        if let Some(fr) = self.frame.as_ref().filter(|_| self.cols < self.w) {
            left.fill_rect(self.cols as f32, 0.0, (self.w - self.cols) as f32, self.h as f32, fr.bg);
        }
        let icon = pages::tile_of(&t, self.screen()).or(self.desk_icon);
        self.turn = Some(Turn { back: true, rise: true, icon, lifted: up, left, spring: None });
        // The pull leaves for the desktop whatever lay between: no page in
        // between, the desktop at the top.
        self.leave_for_desktop();
        self.dirty = true;
    }

    /// Leave whatever page is shown for the desktop, as a pull from the
    /// foot of the screen does: no page in between, the desktop at the top.
    /// A page at work stays: the pull cuts nothing short that going back
    /// waits for.
    pub(super) fn leave_for_desktop(&mut self) {
        if self.working() {
            return;
        }
        self.stack = vec![(Screen::Desktop, 0.0, Focus::Unset)];
        self.state.confirm = 0.0;
        self.settle_page();
        self.refresh();
    }

    /// Set the home page's search bar moving up to the search page's field,
    /// or back down, before the page changes: from where a move not yet
    /// done has it, at the speed it has.
    pub(super) fn start_morph(&mut self, into: bool, now: u64) {
        let (q, speed, title, cancel) = match self.morph.take() {
            Some(m) => {
                let (q, _) = m.at(now);
                let speed = (q - m.at(now.saturating_sub(8)).0) / 0.008;
                (q, speed, m.title, m.cancel)
            }
            None => (if into { 0.0 } else { 1.0 }, 0.0, None, None),
        };
        let target = if into { 1.0 } else { 0.0 };
        // The home page is the page shown, or the one under the search.
        let home = self.stack.len() - if into { 1 } else { 2 };
        // On the way down the keyboard goes with the search page's frame.
        let keyboard = if into { None } else { self.frame.as_ref().map(|fr| keyboard_strip(&self.theme(), fr)) };
        self.morph = Some(Morph {
            into,
            spring: Spring::new(now, (q - target) * 1000.0, speed * 1000.0, MORPH_PERIOD),
            start: now,
            scroll: self.stack[home].1,
            keyboard,
            body: None,
            title,
            cancel,
        });
        self.dirty = true;
    }

    /// Go back a page. Leaving the installing page stops what can be
    /// stopped; while a package is being read or written, the page stays.
    /// In the file manager, back climbs a folder until it reaches the root,
    /// and in a place, until it reaches the place's top.
    pub(super) fn back(&mut self, now: u64) -> Vec<Effect> {
        self.back_by(now, true)
    }

    /// Go back a page, drawn as the page leaving, or, `drawn` false, at
    /// once: a finger that has pulled the page off the screen has shown it
    /// going already.
    pub(super) fn back_by(&mut self, now: u64, drawn: bool) -> Vec<Effect> {
        let (l, mut fx) = (self.state.lang, Vec::new());
        if self.stack.len() <= 1 {
            return fx;
        }
        let morph = self.screen() == Screen::Search && self.stack[self.stack.len() - 2].0 == Screen::Home;
        if morph {
            self.start_morph(false, now);
        }
        let left = self.screen();
        match left {
            Screen::Mount => self.state.mount_failure = None,
            // A slot picked and not switched to is put down.
            Screen::Slot => self.state.next_slot = None,
            // So is a system; the page stays while the boot partitions are
            // being written, which nothing may cut short.
            Screen::Systems => {
                if self.state.systems_work.is_some() {
                    self.say(l.pick(texts::toast::back::WRITING), now);
                    return fx;
                }
                self.state.system_pick = None;
                self.state.asking_delete = false;
            }
            Screen::Files => {
                if let Some(up) = self.state.files.up() {
                    return vec![Effect::ReadDir(up)];
                }
                // Leaving the file manager puts a picked entry down.
                self.state.clip = None;
            }
            Screen::Choose(place) => {
                if let Some(up) = self.state.folder(Screen::Choose(place)).up() {
                    return vec![Effect::ReadPlace(place, up)];
                }
            }
            Screen::FileInfo => {
                self.state.file = None;
                self.state.asking_delete = false;
            }
            // Changes not saved are given up only by going back again while
            // the warning shows.
            Screen::Editor => {
                let changed = self.state.editor.as_ref().expect("the editor page shows a file").changed;
                if changed && self.unsaved_warned.is_none_or(|at| now >= at + TOAST_MS) {
                    self.unsaved_warned = Some(now);
                    self.say(l.pick(texts::toast::back::UNSAVED), now);
                    return fx;
                }
                self.state.editor = None;
                self.unsaved_warned = None;
            }
            // Turning the choice down leaves the entry picked, for another
            // folder.
            Screen::Clash => self.state.clash = None,
            Screen::WlanPassword => {
                self.state.joining = None;
                self.state.password.clear();
            }
            Screen::WlanNetwork => self.state.details = None,
            // A list open over the page closes first.
            Screen::WlanAdd => {
                if let Some(a) = self.state.adding.as_mut().filter(|a| a.choosing) {
                    a.choosing = false;
                    self.refresh();
                    return fx;
                }
                self.state.adding = None;
                self.state.password.clear();
            }
            Screen::FileJob => {
                if self.state.job.as_ref().is_some_and(|j| j.outcome.is_none()) {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                self.state.job = None;
            }
            Screen::Relabel => {
                if self.state.relabel.as_ref().is_some_and(|j| j.outcome.is_none()) {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                self.state.relabel = None;
            }
            // Work on a partition cannot be stopped halfway. Done, the
            // partition may hold another filesystem, and what is to be done
            // to it is chosen afresh.
            Screen::PartWork => {
                if self.state.part_work.as_ref().is_some_and(|w| w.outcome.is_none()) {
                    self.say(l.pick(texts::toast::back::WAIT), now);
                    return fx;
                }
                self.state.part_work = None;
                self.state.fs_action = FsAction::Repair;
            }
            Screen::RestoreBackup => {
                self.state.restore = None;
                self.state.asking_delete = false;
            }
            // The page stays while Skyward is updated: once downloaded, the
            // build is written at once, which nothing may cut short.
            Screen::SkywardUpdate => match self.state.skyward_job.as_ref().filter(|j| j.outcome.is_none()).map(|j| j.stage) {
                Some(Some(SkywardStage::Write { .. })) => {
                    self.say(l.pick(texts::toast::back::WRITING), now);
                    return fx;
                }
                Some(_) => {
                    self.say(l.pick(WAIT_OR_STOP), now);
                    return fx;
                }
                None => {}
            },
            // A restore that has begun writing cannot be stopped.
            Screen::BackupJob => {
                let job = self.state.backup_job.as_ref().expect("the backup page shows work");
                if job.outcome.is_none() {
                    let writing = job.restore && job.at.is_some_and(|a| !a.checking);
                    let said = if writing { texts::toast::back::RESTORING } else { WAIT_OR_STOP };
                    self.say(l.pick(said), now);
                    return fx;
                }
                self.state.backup_job = None;
            }
            _ => {}
        }
        if self.screen() == Screen::Installing {
            match self.state.install {
                // Checking a signature only reads the package, and stops
                // when the page is left.
                Install::Checking { stage: Some(Stage::Signature { .. }), .. } => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                Install::Checking { .. } => {
                    self.say(l.pick(texts::toast::back::READING), now);
                    return fx;
                }
                Install::Updating { .. } | Install::Scripting { .. } => {
                    self.say(l.pick(texts::toast::back::INSTALLING), now);
                    return fx;
                }
                Install::Flashing { .. } => {
                    self.say(l.pick(texts::toast::back::FLASHING), now);
                    return fx;
                }
                Install::Systeming { .. } => {
                    self.say(l.pick(texts::toast::back::INSTALLING), now);
                    return fx;
                }
                Install::Opening => {
                    self.say(l.pick(DATA_OPENING), now);
                    return fx;
                }
                Install::Wiping => {
                    self.say(l.pick(texts::toast::back::WIPING), now);
                    return fx;
                }
                Install::Starting
                | Install::Locked(_)
                | Install::Waiting
                | Install::Choosing(_)
                | Install::SystemChoosing(_)
                | Install::Confirming { .. }
                | Install::ConfirmingScript => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                // Rescue waits on for the computer's next command until
                // its page is left.
                Install::Finished { .. } | Install::Wiped(_) if self.state.source == Source::Rescue => {
                    self.state.install = Install::Ending;
                    fx.push(Effect::Cancel);
                }
                Install::Finished { .. } | Install::Wiped(_) => self.state.install = Install::Idle,
                Install::Idle | Install::Ending => {}
            }
        }
        if !morph && drawn {
            self.leave(true);
        }
        self.stack.pop();
        self.state.confirm = 0.0;
        self.settle_page();
        self.refresh();
        // Coming back to the list of backups looks for them again: one may
        // have been made, deleted or opened up to be read since. So does
        // coming back to the partitions after work on one, which may have
        // changed what it holds.
        match self.screen() {
            Screen::Restore => fx.push(self.find_backups()),
            Screen::Filesystem | Screen::Filesystems if left == Screen::PartWork => fx.push(Effect::ReadFilesystems),
            _ => {}
        }
        fx
    }

    /// What the page shown first asks of the runtime as it shows.
    pub fn arrive(&mut self) -> Vec<Effect> {
        self.arrive_at(self.screen())
    }

    /// What a page asks of the runtime as it opens: what it shows read, or
    /// started. The decrypt page opens the data partition, which is also
    /// how it learns which control a user's screen lock is answered with.
    pub(super) fn arrive_at(&mut self, s: Screen) -> Vec<Effect> {
        match s {
            Screen::Mount => vec![Effect::ReadMounts],
            Screen::Filesystems => vec![Effect::ReadFilesystems],
            Screen::Slot | Screen::Reboot => vec![Effect::ReadSlots],
            Screen::Systems => vec![Effect::ReadSystems],
            Screen::Restore => vec![self.find_backups()],
            Screen::Files => vec![Effect::ReadDir(self.state.files.path.clone())],
            Screen::Choose(place) if self.state.place_open(place) => vec![Effect::ReadPlace(place, self.state.folder(s).path.clone())],
            Screen::Search => {
                self.state.search.clear();
                self.state.layer = Layer::Letters;
                Vec::new()
            }
            Screen::Terminal => {
                self.state.layer = Layer::Letters;
                if self.term.is_none() {
                    self.shell_folder = "/".into();
                    vec![Effect::StartShell(self.shell_folder.clone())]
                } else {
                    Vec::new()
                }
            }
            Screen::Decrypt if matches!(self.state.data, Data::Closed | Data::Failed(_)) => self.open_data(),
            // As the stock updater does, the page asks the update server as
            // it opens, once a network is joined and nothing has been heard.
            Screen::Update => {
                let mut fx = vec![Effect::ReadUpdate];
                if self.state.update_server && matches!(self.state.check, Check::Idle | Check::Failed(_)) && self.state.online() {
                    self.state.check = Check::Asking;
                    fx.push(Effect::CheckUpdate);
                }
                fx
            }
            // Skyward's own page asks GitHub the same way.
            Screen::SkywardUpdate if matches!(self.state.skyward_check, SkywardCheck::Idle | SkywardCheck::Failed(_)) && self.state.online() => {
                self.state.skyward_check = SkywardCheck::Asking;
                vec![Effect::CheckSkyward]
            }
            Screen::Wlan if self.state.wlan.as_ref().is_some_and(|w| w.power == Power::On) => vec![Effect::ScanWlan],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;

    #[test]
    fn rows_open_pages_and_back_returns() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        tap(&mut app, &f, &i, Action::Open(Screen::Settings));
        assert_eq!(app.screen(), Screen::Settings);
        tap_back(&mut app, &f, &i, 0);
        assert_eq!(app.screen(), Screen::Home);
        app.key(Key::Back, 0);
        assert_eq!(app.screen(), Screen::Home, "back on the first page stays there");
    }

    #[test]
    fn the_back_control_stays_put_while_the_page_scrolls() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        tap(&mut app, &f, &i, Action::Open(Screen::Settings));
        app.frame(&f, &i);
        app.touch(Touch::Down(700.0, 2400.0), &f, 0);
        app.touch(Touch::Move(700.0, 2200.0), &f, 0);
        app.touch(Touch::Move(700.0, 2100.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.top().1, 300.0);
        tap_back(&mut app, &f, &i, 0);
        assert_eq!(app.screen(), Screen::Home);
    }

    #[test]
    fn a_page_opened_slides_in_over_the_page_it_opens_from_and_back_off_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        let home = shot(&c);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
        assert_eq!(app.screen(), Screen::Mount);
        assert_eq!(app.deadline(1000), Some(0), "it slides frame by frame");
        // The slide sets off with its first frame, the page opened still
        // past the right edge.
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        let first = shot(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1050);
        let mid = shot(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);
        assert!(app.turn.is_none(), "come to rest, the page left is let go");
        assert_ne!(app.deadline(3000), Some(0));
        let mount = shot(&c);
        for y in [H / 3, H / 2, H - 200] {
            check_slide(&first, &mount, &home, y, (W, 0, 256));
            check_slide(&mid, &mount, &home, y, slid(false, 50));
        }
        let (front, _, _) = slid(false, 50);
        assert!((front as f32 / W as f32 - 0.728).abs() < 0.01, "about three quarters of the way in after 50 ms: {front}");

        tap_back(&mut app, &f, &i, 4000);
        assert_eq!(app.screen(), Screen::Home);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 4000);
        let first = shot(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 4060);
        let mid = shot(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 6000);
        assert!(app.turn.is_none());
        let home = shot(&c);
        for y in [H / 3, H / 2, H - 200] {
            check_slide(&first, &mount, &home, y, (0, W / 4, ((1.0 - BEHIND_DIM) * 256.0).round() as u32));
            check_slide(&mid, &mount, &home, y, slid(true, 60));
        }
    }

    #[test]
    fn a_page_sliding_has_its_leading_corners_rounded_as_the_screens_are() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        app.set_screen_corners(140.0);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        let home = shot(&c);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1050);
        let mid = shot(&c);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 3000);
        let mount = shot(&c);
        let (front, behind, keep) = slid(false, 50);
        let at = |c: &Canvas, x: u32, y: u32| c.data[((y * W + x) * 4) as usize] as u32;
        let y = H - 3;
        assert_eq!(at(&mid, front + 5, y), (at(&home, front + 5 + behind, y) * keep) >> 8, "the corner's tip shows the page behind");
        assert_eq!(at(&mid, front + 140, y), at(&mount, 140, y), "past the corner's curve the page in front is whole");
        assert_eq!(at(&mid, front + 5, H / 2), at(&mount, 5, H / 2), "between the corners its edge is straight");
    }

    #[test]
    fn a_finger_brings_a_sliding_page_to_rest_and_a_page_never_shown_gives_way_at_once() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Home, W, H, PPD);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        tap_at(&mut app, &f, &i, Action::Open(Screen::Mount), 1000);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 1000);
        app.touch(Touch::Down(W as f32 / 2.0, H as f32 / 2.0), &f, 1030);
        assert!(app.turn.is_none(), "a finger brings it to rest");
        app.touch(Touch::Up, &f, 1060);

        // A page rendered anew since it last showed is not drawn as it
        // stands: what replaces it shows at once.
        app.refresh();
        tap_back(&mut app, &f, &i, 2000);
        assert_eq!(app.screen(), Screen::Home);
        assert!(app.turn.is_none());
    }

    #[test]
    fn every_unfinished_entry_now_opens_its_page() {
        let (f, i) = (fonts(), DeviceInfo { slot: Some(Slot::B), ..info() });
        let routes: [(Screen, Screen); 13] = [
            (Screen::Home, Screen::Search),
            (Screen::Home, Screen::Backup),
            (Screen::Home, Screen::Restore),
            (Screen::Home, Screen::Advanced),
            (Screen::Advanced, Screen::Files),
            (Screen::Advanced, Screen::Terminal),
            (Screen::Advanced, Screen::Logs),
            (Screen::Settings, Screen::Language),
            (Screen::Settings, Screen::TimeZone),
            (Screen::Settings, Screen::Terminal),
            (Screen::About, Screen::Logs),
            (Screen::Specs, Screen::License),
            (Screen::Specs, Screen::FontLicense),
        ];
        for (from, to) in routes {
            let mut app = App::new(from, W, H, PPD);
            tap(&mut app, &f, &i, Action::Open(to));
            assert_eq!(app.screen(), to, "from {from:?}");
        }
        let mut app = App::new(Screen::Reboot, W, H, PPD);
        tap(&mut app, &f, &i, Action::Open(Screen::Slot));
        assert_eq!(app.screen(), Screen::Slot);
    }

    /// A page pulled off from deep inside an app folds into its own icon,
    /// not the one the app was opened from: a pull takes off the page it
    /// holds, whatever lies under it, and each app keeps its own tile.
    #[test]
    fn a_page_pulled_off_folds_into_its_own_icon() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Desktop, W, H, PPD);
        let mut c = Canvas::new(W, H);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 0);
        // The clock out of its icon, then the calculator over it: the pull
        // is let go over a page the desktop's icon holds for the calculator
        // alone.
        app.open(Screen::Clock);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 16);
        app.push(Screen::Calculator);
        app.compose(&mut c, &f, &i, "12:00", "11月16日 周一", 32);
        let t = app.theme();
        let (clock, calc) = (pages::tile_of(&t, Screen::Clock), pages::tile_of(&t, Screen::Calculator));
        assert!(clock.is_some() && calc.is_some() && clock != calc, "the two keep tiles of their own");
        assert_eq!(app.desk_icon, clock, "the page was opened out of the clock's icon");
        app.leave_by_foot(200.0);
        let icon = app.turn.as_ref().and_then(|t| t.icon);
        assert!(!app.turn.as_ref().is_some_and(|t| t.lifted < 1.0), "the fold begins where the pull left it");
        assert_eq!(icon, calc, "the page folds into its own icon");
        assert_ne!(icon, clock, "not into the one its opener came out of");
        assert_eq!(app.screen(), Screen::Desktop, "and the desktop is what it leaves for");
    }
}
