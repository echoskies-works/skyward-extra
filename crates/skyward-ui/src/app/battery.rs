//! The battery running down, said on a card: twenty per cent, ten, three,
//! one each, and only while the phone runs on its own charge.
//!
//! A charger in the phone is not a phone running down. Said as the level
//! came up, the card would come back at every step of the charge — twenty
//! per cent reached on the cable is the one thing that must not read as
//! running low — so nothing is said while it charges, the card standing is
//! taken away, and the step already said stays said until the battery is
//! back above [`RECOVERED`].

use crate::tr;
use super::*;

/// The levels a battery running down is said at, from the least short of
/// charge to the last: `20`, `10` and `3` per cent. Past the last there is
/// nothing worth saying — a battery that low goes off before the words are
/// read.
pub(super) const BATTERY_LEVELS: [u8; 3] = [20, 10, 3];

/// A battery back above this has been charged, and a run down from there is
/// a new one that may say its levels over. The two points above the lowest
/// step keep a battery sitting at a level — a charger that stops at twenty,
/// a gauge that flickers — from being said frame after frame.
pub(super) const RECOVERED: u8 = 22;

/// The card's heading, and the one key that puts it away.
const LOW_BATTERY: Text = tri("电池电量不足", "電池電量不足", "Battery low");
const OK: Text = tri("确定", "確定", "OK");

/// What the card says of a battery at `level`.
fn words(l: Lang, level: u8) -> String {
    tr!(
        l,
        "电量仅剩余 {level}%，请及时充电。",
        "電量僅剩餘 {level}%，請及時充電。",
        "Only {level}% is left. Charge it soon."
    )
}

/// Which step of [`BATTERY_LEVELS`] a battery at `level` stands at, nought
/// for one above them all. A battery at the level itself is at that step:
/// twenty per cent is where the first word is said.
fn step(level: u8) -> u8 {
    match level {
        l if l <= BATTERY_LEVELS[2] => 3,
        l if l <= BATTERY_LEVELS[1] => 2,
        l if l <= BATTERY_LEVELS[0] => 1,
        _ => 0,
    }
}

impl App {
    /// The battery as the phone reads it. It is told, not read from here:
    /// the pages have no way to ask the kernel, and asking each frame would
    /// cost a read for nothing.
    ///
    /// One card a step: a battery that goes from twenty to ten says the new
    /// number rather than the same card twice, and one a finger put away is
    /// not put back until it runs down to the next step.
    pub fn battery(&mut self, battery: Option<&skyward_info::Battery>) {
        let Some(b) = battery else { return };
        // The lock draws the charge on its foot line, so a reading that
        // changed is a lock drawn again: without this the line under the
        // clock would go on saying what it said when the screen came up,
        // through a cable going in or a charge running down.
        let now = (b.level, b.plugged);
        if self.battery_now != Some(now) {
            self.battery_now = Some(now);
            self.lock_sheet_gone();
            self.dirty = true;
        }
        // Charged back above every step: a run down from here is a new one,
        // and may say what it says again. Asked before the charger, because
        // a battery full on the cable is exactly the case the steps have to
        // be behind: unplugged at a hundred, it must be told again when it
        // comes back down.
        if b.level >= RECOVERED {
            self.battery_told = 0;
            self.battery_pending = None;
            self.dismiss_battery(self.now);
            return;
        }
        // A charger in the phone, whatever its level: nothing is said, what
        // waits to be said is dropped, and a card already up goes — the
        // phone is filling, not running down.
        if b.plugged {
            self.battery_pending = None;
            self.dismiss_battery(self.now);
            return;
        }
        if step(b.level) <= self.battery_told {
            return;
        }
        self.battery_pending = Some(b.level);
        self.say_battery(self.now);
    }

