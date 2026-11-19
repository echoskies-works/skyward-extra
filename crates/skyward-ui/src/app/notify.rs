//! The notices the panel keeps: what the system and the pages put there,
//! and the rules that keep them in order.
//!
//! A notice belongs to the panel from the moment it is put, not to whoever
//! put it: an app that goes away does not take its notices with it, and one
//! cleared from the list is gone for good. Only a finger or the phone going
//! off empties the list — nothing runs out on its own.

use super::*;

/// How many notices stand at most. Past this the oldest goes, so a page
/// that keeps posting cannot grow the list without end.
const MAX: usize = 50;

/// The name the clock's pages answer by, as the panel knows the app.
pub const CLOCK_APP: &str = "clock";

/// What the clock says while one of its counts runs with its page left
/// behind: one notice each, and a tap goes back to it.
const STOPWATCH_RUNNING: Text = ["秒表仍在运行中", "碼錶仍在執行中", "Stopwatch still running"];
const TIMER_RUNNING: Text = ["计时器仍在运行中", "計時器仍在執行中", "Timer still running"];
const BACK_TO_CLOCK: Text = ["点击此处以返回时钟", "點擊此處以返回時鐘", "Tap to go back to the clock"];

/// Where one of the clock's counts stands with the panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Shown {
    /// No notice was put for it with the page left behind.
    #[default]
    No,
    /// One stands in the list.
    Yes,
    /// One was put and a finger took it away: it stays away until the page
    /// is opened again, so a finger clearing the list is obeyed rather than
    /// fought by the next frame.
    Cleared,
}

/// Who put a notice there, which is also what a row of them is cleared by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// The system's own: the battery running low, a timer that has run out,
    /// work the recovery is doing.
    System,
    /// A page's app, by the name the desktop knows it by.
    App(&'static str),
}

/// What a notice is for, which sets how it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Something happened once: the battery is low, a timer ran out, a
    /// package finished installing.
    Event,
    /// Work under way, with how far it has come in [`Notification::progress`].
    Progress,
    /// Something did not go: an install that failed, a network that could
    /// not be reached.
    Failure,
}

/// A notice: one card in the panel.
#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub source: Origin,
    /// The row it belongs to. A second notice with the same source, tag and
    /// id takes the first's place rather than standing beside it, which is
    /// how work under way moves its own row along instead of leaving a
    /// trail of percentages behind it.
    pub tag: &'static str,
    pub id: u32,
    pub kind: Kind,
    pub title: String,
    pub body: String,
    /// When it was put, in wall-clock milliseconds: the times on the cards
    /// and "3 minutes ago" have to stay right across a restart, which a
    /// count from when the panel started would not.
    pub at: u64,
    /// Where a tap on it goes.
    pub action: Option<Action>,
    /// How far work under way has come: done of all, the bar under the
    /// words. None for anything that is not [`Kind::Progress`].
    pub progress: Option<(u32, u32)>,
    /// Whether it stays in the list without standing over the page a few
    /// seconds first. What a page puts as it is left behind — a count that
    /// goes on running — is a state, not news: the person who left it
    /// knows, and a card over the page would only be in the way.
    pub quiet: bool,
}

impl Notification {
    /// A notice of something that happened once.
    pub fn event(
        source: Origin,
        tag: &'static str,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Notification {
        Notification {
            source,
            tag,
            id: 0,
            kind: Kind::Event,
            title: title.into(),
            body: body.into(),
            at: 0,
            action: None,
            progress: None,
            quiet: false,
        }
    }

    /// Work under way: `done` of `all`, as it stands now.
    pub fn progress(
        source: Origin,
        tag: &'static str,
        title: impl Into<String>,
        body: impl Into<String>,
        done: u32,
        all: u32,
    ) -> Notification {
        Notification {
            kind: Kind::Progress,
            progress: Some((done, all)),
            ..Notification::event(source, tag, title, body)
        }
    }

    /// Something that did not go.
    pub fn failure(
        source: Origin,
        tag: &'static str,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Notification {
        Notification { kind: Kind::Failure, ..Notification::event(source, tag, title, body) }
    }

    /// Where the work has come to, from 0 to 1, or none where the notice
    /// is not about work under way.
    pub fn level(&self) -> Option<f32> {
        match self.progress {
            Some((_, 0)) | None => None,
            Some((done, all)) => Some((done.min(all) as f32) / all as f32),
        }
    }

