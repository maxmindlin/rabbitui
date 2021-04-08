use super::{centered_rect, Drawable, StatefulPane};
use crate::widgets::help::Help;
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

const HELP: &str = "The Exchanges tab is where you view
all existing exchanges with drilldown information.

Keys:
  - h: previous tab
  - l: next tab
  - k: previous row
  - j: next row
  - return: open/close drilldown for selected exchange
  - ?: close the help menu";

pub struct ExchangePane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<ExchangeInfo>,
    bindings_table: Datatable<ExchangeBindings>,
    should_fetch_bindings: bool,
    should_draw_popout: bool,
    should_show_help: bool,
    client: &'a M,
}

impl<'a, M> ExchangePane<'a, M>
where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self {
        let data = client.get_exchange_overview();
        let table = Datatable::<ExchangeInfo>::new(data);
        Self {
            table,
            bindings_table: Datatable::default(),
            should_fetch_bindings: false,
            should_draw_popout: false,
            should_show_help: false,
            client,
        }
    }

    fn draw_popout<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let data = self.bindings_table.data.get();
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
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let b_t = Table::new(b_rows)
            .header(b_header)
            .block(Block::default().borders(Borders::ALL).title("Bindings"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(50),
                Constraint::Length(30),
                Constraint::Max(10),
            ]);
        let pop_area = centered_rect(60, 50, area);
        f.render_widget(Clear, pop_area);
        f.render_stateful_widget(b_t, pop_area, &mut self.bindings_table.state);
    }

    fn forward_table(&mut self) {
        if self.should_draw_popout {
            self.bindings_table.next();
        } else {
            self.table.next();
        }
    }

    fn back_table(&mut self) {
        if self.should_draw_popout {
            self.bindings_table.previous();
        } else {
            self.table.previous();
        }
    }
}

impl<M, B> Drawable<B> for ExchangePane<'_, M>
where
    M: ManagementClient,
    B: Backend,
{
    fn draw(&mut self, f: &mut Frame<B>, area: Rect) {
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
            match self.table.state.selected() {
                None => {}
                Some(i) => {
                    if self.should_fetch_bindings {
                        let drilldown = &row_data[i];
                        let binding_data = self.client.get_exchange_bindings(drilldown);
                        self.bindings_table = Datatable::<ExchangeBindings>::new(binding_data);
                        self.should_fetch_bindings = false;
                    }
                    self.draw_popout(f, area);
                }
            }
        }

        if self.should_show_help {
            let help = Help::new(HELP);
            help.draw(f, area);
        }
    }
}

impl<M, B> StatefulPane<B> for ExchangePane<'_, M>
where
    M: ManagementClient,
    B: Backend,
{
    fn update_in_background(&self) -> bool {
        false
    }

    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('j') => {
                self.forward_table();
            }
            Key::Char('k') => {
                self.back_table();
            }
            Key::Char('\n') => {
                self.should_fetch_bindings = true;
                self.should_draw_popout = !self.should_draw_popout;
            }
            Key::Char('?') => {
                self.should_show_help = !self.should_show_help;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        let data = self.client.get_exchange_overview();
        self.table.data = DataContainer {
            entries: data,
        };
    }
}
