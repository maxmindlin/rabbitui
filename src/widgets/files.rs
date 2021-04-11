use crate::{views::centered_rect, DataContainer, Datatable, ManagementClient, Rowable};

use std::path::PathBuf;

use std::fs;

use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

pub struct FileNavigator {
    root: PathBuf,
    file_table: Datatable<PathBuf>,
}

fn file_name_helper(f: &PathBuf) -> &str {
    match f.file_name() {
        Some(n) => n.to_str().unwrap_or(""),
        None => "",
    }
}

fn table_from_path(path: &PathBuf) -> Datatable<PathBuf> {
    let files: Vec<PathBuf> = fs::read_dir(path)
        .unwrap()
        .map(|r| r.unwrap().path())
        .filter(|f| {
            // skip dotfiles.. for now?
            !file_name_helper(f).starts_with(".")
        })
        .collect();
    let empty = files.is_empty();
    let mut d = Datatable::new(files);
    if !empty {
        d.state.select(Some(0));
    }
    d
}

impl Default for FileNavigator {
    fn default() -> Self {
        // TODO this shouldn't fail.. but it could?
        let root = dirs::home_dir().unwrap();
        Self::new(root)
    }
}

impl FileNavigator {
    pub fn new(root: PathBuf) -> Self {
        let file_table = table_from_path(&root);
        Self { root, file_table }
    }

    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        let pop_area = centered_rect(50, 55, area);
        let data = self.file_table.data.get();
        let rows = data.iter().map(|f| {
            let style = if f.is_dir() {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            let literal = file_name_helper(f).to_string();
            let cell = vec![Cell::from(literal).style(style)];
            Row::new(cell).bottom_margin(1)
        });
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let t = Table::new(rows)
            .block(Block::default().borders(Borders::ALL).title("File-Explorer"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[Constraint::Percentage(100)]);
        f.render_widget(Clear, pop_area);
        f.render_stateful_widget(t, pop_area, &mut self.file_table.state);
    }

    pub fn next(&mut self) {
        self.file_table.next();
    }

    pub fn previous(&mut self) {
        self.file_table.previous();
    }

    fn next_table(&mut self, root: PathBuf) {
        self.file_table = table_from_path(&root);
        self.root = root;
    }

    pub fn select_parent(&mut self) {
        if let Some(p) = self.root.parent() {
            let buf = p.to_path_buf();
            self.next_table(buf);
        }
    }

    pub fn select(&mut self) -> Option<PathBuf> {
        if let Some(i) = self.file_table.state.selected() {
            let f = self.file_table.data.get()[i].clone();
            match f.is_file() {
                true => Some(f),
                false => {
                    self.next_table(f);
                    None
                }
            }
        } else {
            None
        }
    }
}
