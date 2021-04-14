use super::{Drawable, StatefulPane};
use crate::{
    models::Overview,
    widgets::{
        chart::{ChartData, RChart},
        help::Help,
    },
    ManagementClient,
};

use std::{
    sync::{mpsc, Arc},
    thread,
};

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

const HELP: &str = "Welcome to RabbiTui! The help displayed
here is relevant to the Overview tab. Every help panel will be specific to the tab you are in.

The overview pane shows high level throughput analytics.

Keys:
  - h: previous tab
  - l: next tab
  - ?: close the help menu";

#[derive(Default)]
struct OverviewData {
    overall: ChartData,
    ready: ChartData,
    unacked: ChartData,
    disk_read_rate: ChartData,
    disk_write_rate: ChartData,
}

pub struct OverviewPane<M>
where
    M: ManagementClient,
{
    data: OverviewData,
    data_chan: mpsc::Receiver<Overview>,
    data_handle: thread::JoinHandle<()>,
    client: Arc<M>,
    counter: f64,
    should_show_help: bool,
}

impl<M> OverviewPane<M>
where
    M: ManagementClient + 'static,
{
    pub fn new(client: Arc<M>) -> Self {
        let data = client.get_overview();
        let mut overall = ChartData::default();
        overall.push(data.queue_totals.messages);
        let mut ready = ChartData::default();
        ready.push(data.queue_totals.messages_ready);
        let mut unacked = ChartData::default();
        unacked.push(data.queue_totals.messages_unacked);
        let mut disk_read_rate = ChartData::default();
        disk_read_rate.push(data.message_stats.disk_reads_details.rate);
        let mut disk_write_rate = ChartData::default();
        disk_write_rate.push(data.message_stats.disk_writes_details.rate);

        let c = Arc::clone(&client);
        let (tx, rx) = mpsc::channel();
        let handler = thread::spawn(move || loop {
            let d = c.get_overview();
            if tx.send(d).is_err() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(2_000));
        });

        Self {
            client: Arc::clone(&client),
            counter: 0.,
            data_chan: rx,
            data_handle: handler,
            data: OverviewData {
                overall,
                ready,
                unacked,
                disk_read_rate,
                disk_write_rate,
            },
            should_show_help: false,
        }
    }

    fn draw_messages_panel<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.overall, &self.data.ready, &self.data.unacked];
        let colors = [Color::Yellow, Color::Cyan, Color::Red];
        RChart::new(datasets, colors).draw(f, area);
    }

    fn draw_message_rates_panel<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.disk_read_rate, &self.data.disk_write_rate];
        let colors = [Color::Magenta, Color::Green];
        RChart::new(datasets, colors).draw(f, area);
    }

    fn draw_message_rates_list<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.disk_read_rate, &self.data.disk_write_rate];
        let colors = [Color::Magenta, Color::Green];
        let labels = ["Disk read", "Disk write"];
        self.draw_info_list(f, area, labels, datasets, colors, "/s");
    }

    fn draw_info_list<B, const W: usize>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        labels: [&str; W],
        values: [&ChartData; W],
        colors: [Color; W],
        suffix: &str,
    ) where
        B: Backend,
    {
        let items: Vec<ListItem> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let val = values[i].last_value();
                ListItem::new(vec![
                    Spans::from(vec![
                        Span::styled(format!("{:<10}", l), Style::default().fg(colors[i])),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}{}", val, suffix),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    // for putting visual space between rows
                    Spans::from(""),
                ])
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::ALL));
        f.render_widget(list, area);
    }

    fn draw_message_list<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.ready, &self.data.overall, &self.data.unacked];
        let labels = ["Ready", "Total", "Unacked"];
        let colors = [Color::Yellow, Color::Cyan, Color::Red];
        self.draw_info_list(f, area, labels, datasets, colors, "");
    }
}

impl<M, B> Drawable<B> for OverviewPane<M>
where
    M: ManagementClient + 'static,
    B: Backend,
{
    fn draw(&mut self, f: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .split(area);
        let count_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
            .split(chunks[0]);
        let rate_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(85), Constraint::Percentage(15)].as_ref())
            .split(chunks[1]);
        self.draw_messages_panel(f, count_chunks[0]);
        self.draw_message_list(f, count_chunks[1]);
        self.draw_message_rates_panel(f, rate_chunks[0]);
        self.draw_message_rates_list(f, rate_chunks[1]);
        if self.should_show_help {
            let help = Help::new(HELP);
            help.draw(f, area);
        }
    }
}

impl<M, B> StatefulPane<B> for OverviewPane<M>
where
    B: Backend,
    M: ManagementClient + 'static,
{
    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('?') => {
                self.should_show_help = !self.should_show_help;
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        if let Some(update) = self.data_chan.try_iter().next() {
            self.counter += 1.0;
            self.data.ready.push(update.queue_totals.messages_ready);
            self.data.overall.push(update.queue_totals.messages);
            self.data.unacked.push(update.queue_totals.messages_unacked);
            self.data
                .disk_write_rate
                .push(update.message_stats.disk_writes_details.rate);
            self.data
                .disk_read_rate
                .push(update.message_stats.disk_reads_details.rate);
        }
    }
}
