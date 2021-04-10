use super::{centered_rect, Drawable, StatefulPane};
use crate::{
    models::QueueInfo,
    widgets::{confirmation::ConfirmationBox, help::Help, notif::Notification},
    DataContainer, Datatable, ManagementClient, Rowable, TabsState,
};

use clipboard::{ClipboardContext, ClipboardProvider};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
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

pub struct QueuesPane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<QueueInfo>,
    confirmation: ConfirmationBox<'a>,
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
        Self {
            table,
            confirmation: ConfirmationBox::default(),
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
                Constraint::Percentage(20),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
                Constraint::Percentage(10),
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
            self.confirmation.draw(f, area);
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
                if self.should_confirm {
                    // The confirmation box is already open and a
                    // second enter command has been issued.
                    if self.confirmation.is_confirmed() {
                        if let Some(i) = self.table.state.selected() {
                            let info = &self.table.data.get()[i];
                            self.client.purge_queue(&info.name, &info.vhost);
                            self.should_notif_purged = true;
                        }
                    }
                    self.confirmation.reset();
                    self.should_confirm = false;
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
