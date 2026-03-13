mod client;
mod config;
mod events;
mod models;
mod views;
mod widgets;

use client::Client;
use config::AppConfig;
use events::{Event, Events};
use models::{ExchangeBindings, ExchangeInfo, MQMessage, Overview, QueueInfo};
use views::{exchange::ExchangePane, overview::OverviewPane, queues::QueuesPane, StatefulPane};

use std::{
    collections::HashMap,
    error::Error,
    fs,
    io,
    io::Stdout,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use clap::{App as CApp, Arg};
use termion::{
    event::Key,
    input::MouseTerminal,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph, TableState, Tabs, Wrap},
    Frame, Terminal,
};

const DEFAULT_USER: &str = "guest";
const DEFAULT_PASS: &str = "guest";
const DEFAULT_ADDR: &str = "http://localhost:15672";
const ASCII: &str = r#"
   ___       __   __   _ ______     _ 
  / _ \___ _/ /  / /  (_)_  __/_ __(_)
 / , _/ _ `/ _ \/ _ \/ / / / / // / / 
/_/|_|\_,_/_.__/_.__/_/ /_/  \_,_/_/  
                                      
"#;

type TBackend = TermionBackend<AlternateScreen<MouseTerminal<RawTerminal<Stdout>>>>;

/// data access trait for the RabbitMQ
/// Management API. Implemented by any
/// struct used for the app data backend.
pub trait ManagementClient: Send + Sync {
    fn get_exchange_overview(&self) -> Vec<ExchangeInfo>;
    fn get_exchange_bindings(&self, exch: &ExchangeInfo) -> Vec<ExchangeBindings>;
    fn get_overview(&self) -> Overview;
    fn get_queues_info(&self) -> Vec<QueueInfo>;
    fn post_queue_payload(&self, queue_name: String, vhost: &str, payload: String);
    fn pop_queue_item(&self, queue_name: &str, vhost: &str) -> Option<MQMessage>;
    fn ping(&self) -> Result<(), ()>;
    fn purge_queue(&self, queue_name: &str, vhost: &str);
}

pub trait Rowable {
    fn to_row(&self) -> Vec<String>;
}

pub struct TabsState<'a, const L: usize> {
    pub titles: [&'a str; L],
    pub index: usize,
}

impl<'a, const L: usize> TabsState<'a, L> {
    pub fn new(titles: [&'a str; L]) -> Self {
        Self { titles, index: 0 }
    }

    pub fn next(&mut self) {
        self.index = (self.index + 1) % L;
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = L - 1;
        }
    }
}

pub struct DataContainer<T> {
    entries: Vec<T>,
}

impl<T> DataContainer<T> {
    pub fn get(&self) -> &Vec<T> {
        &self.entries
    }

    pub fn get_mut(&mut self) -> &mut Vec<T> {
        &mut self.entries
    }

    pub fn set(&mut self, o: Vec<T>) {
        self.entries = o;
    }
}

/// Stateful container for tabular data. Manages
/// state such as currently selected row, etc.
pub struct Datatable<T> {
    data: DataContainer<T>,
    state: TableState,
}

impl<T> Default for Datatable<T> {
    fn default() -> Self {
        Self {
            data: DataContainer {
                entries: Vec::new(),
            },
            state: TableState::default(),
        }
    }
}

impl<T> Datatable<T> {
    fn new(data: Vec<T>) -> Self {
        Self {
            data: DataContainer { entries: data },
            state: TableState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.data.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.data.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

/// The manager gives us a way to structure the relationship
/// between our tabs and panes. Serves as a middleman between
/// app and panes.
///
/// Also provides nicety guarantees. For example, tabs vs panes
/// should always be 1-1 - const generics here enforce that 1-1 size
/// which makes indexing guaranteed safe (we cannot compile
/// if the tabs index range is different that of our panes).
struct TabsManager<'a, B, const N: usize>
where
    B: Backend,
{
    tabs: TabsState<'a, N>,
    panes: [Box<dyn StatefulPane<B> + 'a>; N],
}

impl<'a, B, const N: usize> TabsManager<'a, B, N>
where
    B: Backend,
{
    pub fn new(tabs: [&'a str; N], panes: [Box<dyn StatefulPane<B> + 'a>; N]) -> Self {
        Self {
            tabs: TabsState::new(tabs),
            panes,
        }
    }

    /// Returns the index that the tabs are at. This will also
    /// correspond to the currently active pane at that
    /// same index.
    pub fn curr(&self) -> usize {
        self.tabs.index
    }

    /// Returns the titles given at initialization. These
    /// are the same as what get drawn into each tab text content.
    pub fn titles(&self) -> &[&'a str; N] {
        &self.tabs.titles
    }

    /// Progres to the next tab. Wraps around
    /// the range upper bound.
    pub fn next(&mut self) {
        self.tabs.next();
    }

    /// Go to the previous tab. Wraps around
    /// the range lower bound.
    pub fn prev(&mut self) {
        self.tabs.previous();
    }

    /// Returns a mutable reference to the currently active
    /// pane.
    pub fn pane(&mut self) -> &mut Box<dyn StatefulPane<B> + 'a> {
        &mut self.panes[self.tabs.index]
    }

    /// Contains the logic for updating all the panes that
    /// "should" be updated upon the state provided by
    /// the panes themselves.
    ///
    /// TODO this isnt really that relevant anymore since
    /// the panes spawn threads that send to an update channel,
    /// so the point of this is pretty minimal now.
    pub fn update(&mut self) {
        self.panes.iter_mut().for_each(|p| p.update());
    }
}

/// The main container for our TUI app. Handles
/// initial setup and highest level state.
struct App<'a, B>
where
    B: Backend,
{
    manager: TabsManager<'a, B, 3>,
}

impl<'a, B> App<'a, B>
where
    B: Backend + 'a,
{
    pub fn new<M: ManagementClient + 'static>(client: Arc<M>, config: AppConfig) -> Self {
        let thread_client = Arc::clone(&client);
        let (overview_tx, overview_rx) = mpsc::channel();
        let (exchange_tx, exchange_rx) = mpsc::channel();
        let (queue_tx, queue_rx) = mpsc::channel();
        // Create data thread. Responsible for gathering new data points
        // and sending to existing receivers.
        thread::spawn(move || loop {
            let overview_data = thread_client.get_overview();
            if overview_tx.send(overview_data).is_err() {
                break;
            }
            let exchange_data = thread_client.get_exchange_overview();
            if exchange_tx.send(exchange_data).is_err() {
                break;
            }
            let queue_data = thread_client.get_queues_info();
            if queue_tx.send(queue_data).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(config.update_rate));
        });
        Self {
            manager: TabsManager::new(
                ["Overview", "Exchanges", "Queues"],
                [
                    Box::new(OverviewPane::new(Arc::clone(&client), overview_rx)),
                    Box::new(ExchangePane::<M>::new(Arc::clone(&client), exchange_rx)),
                    Box::new(QueuesPane::<'a, M>::new(Arc::clone(&client), queue_rx)),
                ],
            ),
        }
    }

    /// The main draw cycle for the app. Draws app-wide
    /// content (headers, tabs, etc.) and then forwards
    /// the reserved pane space to the tab manager for
    /// specific view drawing.
    pub fn draw(&mut self, f: &mut Frame<B>) {
        let chunks = Layout::default()
            .constraints(
                [
                    Constraint::Length(6),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(f.size());
        self.draw_header(f, chunks[0]);
        self.draw_tabs(f, chunks[1]);
        self.manager.pane().draw(f, chunks[2]);
    }

    fn draw_header(&mut self, f: &mut Frame<B>, area: Rect) {
        let text = Text::raw(ASCII);
        let pg_title = Paragraph::new(text)
            .block(Block::default())
            .wrap(Wrap { trim: false });
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(20),
                Constraint::Percentage(5),
            ])
            .split(area);
        let help_t = Text::raw("Press ? for help");
        let p = Paragraph::new(help_t)
            .alignment(Alignment::Right)
            .block(Block::default());
        let meta_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(0)])
            .split(chunks[3]);
        f.render_widget(pg_title, chunks[0]);
        f.render_widget(p, meta_chunks[1]);
    }

    fn draw_tabs(&self, f: &mut Frame<B>, area: Rect) {
        let titles = self
            .manager
            .titles()
            .iter()
            .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
            .collect();
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .highlight_style(Style::default().fg(Color::Yellow))
            .select(self.manager.curr());
        f.render_widget(tabs, area);
    }

    /// Transforms key inputs into app specific behavior. App itself
    /// reserves certain keys that will be used across the app,
    /// regardless of active view. Any other keys are passed off
    /// to the tab manager.
    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('l') => {
                self.manager.next();
            }
            Key::Char('h') => {
                self.manager.prev();
            }
            _ => {
                self.manager.pane().handle_key(key);
            }
        }
    }

    /// Handles tick updates. Most cases are just passed
    /// to the tab manager to determine if individual panes
    /// need updated.
    fn update(&mut self) {
        self.manager.update();
    }
}

/// Load baseline delivered counts from a file.
/// Format: one "queue_name=count" per line.
fn load_baseline(path: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            if let Some((name, val)) = line.split_once('=') {
                if let Ok(n) = val.parse::<u64>() {
                    map.insert(name.to_string(), n);
                }
            }
        }
    }
    map
}

/// Save current delivered counts as baseline.
fn save_baseline(path: &str, queues: &[QueueInfo]) {
    let lines: Vec<String> = queues
        .iter()
        .map(|q| format!("{}={}", q.name, q.message_stats.deliver_get))
        .collect();
    let _ = fs::write(path, lines.join("\n"));
}

/// Prints a row for a known queue.
fn print_queue_row(q: &QueueInfo, name_w: usize, baseline: &HashMap<String, u64>) {
    let in_rate = q.message_stats.publish_details.rate;
    let out_rate = q.message_stats.deliver_get_details.rate;
    let base = baseline.get(&q.name).copied().unwrap_or(0);
    let delivered = q.message_stats.deliver_get.saturating_sub(base);
    println!(
        "  {:<nw$}  {:>5}  {:>6}  {:>5}  {:>9}  {:>4}  {:>8.1}  {:>8.1}",
        q.name, q.ready, q.unacked, q.total,
        delivered, q.consumers, in_rate, out_rate,
        nw = name_w
    );
}

/// Prints a placeholder row for a queue not found in RabbitMQ.
fn print_missing_row(name: &str, name_w: usize) {
    println!(
        "  {:<nw$}  {:>5}  {:>6}  {:>5}  {:>9}  {:>4}  {:>8}  {:>8}",
        name, "-", "-", "-", "-", "-", "-", "-",
        nw = name_w
    );
}

/// Prints the table header.
fn print_snapshot_header(name_w: usize) {
    println!(
        "  {:<nw$}  {:>5}  {:>6}  {:>5}  {:>9}  {:>4}  {:>8}  {:>8}",
        "QUEUE", "READY", "UNACKD", "TOTAL", "DELIVERED", "CONS", "IN/s", "OUT/s",
        nw = name_w
    );
    println!(
        "  {:<nw$}  {:>5}  {:>6}  {:>5}  {:>9}  {:>4}  {:>8}  {:>8}",
        "-".repeat(name_w), "-----", "------", "-----", "---------", "----", "--------", "--------",
        nw = name_w
    );
}

/// Print a one-shot table of queue states to stdout and exit.
/// If baseline_path is provided, on first run (file missing) saves current
/// delivered counts and shows 0. On subsequent runs shows delta from baseline.
fn print_snapshot(client: &Client, filter: Option<&str>, baseline_path: Option<&str>) {
    let queues = client.get_queues_info();

    // Handle baseline: load or create
    let baseline = if let Some(path) = baseline_path {
        let existing = load_baseline(path);
        if existing.is_empty() {
            // First call — save current state as baseline
            save_baseline(path, &queues);
            // Return current counts so delivered shows as 0
            queues
                .iter()
                .map(|q| (q.name.clone(), q.message_stats.deliver_get))
                .collect()
        } else {
            existing
        }
    } else {
        HashMap::new()
    };

    if let Some(f) = filter {
        let names: Vec<&str> = f.split(',').map(|s| s.trim()).collect();
        let name_w = names.iter().map(|n| n.len()).max().unwrap_or(5).max(5);
        print_snapshot_header(name_w);
        for name in &names {
            if let Some(q) = queues.iter().find(|q| q.name == *name) {
                print_queue_row(q, name_w, &baseline);
            } else {
                print_missing_row(name, name_w);
            }
        }
    } else {
        if queues.is_empty() {
            println!("  (no queues found)");
            return;
        }
        let mut sorted: Vec<&QueueInfo> = queues.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        let name_w = sorted.iter().map(|q| q.name.len()).max().unwrap_or(5).max(5);
        print_snapshot_header(name_w);
        for q in &sorted {
            print_queue_row(q, name_w, &baseline);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = CApp::new("RabbiTui")
        .version("0.1.0")
        .author("Max Mindlin <maxmindlin@gmail.com>")
        .about("A TUI application for RabbitMQ management")
        .arg(
            Arg::new("user")
                .about("Username for the API auth")
                .takes_value(true)
                .short('u')
                .long("user")
                .required(false)
                .default_value(DEFAULT_USER),
        )
        .arg(
            Arg::new("pass")
                .about("Password for the API auth")
                .takes_value(true)
                .short('p')
                .long("pass")
                .required(false)
                .default_value(DEFAULT_PASS),
        )
        .arg(
            Arg::new("addr")
                .about("Http(s) address of the API. Excludes trailing slash")
                .takes_value(true)
                .short('a')
                .long("addr")
                .required(false)
                .default_value(DEFAULT_ADDR),
        )
        .arg(
            Arg::new("snapshot")
                .about("Print queue status table to stdout and exit (non-interactive)")
                .long("snapshot")
                .short('s')
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::new("filter")
                .about("Comma-separated queue names to show in snapshot mode")
                .long("filter")
                .short('f')
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::new("baseline")
                .about("File to store baseline delivered counts. DELIVERED column shows delta from baseline. File is created on first call.")
                .long("baseline")
                .short('b')
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let user = matches.value_of("user").unwrap();
    let pass = matches.value_of("pass").unwrap();
    let addr = matches.value_of("addr").unwrap();
    let c = Client::new(addr, user, Some(pass.to_string()));
    if let Err(_) = c.ping() {
        println!("Unable to ping RabbitMQ API.");
        println!("Check that the service is running and that creds are correct.");
        return Ok(());
    }

    // Snapshot mode: print table and exit without TUI setup
    if matches.is_present("snapshot") {
        let filter = matches.value_of("filter");
        let baseline = matches.value_of("baseline");
        print_snapshot(&c, filter, baseline);
        return Ok(());
    }

    // TODO allow override this.
    let config = AppConfig::default();
    let mut app = App::<TBackend>::new::<Client>(Arc::new(c), config);
    // TODO support different backend for non-MacOs.
    // Just need to swap out Termion based upon some config or compile setting.
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let events = Events::new();

    loop {
        terminal.draw(|f| app.draw(f))?;

        match events.next()? {
            Event::Input(key) => match key {
                Key::Char('q') => {
                    break;
                }
                _ => {
                    app.handle_key(key);
                }
            },
            Event::Tick => {
                app.update();
            }
        }
    }
    Ok(())
}
