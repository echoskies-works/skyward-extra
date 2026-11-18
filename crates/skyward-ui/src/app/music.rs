//! The player the desktop opens: which of its two faces the page shows, and
//! which shelf the list is looking at. The songs it will hold, the count it
//! will run and the sound it will make are not here yet — the page is, so
//! that the way in is settled before there is anything to play.

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

/// Which of the player's shelves the list looks at: every song at once, or
/// the three ways songs are kept — by the album they came from, by the
/// artist who made them, by the folder they were taken from.
///
/// The three shelves are named now and are not filled: the list says of each
/// that there is nothing on it yet, so the way into them is settled before
/// there is anything to put on them. A touch takes the page to the shelf it
/// names and leaves it there, as the two faces above do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicFilter {
    /// All the songs, in one list.
    All,
    /// The albums the songs came from.
    Album,
    /// The artists who made them.
    Artist,
    /// The folders they were taken from.
    Folder,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Action, App, Screen};

    const W: u32 = 400;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// Under the player's two faces stand four shelves, in the order the
    /// stock recorder names the kinds of recording it keeps. Each takes the
    /// list and leaves it there — the page and its face where they were —
    /// and the list says of each that there is nothing on it yet, since
    /// none of the three filled ones is read off the phone.
    ///
    /// The three that are not filled are named now rather than later, so
    /// that the way into them is settled before there is anything to put on
    /// them, and the shelf that is shown is the one that is filled.
    #[test]
    fn the_four_shelves_under_the_list_each_take_the_page() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Music, W, H, PPD);
        let acts = |app: &mut App| -> Vec<Action> {
            app.frame(&f, &i).hits.iter().map(|h| h.action).filter(|a| matches!(a, Action::MusicFilter(_))).collect()
        };
        assert_eq!(acts(&mut app), [
            Action::MusicFilter(MusicFilter::All),
            Action::MusicFilter(MusicFilter::Album),
            Action::MusicFilter(MusicFilter::Artist),
            Action::MusicFilter(MusicFilter::Folder),
        ]);
        assert_eq!(app.state.music_filter, MusicFilter::All, "the list starts on every song");

        let every = app.frame(&f, &i).canvas.data.clone();
        for filter in [MusicFilter::Album, MusicFilter::Artist, MusicFilter::Folder] {
            tap_at(&mut app, &f, &i, Action::MusicFilter(filter), 1_000);
            assert_eq!(app.state.music_filter, filter);
            assert_eq!(app.screen(), Screen::Music, "the page stays where it is");
            assert_eq!(app.state.music_face, MusicFace::List, "and keeps the face it showed");
            assert_ne!(app.frame(&f, &i).canvas.data, every, "{filter:?} says of itself that it is empty");
        }
        tap_at(&mut app, &f, &i, Action::MusicFilter(MusicFilter::All), 2_000);
        assert_eq!(app.frame(&f, &i).canvas.data, every, "and the list of every song says what it said");
    }

    /// The shelves name what the list holds and nothing else, so the face
    /// that plays a song carries none of them.
    #[test]
    fn the_shelves_stand_only_under_the_list() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Music, W, H, PPD);
        tap_at(&mut app, &f, &i, Action::MusicFace(MusicFace::Play), 1_000);
        let acts: Vec<Action> = app.frame(&f, &i).hits.iter().map(|h| h.action).collect();
        assert!(!acts.iter().any(|a| matches!(a, Action::MusicFilter(_))), "the playing face carries no shelves");
    }
}
