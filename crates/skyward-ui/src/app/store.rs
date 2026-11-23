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
    use crate::Canvas;

    // A panel as wide as the phone's, in dp: three words plus the gear need
    // room, as on the tasks page.
    const W: u32 = 800;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// The three faces are named along the bar at the foot, in the order the
    /// page names them: the front, everything, and the phone's own. A touch
    /// turns to the face it names and leaves the page there.
    #[test]
    fn the_three_faces_along_the_foot_turn_the_page_to_each() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, W, H, PPD);
        let acts = |app: &mut App| -> Vec<Action> {
            let frame = app.frame(&f, &i);
            frame.foot.as_ref().expect("the store names its faces at the foot").faces.iter().map(|fc| fc.action).collect()
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

    /// The pill behind the face shown moves to the one just touched, and a
    /// frame into the move it is between the two rather than on either.
    #[test]
    fn the_pill_moves_to_the_face_just_touched() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, W, H, PPD);
        let at = |app: &mut App, now: u64| {
            let shown = StoreFace::ALL.iter().position(|face| *face == app.state.store_face).unwrap_or(0);
            app.foot_at(now, shown)
        };
        assert_eq!(at(&mut app, 0), 0.0, "it opens under the front");
        tap_at(&mut app, &f, &i, Action::StoreFace(StoreFace::Mine), 1_000);
        assert_eq!(app.state.store_face, StoreFace::Mine);
        assert_eq!(at(&mut app, 1_000), 0.0, "the move starts from where it was");
        let between = at(&mut app, 1_000 + 90);
        assert!(between > 0.0 && between < 2.0, "half way it is between the two faces, not on either");
        assert_eq!(at(&mut app, 1_000 + 5_000), 2.0, "and it comes to rest under the phone's own");
    }

    /// A finger that comes down on the bar and pulls sideways is working the
    /// bar: it does not turn the page, which is what the bar is for.
    #[test]
    fn a_finger_pulled_along_the_bar_does_not_turn_the_page() {
        let f = fonts();
        let mut app = App::new(Screen::Store, W, H, PPD);
        // A page is drawn before a finger lands on it; so is one here.
        let _ = app.frame(&f, &info());
        let t = app.theme();
        let g = crate::widgets::foot_geom(&t, 3, 0.0);
        let (x, y) = g.cells[0].centre();
        app.touch(Touch::Down(x, y), &f, 0);
        app.touch(Touch::Move(x - 600.0, y + 4.0), &f, 10);
        app.touch(Touch::Up, &f, 20);
        app.tick(10_000);
        assert_eq!(app.state.store_face, StoreFace::Home, "the face stays where it was");
        // And a pull across the page itself still turns it.
        app.touch(Touch::Down(700.0, 600.0), &f, 20_000);
        app.touch(Touch::Move(100.0, 605.0), &f, 20_010);
        app.touch(Touch::Up, &f, 20_020);
        app.tick(30_000);
        assert_eq!(app.state.store_face, StoreFace::All, "a pull across the middle shows everything");
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

    /// The page's own top lies over the strip rather than in the pages: the
    /// large title and the gear keep their place, and their ink, while a
    /// finger pulls one face aside. Without them laid over the strip the top
    /// of the screen would empty out for the length of the pull.
    #[test]
    fn the_title_and_the_gear_stay_while_a_face_is_pulled_aside() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Store, W, H, PPD);
        let mut still = Canvas::new(W, H);
        app.compose(&mut still, &f, &i, "12:00", "", 0);
        // A finger across the middle of the page, well past the slop.
        app.touch(Touch::Down(700.0, 400.0), &f, 0);
        app.touch(Touch::Move(400.0, 404.0), &f, 16);
        let mut moved = Canvas::new(W, H);
        app.compose(&mut moved, &f, &i, "12:00", "", 16);

        // Every dark pixel across the page's top — the title's strokes and
        // the gear's — stands where it stood, at the grey it was.
        let ink: Vec<(u32, u32, u8)> = (0..H / 3)
            .flat_map(|y| (0..W).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let v = grey_at(&still, x as f32, y as f32);
                (v < 128).then_some((x, y, v))
            })
            .collect();
        assert!(!ink.is_empty(), "the page's top carries the title and the gear");
        for (x, y, v) in ink {
            assert_eq!(grey_at(&moved, x as f32, y as f32), v, "pixel ({x}, {y}) of the page's top moved");
        }
    }

    /// The three cells have to sit side by side on the widest panel as well
    /// as the narrowest page the tests draw. None may cover another, none may
    /// reach the gear, and the bar keeps clear of the screen's foot so the
    /// pull up to the desktop still finds it.
    #[test]
    fn the_three_cells_keep_their_own_space_clear_of_the_gear() {
        use crate::app::testing::{H as PH, PPD as PPPD, W as PW};
        use crate::widgets::{button_rect, foot_geom};

        let f = fonts();
        let mut app = App::new(Screen::Store, PW, PH, PPPD);
        let t = app.theme();
        let g = foot_geom(&t, 3, 0.0);
        let overlap = |a: crate::geom::Rect, b: crate::geom::Rect| {
            a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
        };
        assert_eq!(g.cells.len(), 3, "three cells on the bar");
        for k in 0..g.cells.len() {
            for j in (k + 1)..g.cells.len() {
                assert!(!overlap(g.cells[k], g.cells[j]), "cell {k} and cell {j} must not overlap");
            }
            assert!(!overlap(g.cells[k], button_rect(&t)), "cell {k} must stay clear of the gear");
        }
        // The pill sits within the cell it marks, and within no other.
        assert!(g.pill.x >= g.cells[0].x && g.pill.x + g.pill.w <= g.cells[0].x + g.cells[0].w, "the pill marks one cell");
        assert!(g.pill.w < g.cells[0].w, "and leaves the cell's sides visible either side of it");
        // The bar floats: the gesture strip at the very foot stays the
        // screen's own.
        assert!(g.bar.y + g.bar.h <= PH as f32 - t.m.dp(24.0), "the bar leaves the gesture strip clear");
        // The faces answer on the bar, and nowhere else on the page.
        let frame = app.frame(&f, &info());
        assert!(!frame.hits.iter().any(|h| matches!(h.action, Action::StoreFace(_))), "the page itself carries no face controls");
    }
}
