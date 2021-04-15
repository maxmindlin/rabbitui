use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

fn notif_rect(r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(93), Constraint::Percentage(7)].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(85), Constraint::Percentage(15)].as_ref())
        .split(layout[1])[1]
}

pub struct Notification {
    msg: String,
}

impl Notification {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let notif = Paragraph::new(Text::raw(&self.msg))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Center);
        let pop_area = notif_rect(area);
        f.render_widget(Clear, pop_area);
        f.render_widget(notif, pop_area);
    }
}
