use super::{centered_rect, Drawable, Pane};
use crate::models::{ExchangeBindings, ExchangeInfo};
use crate::{DataContainer, Datatable, ManagementClient, Rowable};

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Row, Table},
    Frame,
};

pub struct ExchangePane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<ExchangeInfo>,
    bindings_table: Option<Datatable<ExchangeBindings>>,
    should_draw_popout: bool,
    client: &'a M,
}

impl<M> ExchangePane<'_, M>
where
    M: ManagementClient,
{
    fn draw_popout<B: Backend>(&self, f: &mut Frame<B>, data: &Vec<ExchangeBindings>, area: Rect) {
        let b_header_lits = ExchangeBindings::headers();
        let b_header_cells = b_header_lits
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
        let b_header = Row::new(b_header_cells)
            .style(Style::default())
            .height(1)
            .bottom_margin(1);
        let b_rows = data.iter().map(|r| {
            let vecd = r.to_row();
            let cells = vecd.iter().map(|c| Cell::from(c.clone()));
            Row::new(cells).bottom_margin(1)
        });
        let b_t = Table::new(b_rows)
            .header(b_header)
            .block(Block::default().borders(Borders::ALL).title("Bindings"))
            .widths(&[
                Constraint::Percentage(70),
                Constraint::Length(30),
                Constraint::Max(10),
            ]);
        let pop_area = centered_rect(60, 50, area);
        f.render_widget(Clear, pop_area);
        f.render_widget(b_t, pop_area);
    }
}

impl<'a, M> Pane<ExchangePane<'a, M>>
where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self
    {
        let data = client.get_exchange_overview();
        let table = Datatable::<ExchangeInfo>::new(data);
        Self {
            content: ExchangePane {
                table,
                bindings_table: None,
                should_draw_popout: false,
                client,
            },
        }
    }
}

impl<M> Drawable for ExchangePane<'_, M>
where
    M: ManagementClient,
{
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let row_data = self.table.data.get();
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .margin(1)
            .split(area);
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_literals = ExchangeInfo::headers();
        let header_cells = header_literals
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Green)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = row_data.iter().map(|r| {
            let vecd = r.to_row();
            let cells = vecd
                .iter()
                // TODO this clone here is bad
                .map(|c| Cell::from(c.clone()));
            Row::new(cells).bottom_margin(1)
        });
        let t = Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Exchanges"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(50),
                Constraint::Length(30),
                Constraint::Max(10),
            ]);
        f.render_stateful_widget(t, rects[0], &mut self.table.state);
        if self.should_draw_popout {
            match &self.bindings_table {
                Some(t) => {
                    self.draw_popout(f, t.data.get(), area);
                }
                None => match self.table.state.selected() {
                    None => {}
                    Some(i) => {
                        let drilldown = &row_data[i];
                        let binding_data = self.client.get_exchange_bindings(drilldown);
                        self.draw_popout(f, &binding_data, area);
                        self.bindings_table =
                            Some(Datatable::<ExchangeBindings>::new(binding_data));
                    }
                },
            };
        } else {
            self.bindings_table = None;
        }
    }

    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('j') => {
                self.table.next();
                self.should_draw_popout = false;
            }
            Key::Char('k') => {
                self.table.previous();
                self.should_draw_popout = false;
            }
            Key::Char('\n') => {
                self.should_draw_popout = !self.should_draw_popout;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        let data = self.client.get_exchange_overview();
        self.table.data = DataContainer {
            entries: data,
            staleness: 0,
        };
    }
}
