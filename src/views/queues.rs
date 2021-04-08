use super::{Drawable, StatefulPane, centered_rect};
use crate::{
    TabsState,
    models::QueueInfo,
    widgets::{help::Help, notif::Notification},
    DataContainer, Datatable, ManagementClient, Rowable,
};

use clipboard::{ClipboardContext, ClipboardProvider};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Layout, Rect, Direction},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

const HELP: &str = "The Queues tab is where you can view information on
existing queues.

Keys:
  - h: previous tab
  - l: next tab
  - k: previous row
  - j: next row
  - p: drop message into queue from clipboard
  - ctrl + p: pop message from queue onto clipboard
  - d: purge selected queue
  - return: confirm prompts
  - ?: close the help menu";

const CONFIRM: &str = "This is a destructive action. Confirm action:";


pub struct QueuesPane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<QueueInfo>,
    confirmation: Datatable<&'a str>,
    client: &'a M,
    // TODO this should probably be a Rc<RefMut<>>
    // to the parent app. Probably not best
    // for an indv pane to have a clipboard context
    // when there is only 1 system clipboard..
    clipboard: ClipboardContext,
    should_notif_paste: bool,
    should_notif_copy: bool,
    should_notif_no_msg: bool,
    should_notif_purged: bool,
    should_show_help: bool,
    should_confirm: bool,
}

impl<'a, M> QueuesPane<'a, M>
where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self {
        let data = client.get_queues_info();
        let table = Datatable::<QueueInfo>::new(data);
        let c_table = Datatable::<&'a str>::new(vec!["Yes", "No"]);
        Self {
            table,
            confirmation: c_table,
            client,
            // TODO handle unable to make clipboard?
            clipboard: ClipboardProvider::new().unwrap(),
            should_notif_paste: false,
            should_notif_copy: false,
            should_notif_no_msg: false,
            should_notif_purged: false,
            should_show_help: false,
            should_confirm: false,
        }
    }

    fn draw_confirmation_box<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
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
        let txt = Paragraph::new(Text::raw(CONFIRM))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        let data = self.confirmation.data.get();
        let rows = data.iter().map(|r| {
            let vecd = vec![r.to_string()];
            let cell = vecd.iter()
                .map(|c| Cell::from(c.clone()));
            Row::new(cell).bottom_margin(1)
        });
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let t = Table::new(rows)
            .block(Block::default())
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Percentage(100),
            ]);
        f.render_widget(Clear, pop_area);
        f.render_widget(background, pop_area);
        f.render_widget(txt, chunks[1]);
        f.render_stateful_widget(t, chunks[2], &mut self.confirmation.state);
    }
}

impl<M, B> Drawable<B> for QueuesPane<'_, M>
where
    M: ManagementClient,
    B: Backend,
{
    fn draw(&mut self, f: &mut Frame<B>, area: Rect) {
        let data = self.table.data.get();
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)].as_ref())
            .margin(1)
            .split(area);
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_literals = QueueInfo::headers();
        let header_cells = header_literals
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().fg(Color::Green)));
        let header = Row::new(header_cells)
            .style(normal_style)
            .height(1)
            .bottom_margin(1);
        let rows = data.iter().map(|r| {
            let vecd = r.to_row();
            let cells = vecd.iter().map(|c| Cell::from(c.clone()));
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
        if self.should_notif_paste {
            Notification::new("Pasted from clipboard!".to_string()).draw(f, area);
        } else if self.should_notif_copy {
            Notification::new("Copied to clipboard!".to_string()).draw(f, area);
        } else if self.should_notif_no_msg {
            Notification::new("No messages to copy!".to_string()).draw(f, area);
        } else if self.should_show_help {
            Help::new(HELP).draw(f, area);
        } else if self.should_confirm {
            self.draw_confirmation_box(f, area);
        } else if self.should_notif_purged {
            Notification::new("Queue purged!".to_string()).draw(f, area);
        }
    }
}

impl<'a, M, B> StatefulPane<B> for QueuesPane<'a, M>
where
    M: ManagementClient,
    B: Backend,
{
    fn update_in_background(&self) -> bool {
        false
    }

    fn handle_key(&mut self, key: Key) {
        self.should_notif_copy = false;
        self.should_notif_paste = false;
        self.should_notif_no_msg = false;
        self.should_notif_purged = false;
        match key {
            Key::Char('j') => {
                if self.should_confirm {
                    self.confirmation.next();
                } else {
                    self.table.next();
                }
            }
            Key::Char('k') => {
                if self.should_confirm {
                    self.confirmation.previous();
                } else {
                    self.table.previous();
                }
            }
            Key::Char('p') => {
                if let Some(i) = self.table.state.selected() {
                    // TODO handle clipboard fail.
                    let body = self.clipboard.get_contents().unwrap();
                    let queue_info = &self.table.data.get()[i];
                    self.client.post_queue_payload(
                        queue_info.name.clone(),
                        &queue_info.vhost,
                        body,
                    );
                    self.should_notif_paste = true;
                }
            }
            Key::Ctrl('p') => {
                if let Some(i) = self.table.state.selected() {
                    let info = &self.table.data.get()[i];
                    let res = self.client.pop_queue_item(&info.name, &info.vhost);
                    match res {
                        Some(m) => {
                            self.clipboard.set_contents(m.payload).unwrap();
                            self.should_notif_copy = true;
                        }
                        None => {
                            self.should_notif_no_msg = true;
                        }
                    }
                }
            }
            Key::Char('d') => {
                if self.table.state.selected().is_some() {
                    self.should_confirm = true;
                }
            }
            Key::Char('\n') => {
                // TODO clean up - this is crunchy
                // confirmation popup is open
                if self.should_confirm {
                    // selected a confirmation
                    if let Some(i) = self.confirmation.state.selected() {
                        if self.confirmation.data.get()[i] == "Yes" {
                            // row on the queues table is selected
                            if let Some(j) = self.table.state.selected() {
                                let info = &self.table.data.get()[j];
                                self.client.purge_queue(&info.name, &info.vhost);
                                self.should_notif_purged = true;
                            }
                        }
                    }
                    self.should_confirm = false;
                    self.confirmation = Datatable::<&'a str>::new(vec!["Yes", "No"]);
                }
            }
            Key::Char('?') => {
                self.should_show_help = !self.should_show_help;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        let new_data = self.client.get_queues_info();
        self.table.data = DataContainer { entries: new_data };
    }
}
