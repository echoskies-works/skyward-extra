//! What the menu offers, and how a press moves through it.
//!
//! Kept apart from the drawing and the hardware so the whole thing can be
//! tested on a laptop: three keys in, one decision out.

use crate::lang::Lang;

/// One thing the menu can do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Hand over to Aether and get out of the way.
    Continue,
    /// Wipe what Aether keeps.
    Reset,
    /// The sub-menu for a phone that will not start: reinstall, roll back,
    /// repair, the things that need more than two keys to explain.
    Troubleshoot,
    /// Hand the phone back to the bootloader's own fastboot, so an image can
    /// be written over USB. The menu cannot flash anything, and should not
    /// try: that is the one job the bootloader already does well.
    Fastboot,
    PowerOff,
}

impl Action {
    /// The words on the line, as they appear on the panel.
    ///
    /// Both languages name the same five things, so the pair is written here
    /// rather than in the drawing code: a line that exists in one language
    /// and not the other is a bug the compiler cannot see, but a pair that is
    /// always written together is one the eye can.
    pub fn label(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Action::Continue => ["Continue Startup", "继续启动"],
            Action::Reset => ["Reset Skyward-Extra", "重置 Skyward-Extra"],
            Action::Troubleshoot => ["Troubleshoot", "疑难解答"],
            Action::Fastboot => ["Start Fastboot mode", "启动 Fastboot 模式"],
            Action::PowerOff => ["Power off", "关机"],
        })
    }

    /// Whether choosing it drops what is on the phone. Those ask first.
    pub fn is_destructive(self) -> bool {
        match self {
            Action::Continue | Action::Troubleshoot | Action::Fastboot => false,
            Action::Reset | Action::PowerOff => true,
        }
    }
}

/// The lines, in the order they are drawn.
///
/// Five is the whole menu: two working keys have to walk it one line per
/// press, and anything longer stops being something you can use without
/// looking.
pub const ITEMS: [Action; 5] = [Action::Continue, Action::Reset, Action::Troubleshoot, Action::Fastboot, Action::PowerOff];

/// A key the menu understands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Up,
    Down,
    Power,
}

/// What a press did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    /// Nothing moved: the cursor was already at the end.
    Still,
    /// Redraw the screen.
    Redraw,
    /// The user settled on this.
    Done(Action),
}

/// When Aether stopped.
///
/// The two cases look the same from here — a dead process and an exit code —
/// but they are different things to the person holding the phone: one never
/// got a screen, the other was in use a second ago. The panel says which.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Crash {
    /// It never got as far as drawing anything.
    Startup,
    /// It was up, and then it was not.
    Runtime,
}

/// Why Aether stopped, when it stopped on purpose.
///
/// A number cannot say this: 67 is one code for one thing — an app that is
/// named on the desktop and not built yet — and three icons stop the same
/// way. So the app is said in words, and the words are written here rather
/// than in Aether's own crate: the panel is the one that prints them, in the
/// language the panel speaks, and Aether only leaves the word for the app
/// behind. [`Reason::tag`] is that word, and it is what Aether writes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// The player.
    Music,
    /// The browser.
    Browser,
    /// The tasks page.
    Tasks,
    /// The weather page.
    Weather,
}

impl Reason {
    /// The line on the panel, after "Error code: 67" and a row of air.
    ///
    /// Said the way a person would say it, and about the app rather than
    /// about the code: whoever tapped the icon knows which one they tapped,
    /// and what they need to be told is that it is not their phone that is
    /// broken.
    pub fn label(self, lang: Lang) -> &'static str {
        lang.pick(match self {
            Reason::Music => ["The player is not built yet.", "音乐还没做好"],
            Reason::Browser => ["The browser is not built yet.", "浏览器还没做好"],
            Reason::Tasks => ["The tasks page is not built yet.", "任务管理器还没做好"],
            Reason::Weather => ["The weather page is not built yet.", "天气还没做好"],
        })
    }

    /// The word Aether writes in its stop note. Short, lower case and
    /// ASCII, since it travels through a file rather than through the panel.
    pub fn tag(self) -> &'static str {
        match self {
            Reason::Music => "music",
            Reason::Browser => "browser",
            Reason::Tasks => "tasks",
            Reason::Weather => "weather",
        }
    }

    /// The word back into the app, for the note Aether left behind. An
    /// unknown word is no reason at all rather than a wrong one: a note
    /// written by a newer Aether than this panel knows still leaves the
    /// code on the screen, which is the part that narrows anything down.
    pub fn from_tag(tag: &str) -> Option<Reason> {
        ALL.into_iter().find(|reason| reason.tag() == tag.trim())
    }
}

