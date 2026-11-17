//! The player the desktop opens: which of its two faces the page shows. The
//! songs it will hold, the count it will run and the sound it will make are
//! not here yet — the page is, so that the way in is settled before there is
//! anything to play.

/// The two faces of the player: the songs as a list, and the one being
/// played with the controls that play it. The words across the top show one
/// at a time, and a finger dragged level across the page turns to the other,
/// as the clock's and the calculator's do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicFace {
    /// The songs, as a list.
    List,
    /// The song being played: its cover, its name and the controls.
    Play,
}
