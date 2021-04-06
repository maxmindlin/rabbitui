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
const Y_PADDING: u64 = 10;

#[derive(Default)]
struct OverviewData {
    overall: Vec<(f64, f64)>,
    ready: Vec<(f64, f64)>,
    unacked: Vec<(f64, f64)>,
}

impl OverviewData {
    fn update(&mut self, update: Overview) {
        self.overall.push(
            (
                self.overall.last().unwrap().0 + 1.0,
                update.queue_totals.messages as f64,
            )
        );
    }
}

pub struct OverviewPane<'a, M> where
    M: ManagementClient,
{
    dataset: Vec<(f64, f64)>,
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
        let dataset = vec![(0.0, data.queue_totals.messages)];
        let overall = vec![(0.0, data.queue_totals.messages)];
        let ready = vec![(0.0, data.queue_totals.messages_ready)];
        let unacked = vec![(0.0, data.queue_totals.messages_unacked)];
        Self {
            content: OverviewPane {
                dataset,
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
        let data_y_max = self.dataset
            .iter()
            .map(|p| p.1 as u64)
            .max()
            .unwrap();
        let data_x_max = self.dataset
            .iter()
            .map(|p| p.0 as u64)
            .max()
            .unwrap();
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
                .name("Messages")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .data(self.dataset.as_slice()),
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
            );
        f.render_widget(chart, chunks[0]);

    }

    fn handle_key(&mut self, key: Key) {
        
    }

    fn update(&mut self) {
        let update = self.client.get_overview();
        self.dataset.push(
            (
                self.dataset.last().unwrap().0 + 1.0,
                self.dataset.last().unwrap().1 + 1.0,
                // update.queue_totals.messages + 1.0,
            )
        );
    }
}