/// Every reason there is, so the words the panel prints and the words Aether
/// writes stay in one list.
const ALL: [Reason; 4] = [Reason::Music, Reason::Browser, Reason::Tasks, Reason::Weather];

/// What is on the screen right now.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    /// The list, with `sel` on the cursor.
    List { sel: usize },
    /// "Are you sure" for a destructive item; `yes` is where the cursor is,
    /// and it starts on "No".
    Confirm { action: Action, yes: bool },
    /// Aether died, and `code` is what it left behind. No cursor and no
    /// list: there is one key to press and one thing it does, so the panel
    /// only has to say so.
    Failure {
        crash: Crash,
        code: i32,
        /// Why, where it stopped on purpose. Nothing is said when it did
        /// not: a phone that fell over has no reason line to give, and the
        /// screen is a row shorter for it.
        reason: Option<Reason>,
    },
}

/// The cursor and, when a destructive item has been pressed, the question.
pub struct Menu {
    sel: usize,
    /// "No" or "Yes" once the question is up.
    asking: Option<bool>,
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new()
    }
}

impl Menu {
    pub fn new() -> Self {
        Menu { sel: 0, asking: None }
    }

    pub fn view(&self) -> View {
        match (self.asking, ITEMS.get(self.sel)) {
            (Some(yes), Some(&action)) => View::Confirm { action, yes },
            // `sel` is always an index into ITEMS, so the fallback is only
            // here to keep the match exhaustive.
            _ => View::List { sel: self.sel },
        }
    }

