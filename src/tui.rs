use crate::types::*;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use ratatui::Terminal;
use std::io;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Quit,
    Select,
    AddToQueue,
    PlayNext,
    RestartQueue,
    Search,
    TogglePause,
}

pub struct App {
    pub current_view: ViewType,
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
    pub search_results: Vec<SearchResultItem>,
    pub list_state: ListState,
    pub current_artist_id: Option<String>,
    pub current_album_id: Option<String>,
    pub queue: Vec<Song>,
    pub status_message: Option<String>,
    pub status_message_timeout: Option<u64>,
    pub search_string: String,
    pub in_search: bool,
    pub help_open: bool,
    pub current_base_content: String,
    pub current_playback_source: Option<PlaybackSource>,
}

impl App {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            current_view: ViewType::Artists,
            artists: Vec::new(),
            albums: Vec::new(),
            songs: Vec::new(),
            search_results: Vec::new(),
            list_state,
            current_artist_id: None,
            current_album_id: None,
            queue: Vec::new(),
            status_message: None,
            status_message_timeout: None,
            search_string: String::new(),
            in_search: false,
            help_open: false,
            current_base_content: "Artists".to_string(),
            current_playback_source: None,
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.list_state = ListState::default();
        if !items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn next(&mut self) {
        let items_count = match self.current_view {
            ViewType::Artists => self.artists.len(),
            ViewType::Albums => self.albums.len(),
            ViewType::Songs => self.songs.len(),
            ViewType::Search => self.search_results.len(),
        };

        if items_count == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= items_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let items_count = match self.current_view {
            ViewType::Artists => self.artists.len(),
            ViewType::Albums => self.albums.len(),
            ViewType::Songs => self.songs.len(),
            ViewType::Search => self.search_results.len(),
        };

        if items_count == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    items_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn get_selected_index(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn show_message(&mut self, message: String, timeout: u64) {
        self.status_message = Some(message);
        self.status_message_timeout = Some(timeout);
    }

    pub fn clear_message(&mut self) {
        self.status_message = None;
        self.status_message_timeout = None;
    }
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|f| {
            ui(f, app);
        })?;
        Ok(())
    }

    pub fn handle_event(&mut self, app: &mut App) -> Result<Option<Action>> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(handle_key(key, app));
                }
            }
        }
        Ok(None)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();
    }
}