    /// Whether this is the same row as `other`: a notice put again takes
    /// this one's place.
    pub fn same_row(&self, other: &Notification) -> bool {
        self.source == other.source && self.tag == other.tag && self.id == other.id
    }
}

/// The notices standing, newest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notifier {
    notices: Vec<Notification>,
}

impl Notifier {
    /// Put a notice, or move the one of its row along: either way it stands
    /// first, as the newest thing there.
    pub fn post(&mut self, notice: Notification) {
        self.notices.retain(|standing| !standing.same_row(&notice));
        self.notices.insert(0, notice);
        self.notices.truncate(MAX);
    }

    /// Take one row away: work that ended, a notice the page itself puts
    /// right.
    pub fn cancel(&mut self, source: Origin, tag: &str, id: u32) {
        self.notices.retain(|n| !(n.source == source && n.tag == tag && n.id == id));
    }

    /// Take every notice of one source away, as an app going away does.
    pub fn clear_source(&mut self, source: Origin) {
        self.notices.retain(|n| n.source != source);
    }

    /// Take them all away.
    pub fn clear(&mut self) {
        self.notices.clear();
    }

    /// The notices standing, newest first.
    pub fn iter(&self) -> impl Iterator<Item = &Notification> {
        self.notices.iter()
    }

    pub fn len(&self) -> usize {
        self.notices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notices.is_empty()
    }
}

impl App {
    /// Put a notice in the panel. It belongs to the panel from here on: the
    /// page that put it may go away and it stands, and nothing but a finger
    /// on its clear key or the phone going off takes it away. A notice that
    /// names no time is stamped with the one the panel keeps, which is the
    /// clock the cards read.
    pub fn notify(&mut self, mut notice: Notification) {
        if notice.at == 0 {
            notice.at = self.now;
        }
        // A notice that comes while the panel is out is already in the list
        // under the finger; one that comes over a page stands there a few
        // seconds first. A quiet one is a state rather than news, and stays
        // in the list alone.
        if !notice.quiet && self.banner_allowed(self.now) {
            self.banner = Some(shade::Banner::new(notice.clone(), self.now));
        }
        self.notifier.post(notice);
        // What the panel holds has changed: drawn again from scratch, and
        // the loop woken by it, so it shows without waiting for a finger.
        self.shade_gone();
        self.dirty = true;
    }

    /// The notices standing, newest first.
    pub fn notices(&self) -> impl Iterator<Item = &Notification> {
        self.notifier.iter()
    }

    /// Whether the page in front belongs to the clock: the page itself and
    /// the one its gear opens are one app, and a count running with either
    /// in front is not running in the background.
    fn in_clock_app(&self) -> bool {
        matches!(self.screen(), Screen::Clock | Screen::ClockSound)
    }

    /// A count of the clock that runs with its page left behind says so in
    /// the panel, one notice for each of the two, and is put right as soon
    /// as the page is back in front or the count stops. This is the first of
    /// the panel's real sources: what it puts comes from the phone's own
    /// state, not from a test's words.
    pub(super) fn sync_clock_notices(&mut self) {
        let here = self.in_clock_app();
        let (stopwatch, timer) = {
            let clock = &self.state.clock;
            (!here && clock.stop_since.is_some(), !here && clock.left_since.is_some())
        };
        self.clock_stopwatch =
            self.settle_clock_notice(self.clock_stopwatch, stopwatch, "stopwatch", STOPWATCH_RUNNING);
        self.clock_timer = self.settle_clock_notice(self.clock_timer, timer, "timer", TIMER_RUNNING);
    }

