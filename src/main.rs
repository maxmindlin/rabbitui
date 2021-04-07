mod client;
mod events;
mod models;
mod views;
mod widgets;

use client::Client;
use events::{Event, Events};
use models::{ExchangeBindings, ExchangeInfo, MQMessage, Overview, QueueInfo};
use views::exchange::ExchangePane;
use views::overview::OverviewPane;
use views::queues::QueuesPane;
use views::{Drawable, Pane};

use std::{error::Error, io};

use clap::{App as CApp, Arg};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::{Backend, TermionBackend},
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
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

pub trait ManagementClient {
    fn get_exchange_overview(&self) -> Vec<ExchangeInfo>;
    fn get_exchange_bindings(&self, exch: &ExchangeInfo) -> Vec<ExchangeBindings>;
    fn get_overview(&self) -> Overview;
    fn get_queues_info(&self) -> Vec<QueueInfo>;
    fn post_queue_payload(&self, queue_name: String, vhost: &str, payload: String);
    fn pop_queue_item(&self, queue_name: &str, vhost: &str) -> Option<MQMessage>;
    fn ping(&self) -> Result<(), ()>;
}

pub trait Rowable {
    fn to_row(&self) -> Vec<String>;
}

// taken from
// https://github.com/fdehau/tui-rs/blob/25ff2e5e61f8902101e485743992db2412f77aad/examples/util/mod.rs
pub struct TabsState<'a> {
    pub titles: Vec<&'a str>,
    pub index: usize,
}

impl<'a> TabsState<'a> {
    pub fn new(titles: Vec<&'a str>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

pub struct DataContainer<T> {
    entries: Vec<T>,
    staleness: usize,
}

impl<T> DataContainer<T> {
    pub fn get(&self) -> &Vec<T> {
        &self.entries
    }

    pub fn get_mut(&mut self) -> &mut Vec<T> {
        &mut self.entries
    }

    pub fn is_stale(&self) -> bool {
        self.staleness >= 10
    }

    pub fn set(&mut self, o: Vec<T>) {
        self.entries = o;
    }
}

pub struct Datatable<T> {
    data: DataContainer<T>,
    state: TableState,
}

impl<T> Default for Datatable<T> {
    fn default() -> Self {
        Self {
            data: DataContainer {
                entries: Vec::new(),
                staleness: 0,
            },
            state: TableState::default(),
        }
    }
}

impl<T> Datatable<T> {
    fn new(data: Vec<T>) -> Self {
        Self {
            data: DataContainer {
                entries: data,
                staleness: 0,
            },
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

struct App<'a, M>
where
    M: ManagementClient,
{
    client: &'a M,
    tabs: TabsState<'a>,
    exch_pane: Pane<ExchangePane<'a, M>>,
    overview_pane: Pane<OverviewPane<'a, M>>,
    queues_pane: Pane<QueuesPane<'a, M>>,
}

impl<'a, M> App<'a, M>
where
    M: ManagementClient,
{
    fn new(client: &'a M) -> Self {
        Self {
            client: &client,
            tabs: TabsState::new(vec!["Overview", "Exchanges", "Queues"]),
            exch_pane: Pane::<ExchangePane<'a, M>>::new(&client),
            overview_pane: Pane::<OverviewPane<'a, M>>::new(&client),
            queues_pane: Pane::<QueuesPane<'a, M>>::new(&client),
        }
    }

    fn draw_header<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
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
            .constraints([
                Constraint::Percentage(50),
                Constraint::Min(0),
            ])
            .split(chunks[3]);
        f.render_widget(pg_title, chunks[0]);
        f.render_widget(p, meta_chunks[1]);
    }

    fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
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
        let titles = self
            .tabs
            .titles
            .iter()
            .map(|t| Spans::from(Span::styled(*t, Style::default().fg(Color::Green))))
            .collect();
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Tabs"))
            .highlight_style(Style::default().fg(Color::Yellow))
            .select(self.tabs.index);
        self.draw_header(f, chunks[0]);
        f.render_widget(tabs, chunks[1]);
        match self.tabs.index {
            0 => self.overview_pane.content.draw(f, chunks[2]),
            1 => self.exch_pane.content.draw(f, chunks[2]),
            2 => self.queues_pane.content.draw(f, chunks[2]),
            _ => unreachable!(),
        }
    }

    fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('l') => {
                self.tabs.next();
            }
            Key::Char('h') => {
                self.tabs.previous();
            }
            _ => match self.tabs.index {
                0 => self.overview_pane.content.handle_key(key),
                1 => self.exch_pane.content.handle_key(key),
                2 => self.queues_pane.content.handle_key(key),
                _ => unreachable!(),
            },
        }
    }

    fn update(&mut self) {
        // TODO some tabs might not need constant updating.
        // It makes sense for graphs to, but perhaps not tables.
        // Panes can have their own knowledge and control around updates,
        // this could be way a way to just ferry ticks to the panes.

        // Always send a tick update to overview graphs.
        self.overview_pane.content.update();
        match self.tabs.index {
            1 => self.exch_pane.content.update(),
            2 => self.queues_pane.content.update(),
            _ => {}
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
    let mut app = App::<Client>::new(&c);
    // TODO support different backend for non-MacOs.
    // Just need to swap out Termion based upon some config setting.
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let events = Events::new();

    loop {
        terminal.draw(|f| {
            app.draw(f);
        })?;

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
