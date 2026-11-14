//! Turning a [`View`] into lines of text on a [`Screen`].
//!
//! The menu is a console, not a page: no widgets, no scrolling, no animation.
//! Every line is one cell grid printed in white from the left edge, and the
//! line the keys are on is reversed out — black on white — the way a boot
//! menu has always shown where the cursor is.
//!
//! Every string here is written twice, English and Chinese, side by side, so
//! that a line added to one language cannot be forgotten in the other.

use crate::lang::Lang;
use super::menu::{Action, Crash, View, ITEMS};
use super::screen::{Screen, CELL_H, INK, LINE_H, LIT, SCALE};

/// What the panel says at the top.
pub(crate) const TITLE: [&str; 2] = ["Skyward-Extra Startup Menu", "Skyward-Extra 启动菜单"];

/// What the two keys do, spelled out.
///
/// `V+` / `V-` would be shorter, but nothing on this panel is labelled V, and
/// a first boot should not be a guess. Chinese spells the buttons out for the
/// same reason: 音量上/下 is what the phone's own labels say.
pub(crate) const HINT: [[&str; 2]; 2] = [
    ["Use Volume Up/Down to move highlight,", "使用音量上/下以移动光标，"],
    ["Press Power Button to select it.", "按下电源按钮以选择。"],
];

/// The only way off a failure screen.
///
/// Two lines in English because the sentence is thirty-seven half-width
/// characters: at this scale it would run past the right edge of the panel
/// and cut the last word off. The Chinese version is short enough to fit on
/// one line but is split in the same place, so both languages read as one
/// sentence per line rather than as two layouts.
pub(crate) const EXIT: [[&str; 2]; 2] = [
    ["Press Power Button to restart", "按下电源按钮以重启"],
    ["and show Startup Menu.", "并显示启动菜单。"],
];

/// How a line is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ink {
    /// White on black.
    Plain,
    /// Black on a white band: the line the keys are pointing at.
    Reversed,
}

/// One line of the panel.
///
/// Two kinds because a rule is not text. Drawing it out of hyphens left a
/// dashed line wherever the glyphs carried their own side margins, and the
/// line could only ever be as long as the string was; a rule drawn as a
/// rectangle has no gaps in it and runs edge to edge.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Line {
    /// A row of words, printed in `ink`.
    Words(String, Ink),
    /// A solid bar across the panel.
    Rule,
}

impl Line {
    /// What the line says. A rule says nothing: it is a line, not a row.
    ///
    /// Only the tests read lines back — the drawer matches on the variant
    /// and never needs the words as a string — so on the phone this is dead
    /// weight, which the crate-level attribute does not cover for `target_os
    /// = "android"`, where it is compiled rather than stripped.
    #[allow(dead_code)]
    pub(crate) fn text(&self) -> &str {
        match self {
            Line::Words(text, _) => text,
            Line::Rule => "",
        }
    }

    /// A plain row, which is what almost every line on this panel is.
    fn plain(text: impl Into<String>) -> Self {
        Line::Words(text.into(), Ink::Plain)
    }
}

/// How tall the rule is: one pixel of the font, blown up like everything
/// else on the panel, so it weighs exactly what a stem in the words above it
/// weighs. A thicker bar would read as a divider between sections rather
/// than as the same rule the bootloader uses.
const RULE_H: u32 = SCALE;

/// How much a rule advances the pen: the line itself plus air either side, so
/// it sits tight against the words it separates instead of floating a full
/// row away from them — but not so tight that a rule over a reversed line
/// reads as a ragged edge on the band it is touching.
const RULE_LINE_H: u32 = RULE_H + 5 * SCALE;

/// How far into its row the rule is drawn. Two pixels of air above it and a
/// little more below, which makes the rule read as the heading's underline
/// rather than as the first item's overline.
const RULE_DY: u32 = 2 * SCALE;

/// Paint `view` over the whole screen.
pub fn draw(screen: &mut Screen, view: View, lang: Lang) {
    screen.fill([0, 0, 0]);
    let lines = match view {
        View::List { sel } => list(sel, lang),
        View::Confirm { action, yes } => confirm(action, yes, lang),
        View::Failure { crash, code } => failure(crash, code, lang),
    };
    // Ranged from the top left, not centred. A console starts where a page
    // starts; centring the block would also move every line when the question
    // comes up, since it is a line shorter than the list.
    let mut y = top(screen.h);
    for line in &lines {
        match line {
            Line::Words(text, Ink::Plain) => screen.draw_left(y, text, LIT),
            Line::Words(text, Ink::Reversed) => {
                // The band runs the whole width of the panel, edge to edge,
                // the way a BIOS menu shows where the cursor is; the words on
                // it are knocked out of it rather than drawn beside it.
                screen.fill_rect(0, y, screen.w, CELL_H, LIT);
                screen.draw_left(y, text, INK);
            }
            Line::Rule => screen.fill_rect(0, y + RULE_DY as i32, screen.w, RULE_H, LIT),
        }
        y += match line {
            Line::Rule => RULE_LINE_H,
            _ => LINE_H,
        } as i32;
    }
}

