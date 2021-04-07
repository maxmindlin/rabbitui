use super::{centered_rect, Drawable, Pane};
use crate::models::{QueueInfo};
use crate::{DataContainer, Datatable, ManagementClient, Rowable};

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Row, Table},
    Frame,
};

pub struct QueuesPane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<QueueInfo>,
    client: &'a M,
}

impl<'a, M> Pane<QueuesPane<'a, M>>
where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self {
        let data = client.get_queues_info();
        let table = Datatable::<QueueInfo>::new(data);
        Self {
            content: QueuesPane {
                table,
                client,
            }
        }
    }
}

impl<M> Drawable for QueuesPane<'_, M>
where
    M: ManagementClient
{
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let data = self.table.data.get();
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .margin(1)
            .split(area);
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_literals = QueueInfo::headers();
        let header_cells = header_literals.iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Green)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = data.iter().map(|r| {
            let vecd = r.to_row();
            let cells = vecd.iter()
                .map(|c| Cell::from(c.clone()));
            Row::new(cells).bottom_margin(1)
        });
        let t = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Queues"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
                Constraint::Percentage(16),
            ]);
        f.render_stateful_widget(t, rects[0], &mut self.table.state);
    }

    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('j') => self.table.next(),
            Key::Char('k') => self.table.next(),
            _ => {},
        }
    }

    fn update(&mut self) {
        
    }
}