fn handle_key(key: KeyEvent, app: &mut App) -> Option<Action> {
    if app.help_open {
        if matches!(
            key.code,
            KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Esc
        ) {
            app.help_open = false;
        }
        return None;
    }

    if app.in_search {
        match key.code {
            KeyCode::Enter => {
                app.in_search = false;
                return Some(Action::Search);
            }
            KeyCode::Esc => {
                app.in_search = false;
                app.search_string.clear();
            }
            KeyCode::Backspace => {
                app.search_string.pop();
            }
            KeyCode::Char(c) => {
                app.search_string.push(c);
            }
            _ => {}
        }
        return None;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.next();
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
            return Some(Action::Select);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.current_view == ViewType::Artists {
                return Some(Action::Quit);
            } else {
                // Go back
                match app.current_view {
                    ViewType::Albums => {
                        app.current_view = ViewType::Artists;
                        app.current_artist_id = None;
                        app.current_base_content = "Artists".to_string();
                    }
                    ViewType::Songs => {
                        app.current_view = ViewType::Albums;
                        app.current_album_id = None;
                    }
                    ViewType::Search => {
                        app.current_view = ViewType::Artists;
                        app.search_results.clear();
                        app.current_base_content = "Artists".to_string();
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => match app.current_view {
            ViewType::Albums => {
                app.current_view = ViewType::Artists;
                app.current_artist_id = None;
                app.current_base_content = "Artists".to_string();
            }
            ViewType::Songs => {
                app.current_view = ViewType::Albums;
                app.current_album_id = None;
            }
            ViewType::Search => {
                app.current_view = ViewType::Artists;
                app.search_results.clear();
                app.current_base_content = "Artists".to_string();
            }
            _ => {}
        },
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.in_search = true;
            app.search_string.clear();
        }
        KeyCode::Char('a') => {
            return Some(Action::AddToQueue);
        }
        KeyCode::Char('n') => {
            return Some(Action::PlayNext);
        }
        KeyCode::Char('r') => {
            if !app.queue.is_empty() {
                app.queue.remove(0);
                app.show_message("Removed from queue".to_string(), 1500);
            }
        }
        KeyCode::Char('c') => {
            if !app.queue.is_empty() {
                app.queue.clear();
                app.show_message("Queue cleared".to_string(), 1500);
            }
        }
        KeyCode::Char('p') => {
            return Some(Action::RestartQueue);
        }
        KeyCode::Char('?') => {
            app.help_open = true;
        }
        KeyCode::Char(' ') => {
            return Some(Action::TogglePause);
        }
        _ => {}
    }
    None
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    if app.help_open {
        render_help(f, chunks[0]);
        return;
    }

    if app.in_search {
        render_search(f, chunks[0], app);
        return;
    }

    render_list(f, chunks[0], app);
    render_status(f, chunks[1], app);
}

fn render_list(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = match app.current_view {
        ViewType::Artists => app
            .artists
            .iter()
            .map(|a| ListItem::new(a.name.clone()))
            .collect(),
        ViewType::Albums => app
            .albums
            .iter()
            .map(|a| ListItem::new(a.name.clone()))
            .collect(),
        ViewType::Songs => app
            .songs
            .iter()
            .map(|s| ListItem::new(s.title.clone()))
            .collect(),
        ViewType::Search => app
            .search_results
            .iter()
            .map(|r| {
                let text = match r {
                    SearchResultItem::Album { name, artist, .. } => {
                        format!("[A] {} - {}", name, artist)
                    }
                    SearchResultItem::Song { title, artist, .. } => {
                        format!("[S] {} - {}", title, artist)
                    }
                };
                ListItem::new(text)
            })
            .collect(),
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.current_base_content.as_str()),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightBlue))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_status(f: &mut Frame, area: Rect, app: &mut App) {
    let queue_info = if !app.queue.is_empty() {
        format!("Queue: {} ", app.queue.len())
    } else {
        String::new()
    };

    let status_text = if let Some(msg) = &app.status_message {
        format!("{}{}", queue_info, msg)
    } else {
        format!("{}{}", queue_info, app.current_base_content)
    };

    let help_text = "press ? for help";

    let available_width = area.width as usize;
    let left_width = status_text.len();
    let right_width = help_text.len();

    let middle_spaces = if left_width + right_width < available_width {
        available_width - left_width - right_width
    } else {
        1
    };

    let padding = " ".repeat(middle_spaces);

    let status = Paragraph::new(format!("{}{}{}", status_text, padding, help_text))
        .style(Style::default().fg(Color::White).bg(Color::Blue))
        .alignment(Alignment::Left);

    f.render_widget(status, area);
}

fn render_search(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let search_text = format!("Search: {}", app.search_string);
    let search_box = Paragraph::new(search_text)
        .block(Block::default().borders(Borders::ALL).title("Search"))
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    f.render_widget(search_box, chunks[0]);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("Navigation:"),
        Line::from("  ↑/↓ or j/k    - Navigate up/down"),
        Line::from("  →/l or Enter  - Select item / Play song"),
        Line::from("  ←/h           - Go back"),
        Line::from(""),
        Line::from("Search:"),
        Line::from("  / or i         - Open search"),
        Line::from("  Enter          - Execute search"),
        Line::from("  Escape         - Cancel search"),
        Line::from("  Backspace      - Delete character"),
        Line::from(""),
        Line::from("Queue:"),
        Line::from("  a              - Add song to queue"),
        Line::from("  n              - Play next in queue"),
        Line::from("  r              - Remove first from queue"),
        Line::from("  c              - Clear queue"),
        Line::from("  p              - Start/restart queue"),
        Line::from("  space          - Pause/resume playback"),
        Line::from(""),
        Line::from("General:"),
        Line::from("  ?              - Show this help menu"),
        Line::from("  q/Escape       - Quit app"),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .title("Help")
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    let help_paragraph = Paragraph::new(help_text)
        .block(help_block)
        .alignment(Alignment::Left);

    f.render_widget(help_paragraph, area);
}
