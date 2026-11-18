//! The tasks page the desktop opens: which of its three faces the page shows.
//! The processes it will list, the services and the figures it will draw are
//! not read off the phone yet — the page is, so that the way in is settled
//! before there is anything to show.

/// The three faces of the tasks page: the processes the machine is running,
/// the services it keeps going, and how it is bearing them. The words across
/// the top show one at a time, and a finger dragged level across the page
/// steps to the face beside it, stopping at either end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TasksFace {
    /// The processes, as a list.
    Process,
    /// The services the machine keeps running.
    Service,
    /// The performance: the figures the machine is running at.
    Performance,
}

impl TasksFace {
    /// The faces in the order the words name them, left to right.
    pub const ALL: [TasksFace; 3] = [TasksFace::Process, TasksFace::Service, TasksFace::Performance];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Action, App, Screen, Touch};

    // A panel as wide as the phone's, in dp: three words plus the gear need
    // room. At 200 dp the last word's hit falls under the gear's and a tap
    // on it is swallowed — a phone panel is near 400 dp and has no such
    // trouble.
    const W: u32 = 800;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// The three words across the top are the page's three faces, in the
    /// order the page names them: the processes, the services between them,
    /// and the performance to their right. A touch turns to the face it
    /// names and leaves the page there, and each face draws something of
    /// its own.
    #[test]
    fn the_three_words_across_the_top_turn_the_page_to_each_face() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Tasks, W, H, PPD);
        let acts = |app: &mut App| -> Vec<Action> {
            app.frame(&f, &i).hits.iter().map(|h| h.action).filter(|a| matches!(a, Action::TasksFace(_))).collect()
        };
        assert_eq!(
            acts(&mut app),
            [
                Action::TasksFace(TasksFace::Process),
                Action::TasksFace(TasksFace::Service),
                Action::TasksFace(TasksFace::Performance),
            ],
            "the service stands between the processes and the performance"
        );
        assert_eq!(app.state.tasks_face, TasksFace::Process, "it opens on the processes");

        let processes = app.frame(&f, &i).canvas.data.clone();
        tap_at(&mut app, &f, &i, Action::TasksFace(TasksFace::Service), 1_000);
        assert_eq!(app.state.tasks_face, TasksFace::Service);
        let services = app.frame(&f, &i).canvas.data.clone();
        assert_ne!(services, processes, "the services say something else");

        tap_at(&mut app, &f, &i, Action::TasksFace(TasksFace::Performance), 2_000);
        assert_eq!(app.state.tasks_face, TasksFace::Performance);
        assert_eq!(app.screen(), Screen::Tasks, "the page stays where it is");
        assert_ne!(app.frame(&f, &i).canvas.data, services, "and the performance says something else again");

        tap_at(&mut app, &f, &i, Action::TasksFace(TasksFace::Process), 3_000);
        assert_eq!(app.state.tasks_face, TasksFace::Process);
        assert_eq!(app.frame(&f, &i).canvas.data, processes, "the processes say what they said");
    }

    /// The same steps by finger: a pull to the left moves one face to the
    /// right, a pull back one to the left, and the ends hold rather than
    /// wrapping round to the face at the other end.
    #[test]
    fn a_finger_dragged_level_across_the_page_steps_and_stops() {
        let f = fonts();
        let mut app = App::new(Screen::Tasks, W, H, PPD);
        let pull = |app: &mut App, from: f32, to: f32, now: u64| {
            app.touch(Touch::Down(from, 600.0), &f, now);
            app.touch(Touch::Move(to, 605.0), &f, now);
            app.touch(Touch::Up, &f, now);
        };

        pull(&mut app, 700.0, 100.0, 0);
        assert_eq!(app.state.tasks_face, TasksFace::Service, "a pull left shows the service beside the processes");
        pull(&mut app, 700.0, 100.0, 1_000);
        assert_eq!(app.state.tasks_face, TasksFace::Performance, "another shows the performance");
        pull(&mut app, 700.0, 100.0, 2_000);
        assert_eq!(app.state.tasks_face, TasksFace::Performance, "a third stays at the last face");

        pull(&mut app, 100.0, 700.0, 3_000);
        assert_eq!(app.state.tasks_face, TasksFace::Service, "a pull back shows the service again");
        pull(&mut app, 100.0, 700.0, 4_000);
        assert_eq!(app.state.tasks_face, TasksFace::Process, "and another brings the processes");
        pull(&mut app, 100.0, 700.0, 5_000);
        assert_eq!(app.state.tasks_face, TasksFace::Process, "a last one stays at the first face");
    }

    /// The gear stands at the page's top right with the other three apps'
    /// own, and there is no settings page behind it yet: a touch leaves the
    /// page where it is rather than doing nothing at all.
    #[test]
    fn the_gear_stands_over_the_page_with_nothing_behind_it() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Tasks, W, H, PPD);
        let frame = app.frame(&f, &i);
        assert_eq!(frame.button, Some((crate::widgets::Glyph::Gear, Action::Idle)), "it carries the clock's gear");
        assert!(frame.button_still, "the page never scrolls, so its chip stands");
        tap_button(&mut app, &f, &i, 0);
        assert_eq!(app.screen(), Screen::Tasks, "it stays where it is");
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
        let mut app = App::new(Screen::Tasks, PW, PH, PPPD);
        let frame = app.frame(&f, &i);
        let overlap = |a: crate::geom::Rect, b: crate::geom::Rect| {
            a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
        };
        let tab_rects: Vec<_> = frame.hits.iter().filter(|h| matches!(h.action, Action::TasksFace(_))).map(|h| h.rect).collect();
        assert_eq!(tab_rects.len(), 3, "three words on the page");
        for k in 0..tab_rects.len() {
            for j in (k + 1)..tab_rects.len() {
                assert!(!overlap(tab_rects[k], tab_rects[j]), "word {k} and word {j} must not overlap");
            }
            assert!(!overlap(tab_rects[k], button_rect(&app.theme())), "word {k} must stay clear of the gear");
        }
    }
}