    /// Say what the battery has left to say, where the panel has room for
    /// it. What is not said now waits for the panel to clear rather than
    /// being lost: a card already up, the phone on the lock or asleep, and
    /// work under way whose progress the card would cover all hold it back.
    pub(super) fn say_battery(&mut self, now: u64) {
        let Some(level) = self.battery_pending else { return };
        if self.sheet.is_some()
            || self.power_menu.is_some()
            || self.power_exit.is_some()
            || self.lock.is_some()
            || !self.display_on
            || matches!(self.screen(), Screen::Installing | Screen::BackupJob)
        {
            return;
        }
        self.battery_told = step(level);
        self.battery_pending = None;
        let body = words(self.state.lang, level);
        self.show_sheet(LOW_BATTERY, body, vec![vec![(OK, Action::SheetCancel)]], now);
    }

    /// Put the battery's card away, if it is the one standing: the charger
    /// went in, or the battery came back up. A card of another kind is left
    /// alone — whatever put it still has something to say.
    fn dismiss_battery(&mut self, now: u64) {
        let standing = self.sheet.as_ref().is_some_and(|s| s.title == LOW_BATTERY && s.closing.is_none());
        if standing {
            self.close_sheet(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skyward_info::{Battery, Charge};

    /// A phone on the desktop, its panel lit, its lock away and nothing said.
    fn phone() -> App {
        App::new(Screen::Desktop, 400, 800, 2.0)
    }

    /// A battery at `level`, the cable out or in.
    fn at(level: u8, plugged: bool) -> Battery {
        Battery {
            level,
            charge: if plugged { Charge::Charging } else { Charge::Discharging },
            temp: Some(300),
            health: None,
            cycles: None,
            design: None,
            full: None,
            voltage: None,
            quick: false,
            plugged,
        }
    }

    /// The card standing, its words and its one key.
    fn card(app: &App) -> (Text, String, usize) {
        let sheet = app.sheet.as_ref().expect("a card stands");
        (sheet.title, sheet.body.clone(), sheet.rows.first().map_or(0, Vec::len))
    }

    /// Take the card away as a finger on its key does, and let it fall.
    fn put_away(app: &mut App) {
        app.act(Action::SheetCancel, 0);
        app.sheet = None;
    }

    /// Twenty per cent is where the first card comes, in the words it is
    /// given, with one key to put it away.
    #[test]
    fn a_battery_at_twenty_says_so_once() {
        let mut app = phone();
        app.battery(Some(&at(25, false)));
        assert!(app.sheet.is_none(), "a battery with charge to spare says nothing");

        app.battery(Some(&at(20, false)));
        let (title, body, keys) = card(&app);
        assert_eq!(title, LOW_BATTERY, "the card's own heading");
        assert_eq!(body, "电量仅剩余 20%，请及时充电。");
        assert_eq!(keys, 1, "one key, which only puts it away");
    }

    /// A battery at the level itself has not run lower: it is said once,
    /// whatever the gauge says frame after frame.
    #[test]
    fn a_battery_sitting_at_a_level_is_said_once() {
        let mut app = phone();
        app.battery(Some(&at(20, false)));
        put_away(&mut app);
        app.battery(Some(&at(19, false)));
        app.battery(Some(&at(21, false)));
        app.battery(Some(&at(20, false)));
        assert!(app.sheet.is_none(), "the same step is not said again");
    }

    /// Running down to ten says ten, not twenty again: the number on the
    /// card is what changed.
    #[test]
    fn the_next_step_says_the_new_number() {
        let mut app = phone();
        app.battery(Some(&at(20, false)));
        put_away(&mut app);
        app.battery(Some(&at(10, false)));
        let (_, body, _) = card(&app);
        assert_eq!(body, "电量仅剩余 10%，请及时充电。");
    }

    /// Three per cent is the last word: the phone says it and then there is
    /// nothing left worth saying.
    #[test]
    fn a_phone_started_nearly_empty_says_three_at_once() {
        let mut app = phone();
        app.battery(Some(&at(3, false)));
        let (_, body, _) = card(&app);
        assert_eq!(body, "电量仅剩余 3%，请及时充电。");
        assert_eq!(app.battery_told, 3, "the last step is behind it");
    }

    /// A charger in the phone takes the card away and says nothing more,
    /// however low the battery is: charging past twenty must not read as
    /// running low.
    #[test]
    fn a_charger_says_nothing_and_takes_the_card_away() {
        let mut app = phone();
        app.battery(Some(&at(19, false)));
        assert!(app.sheet.is_some());
        app.battery(Some(&at(19, true)));
        assert!(app.sheet.as_ref().is_some_and(|s| s.closing.is_some()), "the card is falling");

        let mut app = phone();
        app.battery(Some(&at(20, true)));
        assert!(app.sheet.is_none(), "nothing said while it charges");
        app.battery(Some(&at(10, true)));
        app.battery(Some(&at(3, true)));
        assert!(app.sheet.is_none(), "no step is said at all on the cable");
    }

    /// The step said on the cable stays said: unplugging at eighteen — or
    /// charging to twenty and stopping there — does not say it again.
    #[test]
    fn what_was_said_before_the_charger_is_not_said_again() {
        let mut app = phone();
        app.battery(Some(&at(19, false)));
        put_away(&mut app);
        app.battery(Some(&at(19, true)));
        app.battery(Some(&at(19, false)));
        assert!(app.sheet.is_none(), "the step was said once already");
    }

    /// Charged back above every step, a run down is a new one and is said
    /// over again.
    #[test]
    fn a_charged_battery_may_run_down_and_say_so_again() {
        let mut app = phone();
        app.battery(Some(&at(19, false)));
        put_away(&mut app);
        app.battery(Some(&at(30, false)));
        assert_eq!(app.battery_told, 0, "charged back up: the steps are ahead again");
        app.battery(Some(&at(15, false)));
        let (_, body, _) = card(&app);
        assert_eq!(body, "电量仅剩余 15%，请及时充电。");
    }

    /// A battery charged full on the cable is behind its steps, even though
    /// the steps were said before it: unplugged at a hundred, it says so
    /// again when it comes back down.
    #[test]
    fn a_battery_full_on_the_cable_is_told_again_when_it_runs_down() {
        let mut app = phone();
        app.battery(Some(&at(19, false)));
        put_away(&mut app);
        app.battery(Some(&at(19, true)));
        app.battery(Some(&at(100, true)));
        assert_eq!(app.battery_told, 0, "full: the steps are ahead again");
        app.battery(Some(&at(19, false)));
        let (_, body, _) = card(&app);
        assert_eq!(body, "电量仅剩余 19%，请及时充电。");
    }

    /// A card already on the panel is not covered by this one: the battery
    /// waits, and says it as soon as the panel is clear.
    #[test]
    fn a_busy_panel_holds_the_card_until_it_clears() {
        let mut app = phone();
        app.show_sheet(OK, "正在安装".to_string(), vec![vec![(OK, Action::SheetCancel)]], 0);
        app.battery(Some(&at(8, false)));
        assert_eq!(app.sheet.as_ref().unwrap().body, "正在安装", "the other card stands");

        app.sheet = None;
        app.tick(100);
        let (_, body, _) = card(&app);
        assert_eq!(body, "电量仅剩余 8%，请及时充电。", "said once the panel cleared");
    }

    /// The phone on the lock, or asleep, is not spoken over: the battery
    /// waits for the page to come back.
    #[test]
    fn a_locked_or_dark_phone_is_told_when_it_wakes() {
        let mut app = phone();
        app.display_on = false;
        app.battery(Some(&at(8, false)));
        assert!(app.sheet.is_none(), "nothing over a dark panel");
        app.display_on = true;
        app.tick(100);
        assert!(app.sheet.is_some(), "said as soon as it can be read");
    }
}
