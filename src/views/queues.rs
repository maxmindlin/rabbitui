use crate::widgets::Notification;
use super::{centered_rect, Drawable, Pane};
use crate::models::QueueInfo;
use crate::{DataContainer, Datatable, ManagementClient, Rowable};

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Text, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table, Paragraph},
    Frame,
};

pub struct QueuesPane<'a, M>
where
    M: ManagementClient,
{
    table: Datatable<QueueInfo>,
    client: &'a M,
    // TODO this should probably be a Rc<RefMut<>>
    // to the parent app. Probably not best
    // for an indv pane to have a clipboard context
    // when there is only 1 system clipboard..
    clipboard: ClipboardContext,
    should_notif_paste: bool,
    should_notif_copy: bool,
    should_notif_no_msg: bool,
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
                // TODO handle unable to make clipboard?
                clipboard: ClipboardProvider::new().unwrap(),
                should_notif_paste: false,
                should_notif_copy: false,
                should_notif_no_msg: false,
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
        if self.should_notif_paste {
            let notif = Notification::new("Pasted from clipboard!".to_string());
            notif.draw(f, area);
        } else if self.should_notif_copy {
            let notif = Notification::new("Copied to clipboard!".to_string());
            notif.draw(f, area);
        } else if self.should_notif_no_msg {
            let notif = Notification::new("No messages to copy!".to_string());
            notif.draw(f, area);
        }
    }

    fn handle_key(&mut self, key: Key) {
        self.should_notif_copy = false;
        self.should_notif_paste = false;
        self.should_notif_no_msg = false;
        match key {
            Key::Char('j') => self.table.next(),
            Key::Char('k') => self.table.previous(),
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
                        },
                        None => {
                            self.should_notif_no_msg = true;
                        }
                    }
                }
            }
            _ => {},
        }
    }

    fn update(&mut self) {
        let new_data = self.client.get_queues_info();
        self.table.data = DataContainer {
            entries: new_data,
            staleness: 0,
        };
    }
}
