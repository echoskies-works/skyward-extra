//! The menu that comes up before Aether does.
//!
//! It exists because fastboot cannot read Aether's own idea of the phone:
//! which copy boots, what state the last update left it in, how to put it
//! back. Only Aether's code knows that, so there has to be a small piece of
//! Aether that runs when nothing else does, and it has to be small enough to
//! still run when the rest of the system is the thing that broke.
//!
//! So: three lines, two keys, one typeface taken from the bootloader, and a
//! dependency list short enough to read in one go. Nothing here touches the
//! UI crate, the touch panel, the network or the installer.

// A host build has no panel to draw on, so the entry point and the parts it
// alone reaches are compiled out. What is left is still worth building and
// testing — that is where the layout and the key handling get checked — so
// the leftovers are allowed to look unused rather than be hidden behind
// attributes on every item.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use anyhow::Result;

mod bootstate;
mod crashlog;
mod font;
mod lang;
mod liveness;
mod menu;
mod paint;
mod screen;
mod verdict;

#[cfg(target_os = "android")]
mod backlight;
#[cfg(target_os = "android")]
mod keys;
#[cfg(target_os = "android")]
mod watch;

pub use lang::Lang;
pub use menu::{Action, Crash, Reason, View};

/// What the caller was asking of this program.
///
/// init starts the service as `recovery` with no arguments, and a shell gets
/// at the menu by making a link called `startup`. Neither name is a choice
/// the person at the phone made, so `--menu` is there as well: it says what is
/// wanted outright, whatever the binary happens to be called. The word wins
/// over the name, because the name was fixed when the image was built and the
/// word was said a moment ago.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Job {
    /// Start the system and own the panel while it is down.
    Watch,
    /// Put the menu up on its own.
    Menu,
}

/// Read the job off the name the program was started under and the words
/// after it.
///
/// Only `--menu` is understood. Anything else is refused rather than skipped:
/// a name or a word nobody meant is a menu that does not come up, and one that
/// comes up anyway with a typo swallowed is worse than one that says so on the
/// kernel log and stays down.
pub fn job_of(name: &str, args: &[String]) -> Result<Job> {
    let mut menu = false;
    for arg in args {
        match arg.as_str() {
            "--menu" => menu = true,
            other => anyhow::bail!("asmoday: unknown argument {other}"),
        }
    }
    if menu {
        return Ok(Job::Menu);
    }
    match name {
        "recovery" => Ok(Job::Watch),
        "startup" => Ok(Job::Menu),
        other => anyhow::bail!("asmoday: unknown command {other}"),
    }
}

/// Start the system, watch it, and own the panel while it is down.
///
/// This is the recovery service init starts. It lives here, next to the menu,
/// because the two are the same job seen from two sides: while a system runs
/// the parent only waits, and the moment it stops the parent is the menu.
#[cfg(target_os = "android")]
pub fn watch() -> Result<()> {
    watch::run()
}

/// Draw `view` into a fresh RGBA buffer of `w` by `h`.
///
/// The one thing in this crate that is not for the phone: it lets a desk
/// build put the menu on a screen and look at it, which is how the glyphs
/// get compared against the bootloader's before anything is flashed.
pub fn render(w: u32, h: u32, view: View, lang: Lang) -> Vec<u8> {
    let mut screen = screen::Screen::new(w, h);
    paint::draw(&mut screen, view, lang);
    screen.data
}

/// What the caller still has to do once the menu is done.
///
/// The menu answers what it can on its own — a line it cannot act on says so
/// on the panel and goes back to the list — so this only ever means "carry
/// on starting up" or "the phone can go dark".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Choice {
    /// Go on and bring the system up.
    System,
    /// Turn the phone off.
    PowerOff,
    /// Hand the phone to the bootloader's own fastboot, so it comes back up
    /// waiting for an image over USB.
    Fastboot,
}

/// Where the scanout shows up on every phone this runs on.
#[cfg(target_os = "android")]
const CARD: &str = "/dev/dri/card0";

/// How long to sit between looks at /dev/input while waiting for a press.
#[cfg(target_os = "android")]
const IDLE: std::time::Duration = std::time::Duration::from_millis(250);

/// The panel, the buttons and the buffer: everything this crate draws with,
/// so the menu and a failure screen go up the same way.
#[cfg(target_os = "android")]
struct Panel {
    surface: skyward_kms::KmsSurface,
    screen: screen::Screen,
    buttons: keys::Buttons,
}