/// Where the first line starts: below the camera hole, and no further down.
///
/// A sixteenth of the panel clears the hole on the phones this runs on — the
/// hole sits within the top tenth — and leaves the block reading as "the top
/// of the page" rather than as something floating in the middle.
fn top(h: u32) -> i32 {
    (h / 16).max(CELL_H) as i32
}

/// Replace the menu with one line.
///
/// For a choice the menu cannot carry out: the panel has to say something,
/// or a press that changes nothing looks like a hang.
pub fn notice(screen: &mut Screen, text: &str) {
    screen.fill([0, 0, 0]);
    // The same first line as the list, so the notice takes the title's place
    // instead of appearing somewhere else on the panel.
    screen.draw_left(top(screen.h), text, LIT);
}

/// The list: title, rule, the lines, then the rule and the key legend.
fn list(sel: usize, lang: Lang) -> Vec<Line> {
    let mut lines = header(lang);
    for (i, item) in ITEMS.iter().enumerate() {
        let ink = if i == sel { Ink::Reversed } else { Ink::Plain };
        lines.push(Line::Words(item.label(lang).to_string(), ink));
    }
    lines.extend(footer(lang));
    lines
}

/// The question for a destructive item, on the row the first item was on.
fn confirm(action: Action, yes: bool, lang: Lang) -> Vec<Line> {
    let mut lines = header(lang);
    // Full-width question mark in Chinese: a half-width one at the end of a
    // Chinese clause reads as a leftover from the English line.
    lines.push(Line::plain(format!("{}{}", action.label(lang), lang.pick(["?", "？"]))));
    lines.push(Line::plain(""));
    // "No" first and reversed: the safe answer is the one under the cursor
    // when the question appears.
    for (answer, on) in [(lang.pick(["No", "否"]), !yes), (lang.pick(["Yes", "是"]), yes)] {
        let ink = if on { Ink::Reversed } else { Ink::Plain };
        lines.push(Line::Words(answer.to_string(), ink));
    }
    lines.extend(footer(lang));
    lines
}

/// What the panel says when Aether died.
///
/// Said the way a person would say it. `BOOT FAILED` is what a board prints
/// when it gives up, and an alarm in front of a phone that is already dark
/// tells the user nothing they cannot see; what they need is to know it is
/// not their fault and which button to press. The code stays because it is
/// the only fact here that narrows the cause down — 101 is Aether's own
/// panic, 134 a failed assert, 137 the system killing it for memory, 139 a
/// bad pointer — and one line is cheap.
///
/// No stack, no registers: two keys cannot scroll, and the panel holds about
/// twenty lines.
pub(crate) fn failure(crash: Crash, code: i32, lang: Lang) -> Vec<Line> {
    // The first line is the only thing the two screens say differently. A
    // phone that never came up and a phone that was in use are different
    // problems, and whoever is holding it already knows which one happened —
    // so name it, rather than print one generic sentence for both.
    let what = match crash {
        Crash::Startup => ["Skyward-Extra has failed to startup.", "Skyward-Extra 启动失败了。"],
        Crash::Runtime => ["Skyward-Extra stopped while running.", "Skyward-Extra 已停止运行。"],
    };
    let mut lines = vec![
        Line::plain(lang.pick(what)),
        // Two words and a number: no clause to read, and the one fact that
        // matters is not buried in prose. `Error code` rather than `Stop
        // code` because this layer is Aether's, not a blue screen's.
        Line::plain(format!("{} {code}", lang.pick(["Error code:", "错误代码:"]))),
        Line::plain(""),
    ];
    lines.extend(EXIT.iter().map(|line| Line::plain(lang.pick(*line))));
    lines
}

/// The title, a blank, and the rule under it.
///
/// The blank is wanted here: the title is the panel's own heading, and a
/// console puts a row of air between a heading and the rule that closes it.
/// The rule then sits tight against the first item — that gap is the list's,
/// not the heading's.
fn header(lang: Lang) -> Vec<Line> {
    vec![Line::plain(lang.pick(TITLE)), Line::plain(""), Line::Rule]
}

