//! The framebuffer the menu draws into.
//!
//! Nothing here knows about the UI crate: a screen is RGBA rows and a glyph
//! is sixteen rows out of the table in [`font`]. That is the whole reason the
//! menu can be its own program later without dragging a renderer along.

use super::font;

/// How far one pixel is blown up.
///
/// Whole numbers only, nearest neighbour only. A fractional scale would put
/// grey edges on stems that are meant to be one pixel wide, and the panel
/// would stop looking like text on a boot screen and start looking like a
/// bad photo of it.
///
/// Three, which is what the widest line the menu prints — the one naming the
/// keys — needs to stay on this panel: thirty-seven half-width characters at
/// four would be 1184 pixels of glyph plus the gaps, and the Chinese lines
/// are worse, since each character is twice as wide.
pub const SCALE: u32 = 3;

/// One half-width character on screen, its gap included.
///
/// The drawer walks a line by each glyph's own width, so nothing at runtime
/// needs this; it is the unit the layout is reasoned about in, and the tests
/// measure against it.
#[allow(dead_code)]
pub const CELL_W: u32 = font::NARROW_W * SCALE;
/// One character cell on screen, gap included. Chinese is two of these wide.
///
/// This is the glyph, not the line: a reversed band is exactly this tall so
/// it hugs the words on it.
pub const CELL_H: u32 = font::H * SCALE;

/// Blank rows left between lines, before scaling.
///
/// Unifont's Chinese glyphs fill all sixteen rows — a 16x16 cell has no room
/// for the margins a Latin glyph has — so lines printed at the cell height
/// touch, and a page of Chinese turns into one grey block. Four rows of air
/// is what a 16-pixel font needs to read as separate lines at any scale.
pub const LEADING: u32 = 4;

/// From the top of one line to the top of the next.
pub const LINE_H: u32 = CELL_H + LEADING * SCALE;

/// Opaque white: every line of the menu is printed in it, the way the ABL
/// prints its console.
pub const LIT: [u8; 3] = [0xff, 0xff, 0xff];
/// What sits on the white: the line the keys point at, reversed out.
pub const INK: [u8; 3] = [0x00, 0x00, 0x00];
/// Yellow: the one shade the panel allows itself beyond black and white, for
/// a hint that says something is not known rather than something that has
/// been checked.
pub const YELLOW: [u8; 3] = [0xff, 0xff, 0x00];

/// A panel-sized buffer of tightly packed RGBA rows.
pub struct Screen {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

impl Screen {
    /// A black screen of `w` by `h`.
    pub fn new(w: u32, h: u32) -> Self {
        let mut screen = Screen { w, h, data: vec![0; (w as usize) * (h as usize) * 4] };
        screen.fill([0, 0, 0]);
        screen
    }

    /// Paint the whole screen one colour.
    pub fn fill(&mut self, rgb: [u8; 3]) {
        let (pixels, rest) = self.data.as_chunks_mut::<4>();
        debug_assert!(rest.is_empty(), "the buffer is always a whole number of pixels");
        for px in pixels {
            px[0] = rgb[0];
            px[1] = rgb[1];
            px[2] = rgb[2];
            // Alpha stays opaque: this buffer goes to the scanout as-is, so
            // a zero here is not "let the layer below show", it is a hole.
            px[3] = 0xff;
        }
    }

    /// How wide `text` is on screen, in pixels.
    ///
    /// A sum over the characters, not the character count times a cell: the
    /// two languages on this panel are two different widths, and a reversed
    /// band that guessed would either stop short of the last word or run
    /// past it. Nothing measures a line yet, so it stands ready rather than
    /// in use.
    #[allow(dead_code)]
    pub fn text_w(text: &str) -> u32 {
        text.chars().map(|c| font::width(c) * SCALE).sum()
    }

    /// Draw `text` with its top-left cell corner at (`x`, `y`).
    pub fn draw(&mut self, x: i32, y: i32, text: &str, rgb: [u8; 3]) {
        let mut pen = x;
        for c in text.chars() {
            // A character the font does not have is drawn as `?` rather than
            // skipped: the line keeps its length, and what the reader sees is
            // "this glyph is missing" instead of "the text is misspelt".
            let Some(glyph) = font::glyph(c).or_else(|| font::glyph('?')) else { break };
            for row in 0..font::H as usize {
                let bits = glyph.row(row);
                for col in 0..glyph.width {
                    // Bit 15 (or bit 7, half width) is the leftmost pixel.
                    if bits & (1 << (glyph.width - 1 - col)) != 0 {
                        self.fill_rect(pen + (col * SCALE) as i32, y + (row as u32 * SCALE) as i32, SCALE, SCALE, rgb);
                    }
                }
            }
            pen += (glyph.width * SCALE) as i32;
        }
    }

    /// Draw `text` hard against the left edge, its top edge at `y`.
    ///
    /// No indent at all: this is a console, and the bootloader's own starts
    /// in column zero. The one thing that moves right is nothing — the
    /// cursor is a white band behind the line instead of a mark beside it.
    pub fn draw_left(&mut self, y: i32, text: &str, rgb: [u8; 3]) {
        self.draw(0, y, text, rgb);
    }

