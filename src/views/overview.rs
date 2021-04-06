use super::{Pane, Drawable};
use crate::{Datatable, Rowable, ManagementClient};
use crate::models::Overview;

use termion::event::Key;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Terminal,
    Frame,
};

const X_WINDOW: u64 = 100;
const Y_PADDING: u64 = 5;

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
            self.data.drain(..1);
        }
    }

    fn x_max(&self) -> u64 {
        self.data
            .iter()
            .map(|p| p.0 as u64)
            .max()
            .unwrap()
    }

    fn y_max(&self) -> u64 {
        self.data
            .iter()
            .map(|p| p.1 as u64)
            .max()
            .unwrap()
    }
}

#[derive(Default)]
struct OverviewData {
    overall: ChartData,
    ready: ChartData,
    unacked: ChartData,
}

pub struct OverviewPane<'a, M> where
    M: ManagementClient,
{
    data: OverviewData,
    client: &'a M,
}

impl<'a, M> Pane<OverviewPane<'a, M>> where
    M: ManagementClient,
{
    pub fn new(client: &'a M) -> Self where
        M: ManagementClient,
    {
        let data = client.get_overview();
        let mut overall = ChartData::default();
        overall.push(data.queue_totals.messages);
        let mut ready = ChartData::default();
        ready.push(data.queue_totals.messages_ready);
        let mut unacked = ChartData::default();
        unacked.push(data.queue_totals.messages_unacked);
        Self {
            content: OverviewPane {
                client,
                data: OverviewData {
                    overall,
                    ready,
                    unacked,
                },
            }
        }
    }
}

impl<'a, M> Drawable for OverviewPane<'a, M> where
    M: ManagementClient,
{
    fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let data_y_maxes = vec![
            self.data.overall.y_max(),
            self.data.ready.y_max(),
            self.data.unacked.y_max(),
        ];
        let data_y_max = data_y_maxes.iter()
            .max()
            .unwrap();
        // let data_y_max = self.data.ready.y_max();
        // all should have the same
        let data_x_max = self.data.ready.x_max();
        let y_max = data_y_max + Y_PADDING;
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Ratio(1, 2),
                    Constraint::Ratio(1, 2),
                ]
                .as_ref(),
            )
            .split(area);
        let datasets = vec![
            Dataset::default()
                .name("Ready")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Yellow))
                .graph_type(GraphType::Line)
                .data(self.data.ready.data.as_slice()),
            Dataset::default()
                .name("Total")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Cyan))
                .graph_type(GraphType::Line)
                .data(self.data.overall.data.as_slice()),
            Dataset::default()
                .name("Unacked")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Red))
                .graph_type(GraphType::Line)
                .data(self.data.unacked.data.as_slice()),
        ];
        let lb = if data_x_max < X_WINDOW {
            0
        } else {
            data_x_max - X_WINDOW
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
                    .bounds([lb as f64, data_x_max as f64])
            )
            .y_axis(
                Axis::default()
                    .title("Count")
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::raw("0"),
                        Span::styled(format!("{}", y_max), Style::default().add_modifier(Modifier::BOLD)),
                    ])
                    .bounds([0.0, y_max as f64]),
            )
            .hidden_legend_constraints(
                (
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(3, 4)
                )
            );
        f.render_widget(chart, chunks[0]);

    }

    fn handle_key(&mut self, key: Key) {
        
    }

    fn update(&mut self) {
        let update = self.client.get_overview();
        self.data.ready.push(update.queue_totals.messages_ready + 1.0);
        self.data.overall.push(update.queue_totals.messages + 2.0);
        self.data.unacked.push(update.queue_totals.messages_unacked + 4.0);
    }
}
