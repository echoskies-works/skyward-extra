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
        // seconds first.
        if self.banner_allowed(self.now) {
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
}