#[cfg(target_os = "android")]
impl Panel {
    fn open() -> Result<Self> {
        use anyhow::Context;

        // The rules that give /dev its usual owners. Without this the nodes
        // are whatever the kernel left, and a menu running as shell cannot
        // open the card. A missing rule file is not fatal: the panel often
        // works anyway.
        if let Err(e) = skyward_sys::ueventd::apply() {
            eprintln!("startup: {e:#}");
        }

        let surface = skyward_kms::KmsSurface::open(CARD).context("open the display")?;
        let (w, h) = surface.size();
        eprintln!("startup: display {w}x{h}");

        let buttons = keys::Buttons::new();
        if !buttons.found() {
            eprintln!("startup: no buttons yet, still watching /dev/input");
        }
        Ok(Panel { surface, screen: screen::Screen::new(w, h), buttons })
    }

    /// `view` in full, in one frame.
    fn present(&mut self, view: View, lang: Lang) -> Result<()> {
        use anyhow::Context;
        paint::draw(&mut self.screen, view, lang);
        let (w, h) = (self.screen.w, self.screen.h);
        self.surface.present_frame(&self.screen.data, w, h).context("draw the panel")
    }

    /// Say `text` where the menu was, wait for one press, and let the caller
    /// put the menu back.
    ///
    /// For the lines that have nothing behind them yet: a press that changes
    /// nothing has to say so, or the panel looks hung.
    fn say(&mut self, text: &str) -> Result<()> {
        use anyhow::Context;
        paint::notice(&mut self.screen, text);
        let (w, h) = (self.screen.w, self.screen.h);
        self.surface.present_frame(&self.screen.data, w, h).context("draw the notice")?;
        self.buttons.wait(IDLE);
        Ok(())
    }

    /// Wait for the power button, and nothing else: the screen it is pressed
    /// on offers one key and one way on.
    fn wait_power(&mut self) -> menu::Key {
        loop {
            for key in self.buttons.wait(IDLE) {
                if key == menu::Key::Power {
                    return key;
                }
            }
            // A panel that came up before its input devices did would
            // otherwise leave the screen stuck with no way to answer it.
            self.buttons.scan();
        }
    }
}

/// What a view is, as far as the printing goes: two views of the same kind
/// are the same screen with the cursor moved, which is redrawn at once
/// rather than printed again.
#[cfg(target_os = "android")]
fn kind(view: View) -> u8 {
    match view {
        View::List { .. } => 0,
        View::Confirm { .. } => 1,
        View::Failure { .. } => 2,
    }
}

/// Put the menu on the panel and wait for a decision.
///
/// The menu never wipes anything itself: what "reset" means is decided
/// elsewhere, by the code that already knows how to erase a phone safely.
#[cfg(target_os = "android")]
pub fn run() -> Result<Choice> {
    use menu::Step;

    let lang = Lang::chosen();
    let mut panel = Panel::open()?;
    let mut menu = menu::Menu::new();
    panel.present(menu.view(), lang)?;

    loop {
        for key in panel.buttons.wait(IDLE) {
            let before = menu.view();
            match menu.key(key) {
                Step::Still => {}
                Step::Redraw => {
                    let after = menu.view();
                    if kind(after) == kind(before) {
                        panel.present(after, lang)?;
                    } else {
                        // A question is a new screen, not the list with a
                        // band moved, so draw the whole of it at once.
                        panel.present(after, lang)?;
                    }
                }
                Step::Done(Action::Continue) => return Ok(Choice::System),
                Step::Done(Action::PowerOff) => return Ok(Choice::PowerOff),
                Step::Done(Action::Fastboot) => return Ok(Choice::Fastboot),
                Step::Done(Action::Reset) => {
                    // Erasing means driving the same code the UI's factory
                    // reset drives, which reports every step to a screen and
                    // expects somewhere to put a progress bar. This menu has
                    // no room for one, so saying "not yet" where the user can
                    // read it beats a line that looks like it worked.
                    panel.say(lang.pick(["RESET IS NOT WIRED UP YET", "重置还没有接上"]))?;
                    panel.present(menu.view(), lang)?;
                }
                Step::Done(Action::Troubleshoot) => {
                    // The sub-menu behind this is the deep end — reinstall,
                    // roll back, repair filesystems — and none of it exists
                    // yet. Same rule as reset: the panel says so.
                    panel.say(lang.pick(["TROUBLESHOOT IS NOT WIRED UP YET", "疑难解答还没有接上"]))?;
                    panel.present(menu.view(), lang)?;
                }
            }
        }
        panel.buttons.scan();
    }
}

