//! The Aether store's page: which of its three faces the page shows. What
//! the store puts in front, all it lists and what the phone already has are
//! not read from anywhere yet — the page is, so that the way in is settled
//! before there is a shelf to fill, as the tasks page is.

/// The three faces of the Aether store: what it puts in front, everything
/// it lists, and what the phone already has. The words across the top show
/// one at a time, and a finger dragged level across the page steps to the
/// face beside it, stopping at either end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreFace {
    /// The front: what the store would put in front of the phone.
    Home,
    /// Every app the store lists.
    All,
    /// The apps this phone already has from the store.
    Mine,
}

impl StoreFace {
    /// The faces in the order the words name them, left to right.
    pub const ALL: [StoreFace; 3] = [StoreFace::Home, StoreFace::All, StoreFace::Mine];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Action, App, Screen, Touch};

    // A panel as wide as the phone's, in dp: three words plus the gear need
    // room, as on the tasks page.
    const W: u32 = 800;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// The three words across the top are the page's three faces, in the
    /// order the page names them: the front, everything, and the phone's
    /// own. A touch turns to the face it names and leaves the page there.
    #[test]
    fn the_three_words_across_the_top_turn_the_page_to_each_face() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, W, H, PPD);
        let acts = |app: &mut App| -> Vec<Action> {
            app.frame(&f, &i).hits.iter().map(|h| h.action).filter(|a| matches!(a, Action::StoreFace(_))).collect()
        };
        assert_eq!(
            acts(&mut app),
            [
                Action::StoreFace(StoreFace::Home),
                Action::StoreFace(StoreFace::All),
                Action::StoreFace(StoreFace::Mine),
            ],
            "the front stands first, the phone's own last"
        );
        assert_eq!(app.state.store_face, StoreFace::Home, "it opens on the front");

        let front = app.frame(&f, &i).canvas.data.clone();
        tap_at(&mut app, &f, &i, Action::StoreFace(StoreFace::All), 1_000);
        assert_eq!(app.state.store_face, StoreFace::All);
        let all = app.frame(&f, &i).canvas.data.clone();
        assert_ne!(all, front, "everything says something else");

        tap_at(&mut app, &f, &i, Action::StoreFace(StoreFace::Mine), 2_000);
        assert_eq!(app.state.store_face, StoreFace::Mine);
        assert_eq!(app.screen(), Screen::Store, "the page stays where it is");
        assert_ne!(app.frame(&f, &i).canvas.data, all, "and the phone's own says something else again");

        tap_at(&mut app, &f, &i, Action::StoreFace(StoreFace::Home), 3_000);
        assert_eq!(app.state.store_face, StoreFace::Home);
        assert_eq!(app.frame(&f, &i).canvas.data, front, "the front says what it said");
    }

    /// The same steps by finger: a pull to the left moves one face to the
    /// right, a pull back one to the left, and the ends hold rather than
    /// wrapping round to the face at the other end.
    #[test]
    fn a_finger_dragged_level_across_the_page_steps_and_stops() {
        let f = fonts();
        let mut app = App::new(Screen::Store, W, H, PPD);
        let pull = |app: &mut App, from: f32, to: f32, now: u64| {
            app.touch(Touch::Down(from, 600.0), &f, now);
            app.touch(Touch::Move(to, 605.0), &f, now + 10);
            app.touch(Touch::Up, &f, now + 20);
            // The faces follow the finger and settle once it lifts.
            app.tick(now + 10_000);
        };

        pull(&mut app, 700.0, 100.0, 0);
        assert_eq!(app.state.store_face, StoreFace::All, "a pull left shows everything beside the front");
        pull(&mut app, 700.0, 100.0, 20_000);
        assert_eq!(app.state.store_face, StoreFace::Mine, "another shows the phone's own");
        pull(&mut app, 700.0, 100.0, 40_000);
        assert_eq!(app.state.store_face, StoreFace::Mine, "a third stays at the last face");

        pull(&mut app, 100.0, 700.0, 60_000);
        assert_eq!(app.state.store_face, StoreFace::All, "a pull back shows everything again");
        pull(&mut app, 100.0, 700.0, 80_000);
        assert_eq!(app.state.store_face, StoreFace::Home, "and another brings the front");
        pull(&mut app, 100.0, 700.0, 100_000);
        assert_eq!(app.state.store_face, StoreFace::Home, "a last one stays at the first face");
    }

    /// The gear stands at the page's top right with the other apps' own, and
    /// there is no settings page behind it yet: a touch leaves the page
    /// where it is rather than doing nothing at all.
    #[test]
    fn the_gear_stands_over_the_page_with_nothing_behind_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, W, H, PPD);
        let frame = app.frame(&f, &i);
        assert_eq!(frame.button, Some((crate::widgets::Glyph::Gear, Action::Idle)), "it carries the clock's gear");
        assert!(frame.button_still, "the page never scrolls, so its chip stands");
        tap_button(&mut app, &f, &i, 0);
        assert_eq!(app.screen(), Screen::Store, "it stays where it is");
        assert!(app.toast.is_none(), "and says nothing about a page it does not have");
    }

    /// Three words have to sit side by side under the gear, on the widest
    /// panel as well as the narrowest page the tests draw. None may cover
    /// another and none may run under the button, or a tap on it is eaten.
    #[test]
    fn three_words_keep_their_own_space_clear_of_the_gear() {
        use crate::app::testing::{H as PH, PPD as PPPD, W as PW};
        use crate::widgets::button_rect;

        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, PW, PH, PPPD);
        let frame = app.frame(&f, &i);
        let overlap = |a: crate::geom::Rect, b: crate::geom::Rect| {
            a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
        };
        let tab_rects: Vec<_> = frame.hits.iter().filter(|h| matches!(h.action, Action::StoreFace(_))).map(|h| h.rect).collect();
        assert_eq!(tab_rects.len(), 3, "three words on the page");
        for k in 0..tab_rects.len() {
            for j in (k + 1)..tab_rects.len() {
                assert!(!overlap(tab_rects[k], tab_rects[j]), "word {k} and word {j} must not overlap");
            }
            assert!(!overlap(tab_rects[k], button_rect(&app.theme())), "word {k} must stay clear of the gear");
        }
    }
}