    /// One count's notice: put when it starts to run in the background, taken
    /// away when it stops or the page comes back, and left alone once a
    /// finger has cleared it.
    fn settle_clock_notice(&mut self, shown: Shown, running: bool, tag: &'static str, title: Text) -> Shown {
        let mut shown = shown;
        // A notice that stood and no longer does was taken by a finger: it
        // stays taken until the page is opened again.
        if shown == Shown::Yes
            && !self.notifier.iter().any(|n| n.source == Origin::App(CLOCK_APP) && n.tag == tag)
        {
            shown = Shown::Cleared;
        }
        match (running, shown) {
            (true, Shown::No) => {
                let l = self.state.lang;
                let mut notice = Notification::event(Origin::App(CLOCK_APP), tag, l.pick(title), l.pick(BACK_TO_CLOCK));
                notice.action = Some(Action::Open(Screen::Clock));
                notice.quiet = true;
                self.notify(notice);
                Shown::Yes
            }
            (false, Shown::Yes) => {
                self.notifier.cancel(Origin::App(CLOCK_APP), tag, 0);
                self.shade_gone();
                self.dirty = true;
                Shown::No
            }
            (false, Shown::Cleared) => Shown::No,
            (_, other) => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(tag: &'static str) -> Notification {
        Notification::event(Origin::System, tag, "title", "body")
    }

    /// A notice of the same row takes the place of the one standing rather
    /// than standing beside it: work under way moves its row, it does not
    /// leave a trail of percentages.
    #[test]
    fn posting_the_same_row_moves_it_along() {
        let mut notifier = Notifier::default();
        notifier.post(Notification::progress(Origin::System, "flash", "正在刷入", "system", 10, 100));
        notifier.post(Notification::progress(Origin::System, "flash", "正在刷入", "system", 42, 100));
        assert_eq!(notifier.len(), 1, "one row, not two");
        assert_eq!(notifier.iter().next().unwrap().level(), Some(0.42), "and it says where the work is now");
    }

    /// The same tag from another source is another row: two apps may both
    /// have a notice about "weather" without one taking the other's place.
    #[test]
    fn the_same_tag_under_another_source_is_another_row() {
        let mut notifier = Notifier::default();
        notifier.post(one("weather"));
        notifier.post(Notification::event(Origin::App("market"), "weather", "title", "body"));
        assert_eq!(notifier.len(), 2, "each source keeps its own");
    }

    /// The newest stands first, and only so many stand at all.
    #[test]
    fn the_newest_stands_first_and_the_list_is_capped() {
        let mut notifier = Notifier::default();
        for i in 0..(MAX + 5) {
            let mut notice = one("row");
            notice.id = i as u32;
            notifier.post(notice);
        }
        assert_eq!(notifier.len(), MAX, "capped");
        assert_eq!(notifier.iter().next().unwrap().id, (MAX + 4) as u32, "newest first");
        assert!(notifier.iter().all(|n| n.id >= 5), "the oldest went");
    }

    /// Clearing one source leaves the others standing.
    #[test]
    fn clearing_one_source_leaves_the_others() {
        let mut notifier = Notifier::default();
        notifier.post(one("a"));
        notifier.post(Notification::event(Origin::App("market"), "b", "title", "body"));
        notifier.clear_source(Origin::System);
        assert_eq!(notifier.len(), 1, "only the app's is left");
        notifier.clear();
        assert!(notifier.is_empty(), "and clearing takes that too");
    }

    /// A row is taken away by what put it, and nothing else goes with it.
    #[test]
    fn cancelling_takes_one_row() {
        let mut notifier = Notifier::default();
        notifier.post(one("a"));
        notifier.post(one("b"));
        notifier.cancel(Origin::System, "a", 0);
        assert_eq!(notifier.len(), 1, "one row went");
        assert_eq!(notifier.iter().next().unwrap().tag, "b", "the other stands");
    }

    /// Work with nothing done of nothing has no place to be: no bar shows.
    #[test]
    fn work_of_no_whole_has_no_level() {
        let notice = Notification::progress(Origin::System, "x", "t", "b", 0, 0);
        assert_eq!(notice.level(), None);
        let over = Notification::progress(Origin::System, "x", "t", "b", 120, 100);
        assert_eq!(over.level(), Some(1.0), "past the whole still reads as the whole");
    }

    /// A phone on the desktop, the clock's page a step away.
    fn phone() -> App {
        App::new(Screen::Desktop, 400, 800, 2.0)
    }

    /// The clock's page opened over the desktop, so back leaves it.
    fn at_clock(app: &mut App) {
        app.act(Action::Open(Screen::Clock), 0);
    }

    /// A stopwatch running with its page left behind says so in the panel,
    /// and a tap on it goes back to the page. It is quiet: the one who left
    /// the page knows the watch is on, and a card over the page would only
    /// be in the way.
    #[test]
    fn a_stopwatch_left_running_says_so_in_the_panel() {
        let mut app = phone();
        at_clock(&mut app);
        app.state.clock.stop_since = Some(0);
        app.tick(10);
        assert!(app.notifier.is_empty(), "on its page the clock says nothing");

        app.act(Action::Back, 20);
        app.tick(30);
        let notice = app.notifier.iter().next().expect("the panel has it");
        assert_eq!(notice.source, Origin::App(CLOCK_APP));
        assert_eq!(notice.tag, "stopwatch");
        assert_eq!(notice.title, "秒表仍在运行中");
        assert_eq!(notice.body, "点击此处以返回时钟");
        assert_eq!(notice.action, Some(Action::Open(Screen::Clock)), "a tap goes back to it");
        assert!(notice.quiet, "and it does not stand over the page");
        assert!(app.banner.is_none(), "no card came over the page");
    }

    /// A timer running with its page left behind says so too, on its own row.
    #[test]
    fn a_timer_left_running_says_so_in_the_panel() {
        let mut app = phone();
        app.state.clock.span = 60_000;
        app.state.clock.left = 60_000;
        app.state.clock.left_since = Some(0);
        app.tick(10);
        let notice = app.notifier.iter().next().expect("the panel has it");
        assert_eq!(notice.tag, "timer");
        assert_eq!(notice.title, "计时器仍在运行中");
        assert_eq!(notice.body, "点击此处以返回时钟");
        assert_eq!(notice.action, Some(Action::Open(Screen::Clock)));
    }

    /// Both counts at once stand as two rows: they are two things to go
    /// back for.
    #[test]
    fn a_stopwatch_and_a_timer_left_running_stand_as_two_rows() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.state.clock.span = 60_000;
        app.state.clock.left = 60_000;
        app.state.clock.left_since = Some(0);
        app.tick(10);
        assert_eq!(app.notifier.len(), 2, "one row for each");
    }

    /// Coming back to the clock takes the notices away: there is nothing
    /// left behind any more.
    #[test]
    fn coming_back_to_the_clock_takes_its_notice_away() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.tick(10);
        assert_eq!(app.notifier.len(), 1);
        at_clock(&mut app);
        app.tick(20);
        assert!(app.notifier.is_empty(), "back in front, the panel has nothing to say");
    }

