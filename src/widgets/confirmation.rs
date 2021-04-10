use crate::{
    models::QueueInfo,
    views::centered_rect,
    widgets::{help::Help, notif::Notification},
    DataContainer, Datatable, ManagementClient, Rowable, TabsState,
};

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

const TEXT: &str = "This is a destructive action. Confirm action:";

pub struct ConfirmationBox<'a> {
    table: Datatable<&'a str>,
}

impl<'a> ConfirmationBox<'a> {
    pub fn reset(&mut self) {
        self.table.state.select(Some(0));
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let pop_area = centered_rect(30, 30, area);
        let background = Block::default()
            .title(Span::styled("Warning", Style::default().fg(Color::Yellow)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::LightYellow));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(20),
                Constraint::Min(0),
            ])
            .margin(1)
            .split(pop_area);
        let txt = Paragraph::new(Text::raw(TEXT))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        let data = self.table.data.get();
        let rows = data.iter().map(|r| {
            let vecd = vec![r.to_string()];
            let cell = vecd.iter().map(|c| Cell::from(c.clone()));
            Row::new(cell).bottom_margin(1)
        });
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let t = Table::new(rows)
            .block(Block::default())
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[Constraint::Percentage(100)]);
        f.render_widget(Clear, pop_area);
        f.render_widget(background, pop_area);
        f.render_widget(txt, chunks[1]);
        f.render_stateful_widget(t, chunks[2], &mut self.table.state);
    }

    pub fn is_confirmed(&self) -> bool {
        self.table.state.selected() == Some(1)
    }

    pub fn next(&mut self) {
        self.table.next();
    }

    pub fn previous(&mut self) {
        self.table.previous();
    }
}

impl<'a> Default for ConfirmationBox<'a> {
    fn default() -> Self {
        let mut table = Datatable::<&'a str>::new(vec!["No", "Yes"]);
        table.state.select(Some(0));
        Self { table }
    }
}
