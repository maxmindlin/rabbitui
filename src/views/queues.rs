use super::{Drawable, StatefulPane};
use crate::{
    models::QueueInfo,
    widgets::{
        confirmation::ConfirmationBox, files::FileNavigator, help::Help, notif::Notification,
    },
    DataContainer, Datatable, ManagementClient, Rowable,
};

use std::fs;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use clipboard::{ClipboardContext, ClipboardProvider};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
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
  - return: select
  - f: open/close file explorer
  - backspace: go to parent in file explorer
  - ?: close the help menu";

pub struct QueuesPane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<QueueInfo>,
    confirmation: ConfirmationBox<'a>,
    data_chan: mpsc::Receiver<Vec<QueueInfo>>,
    data_handle: thread::JoinHandle<()>,
    explorer: FileNavigator,
    client: Arc<M>,
    // TODO this should probably be a Rc<RefMut<>>
    // to the parent app. Probably not best
    // for an indv pane to have a clipboard context
    // when there is only 1 system clipboard..
    clipboard: ClipboardContext,
    should_notif_paste: bool,
    should_notif_copy: bool,
    should_notif_no_msg: bool,
    should_notif_purged: bool,
    should_notif_from_file: bool,
    should_show_help: bool,
    should_confirm: bool,
    should_open_files: bool,
}

impl<'a, M> QueuesPane<'a, M>
where
    M: ManagementClient + 'static,
{
    pub fn new(client: Arc<M>) -> Self {
        let data = client.get_queues_info();
        let table = Datatable::<QueueInfo>::new(data);
        let (tx, rx) = mpsc::channel();
        let c = Arc::clone(&client);
        let handler = thread::spawn(move || loop {
            let d = c.get_queues_info();
            if tx.send(d).is_err() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(2_000));
        });
        Self {
            table,
            confirmation: ConfirmationBox::default(),
            explorer: FileNavigator::default(),
            data_chan: rx,
            data_handle: handler,
            client: Arc::clone(&client),
            // TODO handle unable to make clipboard?
            clipboard: ClipboardProvider::new().unwrap(),
            should_notif_paste: false,
            should_notif_copy: false,
            should_notif_no_msg: false,
            should_notif_purged: false,
            should_notif_from_file: false,
            should_show_help: false,
            should_confirm: false,
            should_open_files: false,
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
        }
        if self.should_notif_copy {
            Notification::new("Copied to clipboard!".to_string()).draw(f, area);
        }
        if self.should_notif_no_msg {
            Notification::new("No messages to copy!".to_string()).draw(f, area);
        }
        if self.should_confirm {
            self.confirmation.draw(f, area);
        }
        if self.should_notif_purged {
            Notification::new("Queue purged!".to_string()).draw(f, area);
        }
        if self.should_open_files {
            self.explorer.draw(f, area);
        }
        if self.should_notif_from_file {
            Notification::new("Posted from file!".to_string()).draw(f, area);
        }
        if self.should_show_help {
            Help::new(HELP).draw(f, area);
        }
    }
}

impl<'a, M, B> StatefulPane<B> for QueuesPane<'a, M>
where
    M: ManagementClient,
    B: Backend,
{
    fn handle_key(&mut self, key: Key) {
        self.should_notif_copy = false;
        self.should_notif_paste = false;
        self.should_notif_no_msg = false;
        self.should_notif_purged = false;
        self.should_notif_from_file = false;
        match key {
            Key::Char('j') => {
                if self.should_confirm {
                    self.confirmation.next();
                } else if self.should_open_files {
                    self.explorer.next();
                } else {
                    self.table.next();
                }
            }
            Key::Char('k') => {
                if self.should_confirm {
                    self.confirmation.previous();
                } else if self.should_open_files {
                    self.explorer.previous();
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
            Key::Char('f') => {
                self.should_open_files = !self.should_open_files;
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
                } else if self.should_open_files {
                    if let Some(f) = self.explorer.select() {
                        if let Some(i) = self.table.state.selected() {
                            // TODO handle unable to read content
                            let body = fs::read_to_string(f).unwrap();
                            let info = &self.table.data.get()[i];
                            self.client
                                .post_queue_payload(info.name.clone(), &info.vhost, body);
                            self.should_open_files = false;
                            self.should_notif_from_file = true;
                        }
                    }
                }
            }
            Key::Backspace => {
                if self.should_open_files {
                    self.explorer.select_parent();
                }
            }
            Key::Char('?') => {
                self.should_show_help = !self.should_show_help;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        if let Some(d) = self.data_chan.try_iter().next() {
            self.table.data = DataContainer { entries: d };
        }
    }
}
