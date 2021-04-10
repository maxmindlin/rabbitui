use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType},
    Frame,
};

const X_WINDOW: u64 = 100;
const Y_PADDING: f64 = 1.1;

/// Container for data
/// format that feeds into
/// chart draw.
#[derive(Default)]
pub struct ChartData {
    data: Vec<(f64, f64)>,
    counter: f64,
}

impl ChartData {
    /// pushes to the data vec, but only keeps
    /// as many points as X_WINDOW.
    pub fn push(&mut self, n: f64) {
        self.data.push((self.counter.clone(), n));
        self.counter += 1.0;
        if self.data.len() > X_WINDOW as usize {
            self.data.remove(0);
        }
    }

    pub fn y_max(&self) -> f64 {
        self.data
            .iter()
            .cloned()
            .map(|n| n.1)
            .fold(0. / 0., f64::max)
    }
    pub fn y_min(&self) -> f64 {
        self.data
            .iter()
            .cloned()
            .map(|n| n.1)
            .fold(0. / 0., f64::min)
    }

    pub fn x_max(&self) -> f64 {
        // save an iteration because
        // we know that the x values
        // only increase and that
        // the most recent value is
        // equal to the counter.
        self.counter
    }

    pub fn last_value(&self) -> f64 {
        // this is safe where this is used because we always seed with
        // at least 1 value. If we wanted to be abstract, we would
        // check that data is not empty.
        self.data.iter().last().unwrap().1
    }
}

/// A common wrapper around
/// the chart style for rabbitui.
pub struct RChart<'a, const W: usize> {
    data: [&'a ChartData; W],
    colors: [Color; W],
}

impl<'a, const W: usize> RChart<'a, W> {
    pub fn new(data: [&'a ChartData; W], colors: [Color; W]) -> Self {
        Self { data, colors }
    }

    pub fn draw<B: Backend>(&self, f: &mut Frame<B>, area: Rect) {
        let y_max = self.data.iter().map(|d| d.y_max()).fold(0. / 0., f64::max);
        let y_min = self.data.iter().map(|d| d.y_min()).fold(0. / 0., f64::min);
        let x_max = self.data.iter().map(|d| d.x_max()).fold(0., f64::max);
        let datasets: Vec<Dataset> = self
            .data
            .iter()
            .enumerate()
            .map(|(i, d)| {
                Dataset::default()
                    .marker(symbols::Marker::Dot)
                    .style(Style::default().fg(self.colors[i]))
                    .graph_type(GraphType::Line)
                    .data(d.data.as_slice())
            })
            .collect();
        let lb = if x_max < X_WINDOW as f64 {
            0.
        } else {
            x_max - X_WINDOW as f64
        };
        let chart = Chart::new(datasets)
            .block(Block::default().borders(Borders::ALL))
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([lb, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::styled(
                            format!("{}", y_min as u64),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{}", (y_max * Y_PADDING) as u64),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ])
                    .bounds([0., y_max * Y_PADDING]),
            );
        f.render_widget(chart, area);
    }
}