    /// Paint a rectangle, clipped at the edges of the panel.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, rgb: [u8; 3]) {
        let from_x = x.max(0) as u32;
        let to_x = (x + w as i32).clamp(0, self.w as i32) as u32;
        let from_y = y.max(0) as u32;
        let to_y = (y + h as i32).clamp(0, self.h as i32) as u32;
        for py in from_y..to_y {
            for px in from_x..to_x {
                let at = ((py * self.w + px) * 4) as usize;
                self.data[at] = rgb[0];
                self.data[at + 1] = rgb[1];
                self.data[at + 2] = rgb[2];
                self.data[at + 3] = 0xff;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(screen: &Screen, x: u32, y: u32) -> bool {
        screen.data[((y * screen.w + x) * 4) as usize] != 0
    }

    #[test]
    fn a_new_screen_is_black_and_opaque() {
        let screen = Screen::new(CELL_W, CELL_H);
        assert!(screen.data.as_chunks::<4>().0.iter().all(|px| *px == [0, 0, 0, 0xff]));
    }

    #[test]
    fn a_glyph_is_drawn_as_whole_blocks() {
        // Nearest neighbour, three times up: every 3x3 block of the screen is
        // either entirely lit or entirely dark. A grey edge anywhere means
        // the scaling went fractional.
        let mut screen = Screen::new(CELL_W, CELL_H);
        screen.draw(0, 0, "A", LIT);
        assert!((0..CELL_W).any(|x| (0..CELL_H).any(|y| lit(&screen, x, y))), "something is drawn");
        for by in 0..CELL_H / SCALE {
            for bx in 0..CELL_W / SCALE {
                let corner = lit(&screen, bx * SCALE, by * SCALE);
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        assert_eq!(lit(&screen, bx * SCALE + dx, by * SCALE + dy), corner, "block {bx},{by} is not uniform");
                    }
                }
            }
        }
    }

    #[test]
    fn a_space_draws_nothing() {
        let mut screen = Screen::new(CELL_W, CELL_H);
        screen.draw(0, 0, " ", LIT);
        assert!(screen.data.as_chunks::<4>().0.iter().all(|px| *px == [0, 0, 0, 0xff]));
    }

    #[test]
    fn chinese_takes_two_cells_and_starts_where_the_latin_stopped() {
        let mut screen = Screen::new(CELL_W * 2, CELL_H);
        screen.draw(0, 0, "A启", LIT);
        // One half-width character, so the Chinese one starts at CELL_W and
        // runs past the right edge of this buffer: what is on the screen is
        // the A and the left half of 启, nothing else.
        assert!((0..CELL_W).any(|x| (0..CELL_H).any(|y| lit(&screen, x, y))), "the A is drawn");
        assert!(Screen::text_w("A启") == CELL_W + CELL_W * 2, "measured as one and a half cells");
    }

    #[test]
    fn text_past_the_edge_is_clipped_not_wrapped() {
        let mut screen = Screen::new(CELL_W, CELL_H);
        screen.draw(0, 0, "AA", LIT);
        // The second cell begins at CELL_W, off the right edge, so only the
        // first 'A' is drawn and nothing wraps onto another row.
        assert!(lit(&screen, 3 * SCALE, 3 * SCALE) || (0..CELL_W).any(|x| (0..CELL_H).any(|y| lit(&screen, x, y))));
        assert!(!(0..CELL_W).any(|x| lit(&screen, x, CELL_H - 1)), "no wrap");
    }

    #[test]
    fn a_line_starts_at_the_left_edge() {
        let mut screen = Screen::new(200, 100);
        screen.draw_left(0, "M", LIT);
        // Column zero is the first cell: a console prints from the edge, and
        // the cursor is drawn behind the text rather than beside it.
        assert!((0..CELL_W).any(|x| (0..CELL_H).any(|y| lit(&screen, x, y))), "the first cell is at the edge");
    }

    #[test]
    fn a_band_behind_a_line_covers_it_and_stops() {
        let mut screen = Screen::new(400, CELL_H);
        let band = Screen::text_w("Hi");
        // White behind, black in front: what the cursor does to its line.
        screen.fill_rect(0, 0, band, CELL_H, LIT);
        screen.draw_left(0, "Hi", INK);
        assert_eq!(screen.data[..4], [0xff; 4], "the band starts at the edge");
        assert_eq!(screen.data[((band - 1) * 4) as usize], 0xff, "and runs the width of the text");
        assert_eq!(screen.data[(band * 4) as usize], 0x00, "past it the panel is still black");
        // The letters are knocked out of the band rather than drawn beside it.
        assert!(
            (0..CELL_H).any(|y| (0..band).any(|x| screen.data[((y * 400 + x) * 4) as usize] == 0x00)),
            "the letters are knocked out"
        );
    }
}