/// Put a failure on the panel, and wait for the key that leaves it.
///
/// This is the one screen the panel shows that is not about the panel: it is
/// what is left of a system that stopped, so it says which of the two ways it
/// stopped (never came up, or was up and then was not), the code it left, and
/// — where it stopped on purpose — which of its own apps it stopped for.
#[cfg(target_os = "android")]
pub fn show_crash(crash: Crash, code: i32, reason: Option<Reason>) -> Result<()> {
    let lang = Lang::chosen();
    let mut panel = Panel::open()?;
    let view = View::Failure { crash, code, reason };
    panel.present(view, lang)?;
    panel.wait_power();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every language the panel speaks, for the tests that have to hold in
    /// both. Chinese is the wider one and the one with more room to be wrong.
    const BOTH: [Lang; 2] = [Lang::En, Lang::Zh];

    /// The longest line has to fit the panel, or the menu runs off the edge
    /// and the whole point of a small font is lost.
    ///
    /// Measured in pixels rather than characters: a Chinese character is two
    /// cells wide, so a character count would pass a line that does not fit.
    #[test]
    fn the_longest_line_fits_the_panel() {
        // 1200 is what the phone this runs on reports. The menu does not
        // shrink itself for every panel in the tree: one size, the one the
        // machine it goes on has.
        let mut lines: Vec<String> = Vec::new();
        for lang in BOTH {
            lines.push(lang.pick(paint::TITLE).to_string());
            lines.extend(paint::HINT.iter().map(|line| lang.pick(*line).to_string()));
            lines.extend(paint::hints(lang).into_iter().map(|hint| hint.text));
            lines.extend(paint::EXIT.iter().map(|line| lang.pick(*line).to_string()));
            lines.extend(menu::ITEMS.iter().map(|item| item.label(lang).to_string()));
            // The code is the only part of a line that is not fixed, so the
            // widest code there can be goes in the check too.
            for crash in [Crash::Startup, Crash::Runtime] {
                for code in [0, 139, i32::MIN] {
                    lines.extend(paint::failure(crash, code, None, lang).into_iter().map(|line| line.text().to_string()));
                }
            }
        }
        let widest = lines.iter().map(|line| screen::Screen::text_w(line)).max().unwrap_or(0);
        assert!(widest < 1200, "the longest line is {widest} px wide");
    }

    /// Every line, at the scale drawn, has to fit between the notch and the
    /// bottom edge on the shortest panel.
    #[test]
    fn every_view_fits_a_short_panel_below_the_notch() {
        let (w, h) = (1080u32, 1920u32);
        let views = [
            menu::View::List { sel: 0 },
            menu::View::Confirm { action: Action::Reset, yes: false },
            menu::View::Failure { crash: Crash::Startup, code: 139, reason: None },
            menu::View::Failure { crash: Crash::Runtime, code: 139, reason: Some(Reason::Music) },
        ];
        for lang in BOTH {
            for view in views {
                let mut screen = screen::Screen::new(w, h);
                paint::draw(&mut screen, view, lang);
                // Nothing is drawn in the top sixteenth: that is where the hole is.
                let band = h / 16;
                let lit_in_band = (0..band).any(|y| (0..w).any(|x| screen.data[((y * w + x) * 4) as usize] != 0));
                assert!(!lit_in_band, "{lang:?} {view:?} starts below the camera hole");

                let lit_in_bottom = (h * 7 / 8..h).any(|y| (0..w).any(|x| screen.data[((y * w + x) * 4) as usize] != 0));
                assert!(!lit_in_bottom, "{lang:?} {view:?} ends above the bottom edge");
            }
        }
    }

    /// The menu lines are the whole vocabulary of the menu. If one appears or
    /// goes it has to be deliberate, not a side effect.
    #[test]
    fn the_menu_has_five_lines() {
        assert_eq!(menu::ITEMS.len(), 5);
    }

    /// The ways in: the name init or a link gives, and the word a shell says
    /// outright. The word wins over the name, and a word that is nearly right
    /// is refused rather than skipped.
    #[test]
    fn the_job_is_read_from_the_name_and_the_words() {
        let nothing: Vec<String> = Vec::new();
        let menu = vec!["--menu".to_string()];
        assert_eq!(job_of("recovery", &nothing).expect("what init starts"), Job::Watch);
        assert_eq!(job_of("recovery", &menu).expect("a shell asking for it"), Job::Menu);
        assert_eq!(job_of("startup", &nothing).expect("the link"), Job::Menu);
        assert_eq!(job_of("startup", &menu).expect("either way"), Job::Menu);
        assert!(job_of("recovery", &["--menus".to_string()]).is_err(), "a typo is not a request");
        assert!(job_of("startup", &["menu".to_string()]).is_err(), "the dashes are part of the word");
        assert!(job_of("asmoday", &nothing).is_err(), "a name nobody meant starts nothing");
    }
}