/// A rule, a blank, and the key legend.
///
/// The blank is the mirror of the header's: the legend is the panel's
/// footing, not a sixth item, so it stands one row clear of the rule over it.
fn footer(lang: Lang) -> Vec<Line> {
    let mut lines = vec![Line::Rule, Line::plain("")];
    lines.extend(HINT.iter().map(|hint| Line::plain(lang.pick(*hint))));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font;
    use crate::screen::CELL_W;

    /// Which lines are reversed, by index.
    fn marked(lines: &[Line]) -> Vec<usize> {
        lines.iter().enumerate().filter(|(_, line)| matches!(line, Line::Words(_, Ink::Reversed))).map(|(i, _)| i).collect()
    }

    /// Both languages, because every one of these rules has to hold twice.
    const BOTH: [Lang; 2] = [Lang::En, Lang::Zh];

    #[test]
    fn both_views_start_with_the_same_header() {
        // The block is ranged from the top, so a view one line longer does
        // not move the title; only the header has to match. The header is the
        // title, a blank and its rule, nothing more: the first item follows
        // the rule at once.
        for lang in BOTH {
            assert_eq!(list(0, lang)[..3], confirm(Action::Reset, false, lang)[..3], "{lang:?}");
        }
    }

    #[test]
    fn the_header_is_the_title_a_blank_and_a_rule_under_it() {
        for lang in BOTH {
            let lines = list(0, lang);
            assert_eq!(lines[0].text(), lang.pick(TITLE));
            assert_eq!(lines[1].text(), "", "{lang:?}: air under the title");
            assert!(matches!(lines[2], Line::Rule), "{lang:?}: a rule, not words");
            // Both rules are the same line, at the same place: one under the
            // title's blank and one over the legend's blank.
            let rules: Vec<usize> = lines.iter().enumerate().filter(|(_, l)| matches!(l, Line::Rule)).map(|(i, _)| i).collect();
            assert_eq!(rules, vec![2, lines.len() - HINT.len() - 2], "{lang:?}");
        }
    }

    #[test]
    fn a_rule_is_a_solid_bar_across_the_panel() {
        // The whole point of the change: no dashes, no gaps, and no stopping
        // part way across because the string ran out of hyphens.
        let (w, h) = (1200u32, 2608u32);
        for lang in BOTH {
            let mut screen = Screen::new(w, h);
            draw(&mut screen, View::List { sel: 0 }, lang);
            let start = top(h) as u32;
            // Line 2 is the rule under the title: the title's cell, the blank
            // under it, then the air the rule keeps above itself.
            let y = start + 2 * LINE_H + RULE_DY;
            for dy in 0..RULE_H {
                for x in 0..w {
                    assert_eq!(
                        screen.data[(((y + dy) * w + x) * 4) as usize],
                        0xff,
                        "{lang:?}: ({x}, {}) is not lit",
                        y + dy
                    );
                }
            }
            // And it is a rule, not a block: the rows either side of it are
            // still black.
            assert_eq!(screen.data[((y - 1) * w * 4) as usize], 0x00, "{lang:?}: nothing above it");
            assert_eq!(screen.data[(((y + RULE_H) * w) * 4) as usize], 0x00, "{lang:?}: nothing below it");
        }
    }

    #[test]
    fn every_item_is_a_line_and_only_the_cursor_is_reversed() {
        for lang in BOTH {
            for sel in 0..ITEMS.len() {
                let lines = list(sel, lang);
                assert_eq!(marked(&lines), vec![sel + 3], "{lang:?}: the cursor is on item {sel}");
                for item in ITEMS {
                    assert!(
                        lines.iter().any(|line| line.text() == item.label(lang)),
                        "{lang:?}: {} is on the panel",
                        item.label(lang)
                    );
                }
            }
        }
    }

    #[test]
    fn the_question_starts_on_no() {
        for lang in BOTH {
            let lines = confirm(Action::Reset, false, lang);
            assert_eq!(lines[3].text(), format!("{}{}", Action::Reset.label(lang), lang.pick(["?", "？"])));
            assert_eq!(marked(&lines), vec![5], "{lang:?}: No is reversed");
            assert_eq!(lines[5].text(), lang.pick(["No", "否"]));
            assert_eq!(lines[6].text(), lang.pick(["Yes", "是"]));
        }
    }

    #[test]
    fn picking_yes_moves_the_band() {
        for lang in BOTH {
            let lines = confirm(Action::Reset, true, lang);
            assert_eq!(marked(&lines), vec![6], "{lang:?}: Yes is reversed");
            assert_eq!(lines[5].text(), lang.pick(["No", "否"]));
            assert_eq!(lines[6].text(), lang.pick(["Yes", "是"]));
        }
    }

    #[test]
    fn the_question_sits_where_the_first_item_was() {
        // Crossing into the question must not move the title, and the
        // question should read as part of the list rather than of the header.
        for lang in BOTH {
            assert_eq!(list(0, lang)[3].text(), Action::Continue.label(lang));
            assert_eq!(confirm(Action::Reset, false, lang)[3].text(), format!("{}{}", Action::Reset.label(lang), lang.pick(["?", "？"])));
        }
    }

    #[test]
    fn the_legend_spells_the_keys_out_and_ends_both_views() {
        for lang in BOTH {
            for lines in [list(0, lang), confirm(Action::Reset, false, lang)] {
                let n = lines.len();
                for (i, hint) in HINT.iter().enumerate() {
                    assert_eq!(lines[n - HINT.len() + i].text(), lang.pick(*hint), "{lang:?}: the legend ends the panel, in order");
                }
                assert!(HINT.iter().all(|h| !h.iter().any(|text| text.contains("V+") || text.contains("V-"))), "no abbreviations");
            }
        }
    }

    #[test]
    fn the_panel_only_ever_shows_black_and_white() {
        let (w, h) = (1200u32, 2608u32);
        for lang in BOTH {
            let mut screen = Screen::new(w, h);
            draw(&mut screen, View::List { sel: 1 }, lang);
            // No greys and no theme: one colour on black, with the cursor as
            // the same colour reversed rather than a dimmer shade of it.
            let pixels = screen.data.as_chunks::<4>().0;
            assert!(pixels.iter().all(|px| *px == [0, 0, 0, 0xff] || *px == [0xff; 4]), "{lang:?}: a pixel that is neither");
            assert!(pixels.contains(&[0xff; 4]), "{lang:?}: and something is lit");
        }
    }

    #[test]
    fn a_notice_replaces_the_whole_menu_with_one_line() {
        let (w, h) = (1080u32, 2400u32);
        let mut screen = Screen::new(w, h);
        draw(&mut screen, View::List { sel: 0 }, Lang::Zh);
        notice(&mut screen, "重置还没有接上");
        // One line of glyphs and nothing else: the list is gone, so a press
        // that could not be carried out does not look like a redraw.
        let rows: Vec<u32> = (0..h).filter(|&y| (0..w).any(|x| screen.data[((y * w + x) * 4) as usize] != 0)).collect();
        assert!(!rows.is_empty(), "the notice is drawn");
        assert!(*rows.last().unwrap() - rows[0] < LINE_H, "and it is a single line");
    }

    #[test]
    fn the_block_starts_under_the_camera_hole_and_near_the_top() {
        let (w, h) = (400u32, 800u32);
        for lang in BOTH {
            let mut screen = Screen::new(w, h);
            draw(&mut screen, View::List { sel: 0 }, lang);

            let lit_at = |y: u32| (0..w).any(|x| screen.data[((y * w + x) * 4) as usize] != 0);
            let rows: Vec<u32> = (0..h).filter(|&y| lit_at(y)).collect();
            let first = *rows.first().expect("something is drawn");
            assert!(first >= h / 16, "{lang:?}: clear of the camera hole");
            assert!(first < h / 5, "{lang:?}: and near the top rather than centred");
        }
    }

    #[test]
    fn every_line_starts_at_the_left_edge() {
        let (w, h) = (1200u32, 2608u32);
        for lang in BOTH {
            let mut screen = Screen::new(w, h);
            draw(&mut screen, View::List { sel: 0 }, lang);
            let start = top(h) as u32;
            let mut top_y = start;
            for (line, what) in list(0, lang).iter().enumerate() {
                let lit_at = |x: u32| (top_y..top_y + CELL_H).any(|y| screen.data[((y * w + x) * 4) as usize] != 0);
                if let Some(left) = (0..w).find(|&x| lit_at(x)) {
                    // Glyphs carry a column or two of their own margin, so this
                    // is "within the first cell", not "at pixel zero".
                    assert!(left < CELL_W, "{lang:?}: line {line} starts at column {left}, not against the edge");
                }
                top_y += match what {
                    Line::Rule => RULE_LINE_H,
                    _ => LINE_H,
                };
            }
        }
    }

    #[test]
    fn a_failure_says_what_happened_and_ends_with_the_way_out() {
        for lang in BOTH {
            for (crash, what) in [
                (Crash::Startup, ["Skyward-Extra has failed to startup.", "Skyward-Extra 启动失败了。"]),
                (Crash::Runtime, ["Skyward-Extra stopped while running.", "Skyward-Extra 已停止运行。"]),
            ] {
                let lines = failure(crash, 139, lang);
                assert_eq!(lines[0].text(), lang.pick(what), "{lang:?}: the screen names which case it was");
                assert_eq!(lines[1].text(), format!("{} 139", lang.pick(["Error code:", "错误代码:"])), "{lang:?}");
                let n = lines.len();
                for (i, exit) in EXIT.iter().enumerate() {
                    assert_eq!(lines[n - EXIT.len() + i].text(), lang.pick(*exit), "{lang:?}: the way out ends the screen, in order");
                }
            }
        }
    }

    #[test]
    fn the_two_failures_differ_only_in_the_first_line() {
        for lang in BOTH {
            let startup = failure(Crash::Startup, 139, lang);
            let runtime = failure(Crash::Runtime, 139, lang);
            assert_ne!(startup[0].text(), runtime[0].text(), "{lang:?}: which case it was is said");
            // Same stop code, same blank, same key: one layout with a different
            // verdict on top, not two designs.
            assert_eq!(&startup[1..], &runtime[1..], "{lang:?}");
        }
    }

    #[test]
    fn a_failure_is_not_the_menu_with_a_line_on_top() {
        for lang in BOTH {
            let lines = failure(Crash::Runtime, 101, lang);
            // Its own words, not the menu's: a failure screen that still offers
            // "Continue Startup" invites a press that goes nowhere.
            assert!(lines.iter().all(|line| line.text() != lang.pick(TITLE)), "{lang:?}: no title");
            for item in ITEMS {
                assert!(lines.iter().all(|line| line.text() != item.label(lang)), "{lang:?}: {} is not offered", item.label(lang));
            }
            assert!(marked(&lines).is_empty(), "{lang:?}: no cursor, there is nothing to move over");
        }
    }

    #[test]
    fn a_failure_is_five_lines_of_white_on_black() {
        let (w, h) = (1200u32, 2608u32);
        for lang in BOTH {
            for crash in [Crash::Startup, Crash::Runtime] {
                let mut screen = Screen::new(w, h);
                draw(&mut screen, View::Failure { crash, code: 139 }, lang);
                // What happened, the code, a blank, how to leave: five rows of
                // cells and nothing under them.
                let lines = failure(crash, 139, lang).len();
                assert_eq!(lines, 5);
                let below = top(h) as u32 + lines as u32 * LINE_H;
                let lit = |y: u32| (0..w).any(|x| screen.data[((y * w + x) * 4) as usize] != 0);
                assert!((0..h).any(lit), "{lang:?} {crash:?}: draws something");
                assert!(!(below..h).any(lit), "{lang:?} {crash:?}: stops after the last line");
                let pixels = screen.data.as_chunks::<4>().0;
                assert!(
                    pixels.iter().all(|px| *px == [0, 0, 0, 0xff] || *px == [0xff; 4]),
                    "{lang:?} {crash:?}: one colour, no grey"
                );
            }
        }
    }

    /// Every character either language can put on this panel has a glyph.
    ///
    /// The drawer falls back to `?` for a character it cannot find, which
    /// keeps the panel up but spells the line wrong — and a menu reading
    /// "重置 Skyward-Extra" as "?? Skyward-Extra" is worse than no menu. So
    /// the coverage is checked here, on the strings themselves, instead of
    /// discovered on a phone.
    #[test]
    fn every_character_the_panel_can_print_has_a_glyph() {
        for lang in BOTH {
            let mut strings: Vec<String> = vec![lang.pick(TITLE).to_string()];
            strings.extend(HINT.iter().map(|line| lang.pick(*line).to_string()));
            strings.extend(EXIT.iter().map(|line| lang.pick(*line).to_string()));
            strings.extend(ITEMS.iter().map(|item| item.label(lang).to_string()));
            strings.extend(ITEMS.iter().flat_map(|item| [format!("{}{}", item.label(lang), lang.pick(["?", "？"])), item.label(lang).to_string()]));
            strings.extend([lang.pick(["No", "否"]).to_string(), lang.pick(["Yes", "是"]).to_string()]);
            strings.extend([lang.pick(["Error code:", "错误代码:"]).to_string(), "139".to_string(), "-2147483648".to_string()]);
            for crash in [Crash::Startup, Crash::Runtime] {
                strings.extend(failure(crash, 139, lang).into_iter().map(|line| line.text().to_string()));
            }

            for text in strings {
                for c in text.chars() {
                    assert!(font::glyph(c).is_some(), "{lang:?}: no glyph for {c:?} (U+{:04X}) in {text:?}", c as u32);
                }
            }
        }
    }
}
