use crate::views::centered_rect;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Text, Spans},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub struct Help {
    text: String,
}

impl Help {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let t = Text::raw(&self.text);
        let pg = Paragraph::new(t)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(
                        Span::styled(
                            "Help",
                            Style::default()
                                .fg(Color::Red)
                        )
                    )
            )
            .wrap(Wrap { trim: true });
        let pop_area = centered_rect(30, 40, area);
        f.render_widget(Clear, pop_area);
        f.render_widget(pg, pop_area);
    }
}
