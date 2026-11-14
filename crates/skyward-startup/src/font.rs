//! The typeface: GNU Unifont, sixteen rows tall, one bit per pixel.
//!
//! Latin is half width (8 pixels) and Chinese full width (16), which is how
//! Unifont is drawn and how the panel has to measure a line. Sixteen rows
//! rather than the bootloader's nineteen: this face has to cover GB2312, and
//! EDK2's table stops at U+00FF with no CJK in it at all.
//!
//! The glyphs live in `font/glyphs.bin`, written by
//! `aether-project/tools/build-unifont.py` and pulled in with
//! `include_bytes!`. Not a Rust array: seven and a half thousand glyphs
//! written as literal numbers is minutes of compile time for nothing.
//!
//! Nothing here panics on a character it does not have. A missing glyph is a
//! real possibility — an emoji pasted into a label, a character outside
//! GB2312 — and a text renderer that aborts the whole program over one of
//! them would take the panel down with it.

use std::cmp::Ordering;

/// Rows of pixels in a glyph.
const ROWS: usize = 16;
/// Glyph height in pixels.
pub const H: u32 = ROWS as u32;
/// Width of the half-width characters, which is also what a character with
/// no glyph is assumed to take.
pub const NARROW_W: u32 = 8;

/// Bytes per entry: code point, width, then [`ROWS`] rows.
const STRIDE: usize = 2 + 2 + ROWS * 2;

/// Code point, width and rows for every character the menu can print.
static TABLE: &[u8] = include_bytes!("font/glyphs.bin");

/// One character, ready to draw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Glyph {
    rows: [u16; ROWS],
    /// Pixels across: 8 for Latin and punctuation, 16 for Chinese.
    pub width: u32,
}

impl Glyph {
    /// Row `row` from the top, most significant bit the leftmost pixel.
    pub fn row(&self, row: usize) -> u16 {
        self.rows[row]
    }
}

/// The glyph for `c`, or nothing when the font has no such character.
///
/// A binary search over the table: code points are stored ascending, and the
/// menu prints a few dozen characters per frame, so nothing cleverer is
/// worth the code.
pub fn glyph(c: char) -> Option<Glyph> {
    let code = u32::from(c);
    // The table only holds the basic plane, which is all GB2312 uses. A
    // character above it is not in here by construction, and casting it to a
    // u16 would silently look up the wrong one.
    if code > 0xFFFF {
        return None;
    }
    let key = code as u16;

    let (mut low, mut high) = (0usize, TABLE.len() / STRIDE);
    while low < high {
        let mid = (low + high) / 2;
        let at = mid * STRIDE;
        let found = u16::from_be_bytes([TABLE[at], TABLE[at + 1]]);
        match found.cmp(&key) {
            Ordering::Less => low = mid + 1,
            Ordering::Greater => high = mid,
            Ordering::Equal => {
                let width = u16::from_be_bytes([TABLE[at + 2], TABLE[at + 3]]) as u32;
                let mut rows = [0u16; ROWS];
                for (i, row) in rows.iter_mut().enumerate() {
                    let p = at + 4 + i * 2;
                    *row = u16::from_be_bytes([TABLE[p], TABLE[p + 1]]);
                }
                return Some(Glyph { rows, width });
            }
        }
    }
    None
}

/// How many pixels `c` takes up, whether or not the font has it.
///
/// Callers use this to keep the rest of a line where it belongs when one
/// character had to be drawn as something else.
pub fn width(c: char) -> u32 {
    glyph(c).map_or(NARROW_W, |glyph| glyph.width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_a_whole_number_of_rows() {
        assert_eq!(TABLE.len() % STRIDE, 0, "the file is the format the reader expects");
        // 7.4k glyphs: GB2312 in full plus ASCII. A much smaller file means
        // the generator picked up the wrong source.
        let count = TABLE.len() / STRIDE;
        assert!(count > 7000, "only {count} glyphs in the table");
    }

    #[test]
    fn latin_is_half_width_and_chinese_is_full_width() {
        assert_eq!(glyph('A').expect("A").width, 8);
        assert_eq!(glyph(' ').expect("space").width, 8);
        assert_eq!(glyph('启').expect("启").width, 16);
        // Full-width punctuation is what most of the Chinese sentences are
        // joined with, so it has to be wide as well.
        assert_eq!(glyph('，').expect("，").width, 16);
        assert_eq!(glyph('。').expect("。").width, 16);
    }

    #[test]
    fn a_glyph_has_something_to_draw() {
        // 'A' is a triangle on legs, so the top of the cell is blank and the
        // rows under it are not.
        let a = glyph('A').expect("A");
        assert!(a.row(0) == 0, "the first row of a capital is blank");
        assert!((0..ROWS).filter(|&r| a.row(r) != 0).count() > 5, "and the rest is not");
        // A full-width character uses bits above the eighth column.
        let wide = glyph('启').expect("启");
        assert!((0..ROWS).any(|r| wide.row(r) & 0xff00 != 0), "Chinese uses the right half");
    }

    #[test]
    fn characters_outside_the_table_are_reported_not_guessed() {
        // Above the basic plane: no entry, and the reader must not wrap the
        // code point into a u16 and find some unrelated glyph.
        assert!(glyph('\u{1F600}').is_none());
        // A control character was never in the set.
        assert!(glyph('\u{1}').is_none());
        // And the width still answers, so a line keeps its shape.
        assert_eq!(width('\u{1F600}'), NARROW_W);
    }

    #[test]
    fn the_length_of_a_line_is_the_sum_of_its_characters() {
        assert_eq!("AB".chars().map(width).sum::<u32>(), 16);
        assert_eq!("继续".chars().map(width).sum::<u32>(), 32);
        assert_eq!("A续".chars().map(width).sum::<u32>(), 24);
    }
}
