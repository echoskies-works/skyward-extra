//! The tasks page the desktop opens: which of its two faces the page shows.
//! The processes it will list and the figures it will draw are not read off
//! the phone yet — the page is, so that the way in is settled before there is
//! anything to show.

/// The two faces of the tasks page: the processes the machine is running,
/// and how it is bearing them. The words across the top show one at a time,
/// and a finger dragged level across the page turns to the other, as the
/// clock's, the calculator's and the player's do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TasksFace {
    /// The processes, as a list.
    Process,
    /// The performance: the figures the machine is running at.
    Performance,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::testing::*;
    use crate::app::{Action, App, Screen, Touch};

    const W: u32 = 400;
    const H: u32 = 800;
    const PPD: f32 = 2.0;

    /// The two words across the top are the page's two faces, in the order
    /// the page names them: the processes first, the performance to their
    /// right. A touch turns to the face it names and leaves the page there,
    /// and the face shown draws something else than the one it left.
    #[test]
    fn the_two_words_across_the_top_turn_the_page_to_either_face() {
        let (f, i) = (fonts(), info());
        let mut app = App::new(Screen::Tasks, W, H, PPD);
        let acts = |app: &mut App| -> Vec<Action> {
            app.frame(&f, &i).hits.iter().map(|h| h.action).filter(|a| matches!(a, Action::TasksFace(_))).collect()
        };
        assert_eq!(acts(&mut app), [Action::TasksFace(TasksFace::Process), Action::TasksFace(TasksFace::Performance)]);
        assert_eq!(app.state.tasks_face, TasksFace::Process, "it opens on the processes");

        let processes = app.frame(&f, &i).canvas.data.clone();
        tap_at(&mut app, &f, &i, Action::TasksFace(TasksFace::Performance), 1_000);
        assert_eq!(app.state.tasks_face, TasksFace::Performance);
        assert_eq!(app.screen(), Screen::Tasks, "the page stays where it is");
        assert_ne!(app.frame(&f, &i).canvas.data, processes, "and the other face says something else");

        tap_at(&mut app, &f, &i, Action::TasksFace(TasksFace::Process), 2_000);
        assert_eq!(app.state.tasks_face, TasksFace::Process);
        assert_eq!(app.frame(&f, &i).canvas.data, processes, "the processes say what they said");
    }

    /// The same turn by finger: a pull to the left shows the page to the
    /// right of the processes, and a pull back brings them again.
    #[test]
    fn a_finger_dragged_level_across_the_page_turns_it() {
        let f = fonts();
        let mut app = App::new(Screen::Tasks, W, H, PPD);
        app.touch(Touch::Down(340.0, 600.0), &f, 0);
        app.touch(Touch::Move(120.0, 605.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.state.tasks_face, TasksFace::Performance, "a pull to the left shows the page to the right");

        app.touch(Touch::Down(80.0, 600.0), &f, 0);
        app.touch(Touch::Move(300.0, 605.0), &f, 0);
        app.touch(Touch::Up, &f, 0);
        assert_eq!(app.state.tasks_face, TasksFace::Process, "and a pull back brings the processes again");
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
}
