use super::{Drawable, Pane};
use crate::ManagementClient;

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem},
    Frame,
};

const X_WINDOW: u64 = 100;
const Y_PADDING: f64 = 1.1;

#[derive(Default)]
struct ChartData {
    data: Vec<(f64, f64)>,
    // For right now we are using
    // a generic tick counter as our
    // time tracking.
    counter: f64,
}

impl ChartData {
    fn push(&mut self, n: f64) {
        self.data.push((self.counter.clone(), n));
        self.counter += 1.0;
        if self.data.len() > X_WINDOW as usize {
            self.data.remove(0);
        }
    }

    fn x_max(&self) -> u64 {
        self.data.iter().map(|p| p.0 as u64).max().unwrap()
    }

    fn y_max(&self) -> u64 {
        self.data.iter().map(|p| p.1 as u64).max().unwrap()
    }

    fn last_value(&self) -> f64 {
        // this is safe where this is used because we always seed with
        // at least 1 value. If we wanted to be abstract, we would
        // check that data is not empty.
        self.data.iter().last().unwrap().1
    }
}

#[derive(Default)]
struct OverviewData {
    overall: ChartData,
    ready: ChartData,
    unacked: ChartData,
    disk_read_rate: ChartData,
    disk_write_rate: ChartData,
}

pub struct OverviewPane<'a, M>
where
    M: ManagementClient,
{
    data: OverviewData,
    client: &'a M,
    counter: f64,
}

impl<M> OverviewPane<'_, M>
where
    M: ManagementClient,
{
    fn draw_chart<B, const W: usize>(
        &self,
        f: &mut Frame<B>,
        area: Rect,
        data: [&ChartData; W],
        colors: [Color; W],
    ) where
        B: Backend,
    {
        let y_max = data.iter().map(|d| d.y_max()).max().unwrap();
        let datasets: Vec<Dataset> = data
            .iter()
            .enumerate()
            .map(|(i, d)| {
                Dataset::default()
                    .marker(symbols::Marker::Dot)
                    .style(Style::default().fg(colors[i]))
                    .graph_type(GraphType::Line)
                    .data(d.data.as_slice())
            })
            .collect();
        let lb = if (self.counter as u64) < X_WINDOW {
            0
        } else {
            (self.counter as u64) - X_WINDOW
        };
        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(Span::styled(
                        "Messages",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([lb as f64, self.counter]),
            )
            .y_axis(
                Axis::default()
                    .title("Count")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::raw("0"),
                        Span::styled(
                            format!("{}", y_max),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ])
                    .bounds([0.0, y_max as f64 * Y_PADDING]),
            );
        f.render_widget(chart, area);
    }

    fn draw_messages_panel<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.overall, &self.data.ready, &self.data.unacked];
        let colors = [Color::Yellow, Color::Cyan, Color::Red];
        self.draw_chart(f, area, datasets, colors);
    }

    fn draw_message_rates_panel<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let datasets = [&self.data.disk_read_rate, &self.data.disk_write_rate];
        let colors = [Color::Magenta, Color::Green];
        self.draw_chart(f, area, datasets, colors);
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
                        Span::styled(format!("{:<12}", l), Style::default().fg(colors[i])),
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

impl<'a, M> Pane<OverviewPane<'a, M>>
where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self
    where
        M: ManagementClient,
    {
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
        Self {
            content: OverviewPane {
                client,
                counter: 0.,
                data: OverviewData {
                    overall,
                    ready,
                    unacked,
                    disk_read_rate,
                    disk_write_rate,
                },
            },
        }
    }
}

impl<M> Drawable for OverviewPane<'_, M>
where
    M: ManagementClient,
{
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
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
    }

    fn handle_key(&mut self, _key: Key) {}

    fn update(&mut self) {
        let update = self.client.get_overview();
        self.counter += 1.0;
        // TODO MAKE SURE TO REMOVE DUMMY ADDITION
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
