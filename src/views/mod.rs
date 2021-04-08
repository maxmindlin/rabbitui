pub mod exchange;
pub mod overview;
pub mod queues;

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// A generically drawable object. Simply describes
/// ability to be drawn given an area boundary and
/// a frame.
pub trait Drawable<B>
where
    B: Backend,
{
    fn draw(&mut self, f: &mut Frame<B>, area: Rect);
}


/// A pane that manages its own state. This involves
/// any knowledge around handling updates and inputs.
pub trait StatefulPane<B>: Drawable<B>
where
    B: Backend,
{
    fn update_in_background(&self) -> bool;
    fn handle_key(&mut self, key: Key);
    fn update(&mut self);
}

/// helper function to create a centered rect using up
/// certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
