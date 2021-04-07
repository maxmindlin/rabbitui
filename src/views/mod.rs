pub mod exchange;
pub mod overview;
pub mod queues;

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub trait Drawable {
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect);
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

pub struct Pane<D>
where
    D: Drawable,
{
    pub content: D,
}