    /// Feed one key press. Releases and repeats are the caller's problem.
    pub fn key(&mut self, key: Key) -> Step {
        if let Some(yes) = self.asking {
            return match key {
                // The two keys toggle; the question only has two answers, so
                // either direction does the same thing.
                Key::Up | Key::Down => {
                    self.asking = Some(!yes);
                    Step::Redraw
                }
                Key::Power if yes => Step::Done(ITEMS[self.sel]),
                Key::Power => {
                    self.asking = None;
                    Step::Redraw
                }
            };
        }
        match key {
            Key::Up => {
                let next = self.sel.saturating_sub(1);
                if next == self.sel {
                    return Step::Still;
                }
                self.sel = next;
                Step::Redraw
            }
            Key::Down => {
                let next = (self.sel + 1).min(ITEMS.len() - 1);
                if next == self.sel {
                    return Step::Still;
                }
                self.sel = next;
                Step::Redraw
            }
            Key::Power => {
                let action = ITEMS[self.sel];
                if action.is_destructive() {
                    // Always "No" first: a pocket press must not wipe a phone.
                    self.asking = Some(false);
                    Step::Redraw
                } else {
                    Step::Done(action)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_starts_at_the_top_and_stops_there() {
        let mut menu = Menu::new();
        assert_eq!(menu.view(), View::List { sel: 0 });
        assert_eq!(menu.key(Key::Up), Step::Still);
        assert_eq!(menu.view(), View::List { sel: 0 });
    }

    #[test]
    fn the_cursor_stops_at_the_last_line() {
        let mut menu = Menu::new();
        for _ in 1..ITEMS.len() {
            assert_eq!(menu.key(Key::Down), Step::Redraw);
        }
        assert_eq!(menu.view(), View::List { sel: ITEMS.len() - 1 });
        // The bottom of the list, not wrapped round to the top: wrapping
        // makes a short list feel like it has an edge you can fall off.
        assert_eq!(menu.key(Key::Down), Step::Still);
    }

    #[test]
    fn continuing_needs_no_question() {
        let mut menu = Menu::new();
        assert_eq!(menu.key(Key::Power), Step::Done(Action::Continue));
    }

    #[test]
    fn a_destructive_line_asks_and_starts_on_no() {
        let mut menu = Menu::new();
        menu.key(Key::Down);
        assert_eq!(menu.key(Key::Power), Step::Redraw);
        assert_eq!(menu.view(), View::Confirm { action: Action::Reset, yes: false });
        // Power on "No" backs out and changes nothing.
        assert_eq!(menu.key(Key::Power), Step::Redraw);
        assert_eq!(menu.view(), View::List { sel: 1 });
    }

    #[test]
    fn yes_goes_through() {
        let mut menu = Menu::new();
        for _ in 0..ITEMS.len() - 1 {
            menu.key(Key::Down);
        }
        menu.key(Key::Power);
        assert_eq!(menu.view(), View::Confirm { action: Action::PowerOff, yes: false });
        menu.key(Key::Up);
        assert_eq!(menu.view(), View::Confirm { action: Action::PowerOff, yes: true });
        assert_eq!(menu.key(Key::Power), Step::Done(Action::PowerOff));
    }

    #[test]
    fn fastboot_and_troubleshoot_are_reached_without_a_question() {
        // Neither one drops anything, so neither asks first: a question here
        // would be a press for nothing.
        let mut menu = Menu::new();
        menu.key(Key::Down);
        menu.key(Key::Down);
        assert_eq!(menu.key(Key::Power), Step::Done(Action::Troubleshoot));
        let mut menu = Menu::new();
        for _ in 0..3 {
            menu.key(Key::Down);
        }
        assert_eq!(menu.key(Key::Power), Step::Done(Action::Fastboot));
    }

    #[test]
    fn either_key_toggles_the_answer() {
        let mut menu = Menu::new();
        menu.key(Key::Down);
        menu.key(Key::Power);
        menu.key(Key::Down);
        assert_eq!(menu.view(), View::Confirm { action: Action::Reset, yes: true });
        menu.key(Key::Up);
        assert_eq!(menu.view(), View::Confirm { action: Action::Reset, yes: false });
    }

    #[test]
    fn backing_out_leaves_the_cursor_where_it_was() {
        let mut menu = Menu::new();
        menu.key(Key::Down);
        menu.key(Key::Power);
        menu.key(Key::Power);
        assert_eq!(menu.view(), View::List { sel: 1 });
        assert_eq!(menu.key(Key::Power), Step::Redraw, "asking again, not going through");
    }

    #[test]
    fn a_reason_and_its_word_are_the_same_app() {
        // Aether writes the word and the panel prints the line, so the two
        // have to name the same app: a note that reads "music" and a screen
        // that says the browser is worse than no reason line at all.
        for reason in [Reason::Music, Reason::Browser, Reason::Tasks] {
            assert_eq!(Reason::from_tag(reason.tag()), Some(reason), "{reason:?}");
            for lang in [Lang::En, Lang::Zh] {
                assert!(!reason.label(lang).is_empty(), "{reason:?} {lang:?}");
            }
        }
        // The note is a line of a file, so it arrives with a newline on it.
        assert_eq!(Reason::from_tag("music\n"), Some(Reason::Music));
        // A word this panel does not know is no reason at all, rather than a
        // wrong one: the code on the screen is still right.
        assert_eq!(Reason::from_tag("telephone"), None);
        assert_eq!(Reason::from_tag(""), None);
    }

    #[test]
    fn every_reason_says_what_the_app_is_in_each_language() {
        // The words themselves, not just that they exist: these are the lines
        // a person reads after tapping an icon, and they are said about the
        // app rather than about the code.
        let said = [
            (Reason::Music, ["The player is not built yet.", "音乐还没做好"]),
            (Reason::Browser, ["The browser is not built yet.", "浏览器还没做好"]),
            (Reason::Tasks, ["The tasks page is not built yet.", "任务管理器还没做好"]),
            (Reason::Weather, ["The weather page is not built yet.", "天气还没做好"]),
        ];
        for (reason, [en, zh]) in said {
            assert_eq!(reason.label(Lang::En), en);
            assert_eq!(reason.label(Lang::Zh), zh);
        }
    }
}