    /// A count that stops with the page left behind takes its notice away by
    /// itself: only one that still runs is worth going back for.
    #[test]
    fn a_stopwatch_that_stops_takes_its_notice_away() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.tick(10);
        assert_eq!(app.notifier.len(), 1);
        app.state.clock.stop_since = None;
        app.tick(20);
        assert!(app.notifier.is_empty(), "the watch stands still, the panel is quiet");
    }

    /// A finger that clears the list is obeyed: the count going on behind it
    /// does not put the notice back frame after frame.
    #[test]
    fn a_notice_cleared_by_a_finger_is_not_put_back() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.tick(10);
        app.notifier.clear();
        app.tick(20);
        app.tick(30);
        assert!(app.notifier.is_empty(), "the finger is obeyed");
    }

    /// Cleared or not, a later errand tells it again: the page is opened
    /// again and left again, and the notice comes back.
    #[test]
    fn leaving_the_clock_again_after_clearing_tells_it_again() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.tick(10);
        app.notifier.clear();
        app.tick(20);
        assert!(app.notifier.is_empty());

        at_clock(&mut app);
        app.tick(30);
        app.act(Action::Back, 40);
        app.tick(50);
        assert_eq!(app.notifier.len(), 1, "a later errand is told again");
    }

    /// The page the clock's gear opens is the clock too: a count running
    /// with it in front is not running in the background.
    #[test]
    fn the_clocks_own_page_is_still_the_clock() {
        let mut app = phone();
        app.state.clock.stop_since = Some(0);
        app.act(Action::Open(Screen::ClockSound), 0);
        app.tick(10);
        assert!(app.notifier.is_empty(), "a page opened from the clock is the clock");
    }

    /// A timer that runs out with the page left behind keeps to the sheet
    /// and the motor: the panel gets nothing of its end, and the notice of
    /// its running goes as the count stops.
    #[test]
    fn a_timer_that_ends_keeps_to_the_sheet() {
        let mut app = phone();
        app.state.clock.span = 60_000;
        app.state.clock.left = 0;
        app.state.clock.left_since = Some(0);
        app.tick(10);
        assert!(app.sheet.is_some(), "its end stands in the sheet");
        assert!(app.notifier.is_empty(), "and the panel is told nothing");
    }
}
