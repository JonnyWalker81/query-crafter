use std::{
  collections::{BTreeMap, HashMap},
  fmt::Display,
  fs,
  path::PathBuf,
  rc::Rc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use clipboard::{ClipboardContext, ClipboardProvider};
use chrono;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use directories::ProjectDirs;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Postgres, Row};
use strum::Display;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tui_textarea::{Input, TextArea};

use super::{Component, ComponentKind, Frame};
use crate::{
  action::Action,
  components::vim::Vim,
  config::{Config, KeyBindings},
  editor_common::{Mode, Transition},
  editor_component::EditorComponent,
};

const VISIBLE_COLUMNS: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryHistoryEntry {
  pub query: String,
  pub timestamp: u64,
  pub success: bool,
}

impl QueryHistoryEntry {
  pub fn new(query: String, success: bool) -> Self {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_secs();
    Self { query, timestamp, success }
  }

  pub fn formatted_time(&self) -> String {
    // Convert timestamp to readable format
    let datetime = UNIX_EPOCH + Duration::from_secs(self.timestamp);
    format!("{:?}", datetime)
      .split_once('.')
      .map(|(date_time, _)| date_time.replace("SystemTime { tv_sec: ", "").replace(", tv_nsec: 0 }", ""))
      .unwrap_or_else(|| "Unknown".to_string())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DbTable {
  pub name: String,
  pub schema: String,
}

#[derive(Default)]
pub struct Db<'a> {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  tables: Vec<DbTable>,
  selected_table_index: usize,
  selected_row_index: usize,
  selected_headers: Vec<String>,
  query_results: Vec<Vec<String>>,
  selected_component: ComponentKind,
  query_input: TextArea<'a>,
  vim_editor: Vim,
  horizonal_scroll_offset: usize,
  show_row_details: bool,
  table_search_query: String,
  is_searching_tables: bool,
  row_is_selected: bool,
  detail_row_index: usize,
  error_message: Option<String>,
  // Query history functionality
  query_history: Vec<QueryHistoryEntry>,
  selected_history_index: usize,
  selected_tab: usize, // 0 = Query, 1 = History
  history_file_path: PathBuf,
  last_executed_query: Option<String>, // Track last query for history saving
}

impl<'a> Db<'a> {
  pub fn new() -> Self {
    let mut instance = Self::default();
    
    // Initialize history file path in user's config directory
    if let Some(proj_dirs) = ProjectDirs::from("com", "query-crafter", "query-crafter") {
      instance.history_file_path = proj_dirs.config_dir().join("query_history.json");
    } else {
      // Fallback to current directory if config dir not available
      instance.history_file_path = PathBuf::from("query_history.json");
    }
    
    // Load existing history
    instance.load_query_history();
    
    instance
  }

  fn get_history_file_path(&self) -> &PathBuf {
    &self.history_file_path
  }

  fn load_query_history(&mut self) {
    if let Ok(contents) = fs::read_to_string(&self.history_file_path) {
      if let Ok(history) = serde_json::from_str::<Vec<QueryHistoryEntry>>(&contents) {
        self.query_history = history;
      }
    }
  }

  fn save_query_history(&self) {
    // Ensure directory exists
    if let Some(parent) = self.history_file_path.parent() {
      let _ = fs::create_dir_all(parent);
    }
    
    // Save history to file
    if let Ok(json) = serde_json::to_string_pretty(&self.query_history) {
      let _ = fs::write(&self.history_file_path, json);
    }
  }

  fn add_to_history(&mut self, query: &str, success: bool) {
    let query = query.trim().to_string();
    
    // Don't add empty queries
    if query.is_empty() {
      return;
    }
    
    // Only add successful queries to history
    if !success {
      return;
    }
    
    // Check if this exact query already exists in recent history (last 10 entries)
    let recent_limit = 10.min(self.query_history.len());
    let recent_queries = &self.query_history[self.query_history.len().saturating_sub(recent_limit)..];
    
    if !recent_queries.iter().any(|entry| entry.query == query) {
      let entry = QueryHistoryEntry::new(query, success);
      self.query_history.push(entry);
      
      // Keep only last 100 entries to prevent unlimited growth
      if self.query_history.len() > 100 {
        self.query_history.drain(0..self.query_history.len() - 100);
      }
      
      self.save_query_history();
    }
  }

  fn column_count(&self) -> usize {
    self.selected_headers.len()
  }

  fn json(&self) -> Option<String> {
    if self.query_results.is_empty() {
      return None;
    }

    let json_str = if self.row_is_selected {
      if let Some(selected_row) = self.query_results.get(self.selected_row_index) {
        if let Some(selected_cell) = selected_row.get(self.detail_row_index) {
          selected_cell.to_string()
        } else {
          String::new()
        }
      } else {
        String::new()
      }
    } else {
      let row_data = self.query_results[self.selected_row_index].iter().zip(self.selected_headers.iter()).fold(
        BTreeMap::new(),
        |mut acc, (value, header)| {
          acc.insert(header, value);
          acc
        },
      );

      serde_json::to_string_pretty(&row_data).unwrap()
    };

    Some(json_str)
  }

  fn table_row_count(&self) -> usize {
    self.tables.len()
  }

  fn render_table_list(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let table_chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
      .split(chunks[1]);

    let tables_border_color = if self.selected_component == ComponentKind::Home { Color::Cyan } else { Color::White };
    let tables = Block::default()
      .borders(Borders::ALL)
      .style(Style::default().fg(tables_border_color))
      .title("Tables")
      .border_type(BorderType::Plain);

    let table_list_chunks = if self.is_searching_tables {
      Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(table_chunks[0])
    } else {
      table_chunks.clone()
    };

    if self.is_searching_tables {
      let search_block = Block::default().borders(Borders::ALL).title("Search");
      let search_text =
        Paragraph::new(Text::styled(self.table_search_query.to_string(), Style::default().fg(Color::Yellow)))
          .block(search_block);
      f.render_widget(search_text, table_list_chunks[0]);
    }

    let table_render_chunk = if self.is_searching_tables { table_list_chunks[1] } else { table_list_chunks[0] };

    let mut table_list_state = ListState::default();
    table_list_state.select(Some(self.selected_table_index));
    let items: Vec<ListItem> = self.tables.iter().map(|t| ListItem::new(t.name.to_string())).collect();

    let list = List::new(items)
      .block(tables)
      .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, table_render_chunk, &mut table_list_state);

    Ok(table_chunks)
  }

  fn render_query_input(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let query_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Length(2), Constraint::Length(6), Constraint::Min(1)].as_ref())
      .split(chunks[1]);

    let query_border_color = if self.selected_component == ComponentKind::Query { Color::Cyan } else { Color::White };
    let border_style = Style::default().fg(query_border_color);

    // Render tabs
    let tabs = Tabs::new(["Query", "History"])
      .style(Style::new().white())
      .highlight_style(Style::new().yellow().underlined())
      .select(self.selected_tab)
      .padding("", "")
      .divider(" ");
    f.render_widget(tabs, query_chunks[0]);

    // Render content based on selected tab
    match self.selected_tab {
      0 => {
        // Query tab - show the text editor
        let input_block = Block::default().borders(Borders::ALL).border_style(border_style).title("Query");
        self.query_input.set_block(input_block);
        f.render_widget(&self.query_input, query_chunks[1]);
      },
      1 => {
        // History tab - show the history list
        self.render_history_list(f, query_chunks[1], query_border_color)?;
      },
      _ => {
        // Default to query tab
        let input_block = Block::default().borders(Borders::ALL).border_style(border_style).title("Query");
        self.query_input.set_block(input_block);
        f.render_widget(&self.query_input, query_chunks[1]);
      }
    }

    Ok(query_chunks)
  }

  fn render_history_list(&mut self, f: &mut Frame<'_>, area: Rect, border_color: Color) -> Result<()> {
    let history_block = Block::default()
      .borders(Borders::ALL)
      .border_style(Style::default().fg(border_color))
      .title("Query History");

    if self.query_history.is_empty() {
      let empty_msg = Paragraph::new("No query history available")
        .block(history_block)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
      f.render_widget(empty_msg, area);
      return Ok(());
    }

    // Create list items from history (most recent first)
    let items: Vec<ListItem> = self.query_history
      .iter()
      .rev() // Show most recent first
      .enumerate()
      .map(|(idx, entry)| {
        let truncated_query = if entry.query.len() > 60 {
          format!("{}...", &entry.query[..57])
        } else {
          entry.query.clone()
        };
        
        let time_str = chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
          .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
          .unwrap_or_else(|| "Unknown".to_string());

        let display_text = format!("{} | {}", time_str, truncated_query);
        
        ListItem::new(display_text)
          .style(if entry.success { 
            Style::default().fg(Color::Green) 
          } else { 
            Style::default().fg(Color::Red) 
          })
      })
      .collect();

    let mut list_state = ListState::default();
    // Convert reverse index back to forward index for selection
    if !self.query_history.is_empty() {
      let reverse_index = self.query_history.len().saturating_sub(1).saturating_sub(self.selected_history_index);
      list_state.select(Some(reverse_index));
    }

    let history_list = List::new(items)
      .block(history_block)
      .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(history_list, area, &mut list_state);

    Ok(())
  }

  fn render_query_result_details(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(chunks[2]);

    if let Some(selected_row) = self.query_results.get(self.selected_row_index) {
      let normal_style = Style::default();
      let header_cells = ["Name", "value"]
        .iter()
        .map(|h| Cell::from(h.to_string()).style(Style::default().fg(Color::Red).bg(Color::Green)));
      let header = ratatui::widgets::Row::new(header_cells).style(normal_style).height(1);

      let rows = selected_row
        .iter()
        .zip(self.selected_headers.iter())
        .map(|(c, r)| {
          let cells = [Cell::from(r.to_string()), Cell::from(c.to_string())];
          ratatui::widgets::Row::new(cells).height(1).bottom_margin(1)
        })
        .collect::<Vec<_>>();

      let status_text =
        Paragraph::new(Text::styled(format!("Rows: {}", rows.len()), Style::default().fg(Color::Yellow)));
      f.render_widget(status_text, table_chunks[1]);

      let results_border_color =
        if self.selected_component == ComponentKind::Results { Color::Cyan } else { Color::White };
      let mut table_state = TableState::default();
      table_state.select(Some(self.detail_row_index));
      let result_table = Table::default()
        .rows(rows)
        .header(header)
        .column_spacing(10)
        .block(
          Block::default()
            .borders(Borders::ALL)
            .title("Results")
            .fg(results_border_color)
            .border_type(BorderType::Plain),
        )
        .highlight_symbol(">>")
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
        .widths([Constraint::Length(40), Constraint::Length(40), Constraint::Length(40)]);

      f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);
    }

    Ok(chunks)
  }

  fn render_query_results(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    if self.row_is_selected {
      self.render_query_result_details(f, chunks)
    } else {
      self.render_query_results_table(f, chunks)
    }
  }

  fn render_query_results_table(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(chunks[2]);

    let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;
    let normal_style = Style::default();
    let header_cells = self
      .selected_headers
      .iter()
      .skip(skip_count)
      .take(VISIBLE_COLUMNS)
      .map(|h| Cell::from(h.to_string()).style(Style::default().fg(Color::Red).bg(Color::Green)));
    let header = ratatui::widgets::Row::new(header_cells).style(normal_style).height(1);

    let rows = self
      .query_results
      .iter()
      .map(|r| {
        let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| Cell::from(c.to_string()));
        ratatui::widgets::Row::new(cells).height(1).bottom_margin(1)
      })
      .collect::<Vec<_>>();

    let status_text = Paragraph::new(Text::styled(format!("Rows: {}", rows.len()), Style::default().fg(Color::Yellow)));
    f.render_widget(status_text, table_chunks[1]);

    let results_border_color =
      if self.selected_component == ComponentKind::Results { Color::Cyan } else { Color::White };
    let mut table_state = TableState::default();
    table_state.select(Some(self.selected_row_index));
    let result_table = Table::default()
      .rows(rows)
      .header(header)
      .column_spacing(10)
      .block(
        Block::default().borders(Borders::ALL).title("Results").fg(results_border_color).border_type(BorderType::Plain),
      )
      .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
      .widths([Constraint::Length(40), Constraint::Length(40), Constraint::Length(40)]);

    f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);

    if self.show_row_details {
      if let Some(json_str) = self.json() {
        let area = self.centered_rect(80, 60, f.area());
        let block = Block::default().title("Row Details").borders(Borders::ALL);
        let paragraph = Paragraph::new(json_str.as_str()).block(block).wrap(Wrap { trim: true });
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
      }
    }

    Ok(chunks)
  }

  fn render_error(&mut self, f: &mut Frame<'_>) -> Result<()> {
    if let Some(error_message) = &self.error_message {
      let area = self.centered_rect(60, 20, f.area());
      let block = Block::default().title("Error").borders(Borders::ALL).border_style(Style::default().fg(Color::Red));
      let paragraph = Paragraph::new(error_message.as_str()).block(block).wrap(Wrap { trim: true });
      f.render_widget(Clear, area);
      f.render_widget(paragraph, area);
    }

    Ok(())
  }

  fn toggle_comment_on_textarea(&mut self) {
    let cursor_row = self.query_input.cursor().0;
    let lines = self.query_input.lines().to_vec(); // Clone to avoid borrowing issues

    if cursor_row >= lines.len() {
      return;
    }

    let current_line = &lines[cursor_row];
    let trimmed = current_line.trim_start();

    // Use SQL comment style for database queries
    let comment_prefix = "--";

    // Check if line is already commented
    let is_commented = trimmed.starts_with(comment_prefix);
    let leading_whitespace = current_line.len() - trimmed.len();

    // Move to beginning of line
    self.query_input.move_cursor(tui_textarea::CursorMove::Head);

    if is_commented {
      // Uncomment: find and remove comment prefix
      for _ in 0..leading_whitespace {
        self.query_input.move_cursor(tui_textarea::CursorMove::Forward);
      }
      // Delete the comment prefix
      for _ in 0..comment_prefix.len() {
        self.query_input.delete_next_char();
      }
      // Remove space after comment if present
      let current_lines = self.query_input.lines().to_vec();
      if let Some(updated_line) = current_lines.get(cursor_row) {
        if let Some(first_char) = updated_line.chars().nth(self.query_input.cursor().1) {
          if first_char == ' ' {
            self.query_input.delete_next_char();
          }
        }
      }
    } else {
      // Comment: add comment prefix
      for _ in 0..leading_whitespace {
        self.query_input.move_cursor(tui_textarea::CursorMove::Forward);
      }
      // Insert comment prefix with space
      for c in format!("{comment_prefix} ").chars() {
        self.query_input.insert_char(c);
      }
    }
  }

  fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
      ])
      .split(r);

    Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
      ])
      .split(popup_layout[1])[1]
  }
}

impl<'a> Component for Db<'a> {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  fn init(&mut self, _area: ratatui::layout::Rect) -> Result<()> {
    Ok(())
  }

  fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
    if let Some(crate::tui::Event::Key(key)) = event {
      self.handle_key_events(key)
    } else {
      Ok(None)
    }
  }

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    match self.selected_component {
      ComponentKind::Home => {
        // Searching for a table
        match key.code {
          KeyCode::Char(c) => {
            if c == '/' {
              self.is_searching_tables = true;
            }

            if self.is_searching_tables && c != '/' {
              self.table_search_query.push(c);
              return Ok(Some(Action::LoadTables(self.table_search_query.clone())));
            }
          },
          KeyCode::Enter => {
            if self.is_searching_tables {
              self.is_searching_tables = false;
            }
          },
          KeyCode::Backspace => {
            self.table_search_query.pop();
          },
          KeyCode::Esc => {
            self.table_search_query.clear();
            if !self.is_searching_tables {
              return Ok(Some(Action::LoadTables(String::new())));
            } else {
              self.is_searching_tables = false;
            }
          },
          _ => {},
        }
      },
      ComponentKind::Query => {
        // Handle tab switching first (works in all modes)
        match key.code {
          KeyCode::Char('Q') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
            self.selected_tab = 0; // Switch to Query tab
            return Ok(None);
          },
          KeyCode::Char('H') if key.modifiers.contains(KeyModifiers::CONTROL) && key.modifiers.contains(KeyModifiers::SHIFT) => {
            self.selected_tab = 1; // Switch to History tab
            return Ok(None);
          },
          KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            self.selected_tab = if self.selected_tab > 0 { self.selected_tab - 1 } else { 1 };
            return Ok(None);
          },
          KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            self.selected_tab = if self.selected_tab < 1 { self.selected_tab + 1 } else { 0 };
            return Ok(None);
          },
          _ => {}
        }

        // Handle history tab navigation
        if self.selected_tab == 1 {
          match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
              if self.selected_history_index > 0 {
                self.selected_history_index -= 1;
              }
              return Ok(None);
            },
            KeyCode::Down | KeyCode::Char('j') => {
              if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() - 1 {
                self.selected_history_index += 1;
              }
              return Ok(None);
            },
            KeyCode::Enter => {
              // Execute selected history item
              if let Some(entry) = self.query_history.get(self.query_history.len().saturating_sub(1).saturating_sub(self.selected_history_index)) {
                return Ok(Some(Action::HandleQuery(entry.query.clone())));
              }
              return Ok(None);
            },
            KeyCode::Char('c') => {
              // Copy selected history item to query editor
              if let Some(entry) = self.query_history.get(self.query_history.len().saturating_sub(1).saturating_sub(self.selected_history_index)) {
                self.query_input.select_all();
                self.query_input.cut();
                self.query_input.insert_str(&entry.query);
                self.selected_tab = 0; // Switch back to query tab
              }
              return Ok(None);
            },
            KeyCode::Char('d') => {
              // Delete selected history item
              if !self.query_history.is_empty() {
                let actual_index = self.query_history.len().saturating_sub(1).saturating_sub(self.selected_history_index);
                self.query_history.remove(actual_index);
                self.save_query_history();
                // Adjust selection if needed
                if self.selected_history_index >= self.query_history.len() && self.selected_history_index > 0 {
                  self.selected_history_index = self.query_history.len().saturating_sub(1);
                }
              }
              return Ok(None);
            },
            _ => return Ok(None),
          }
        }

        // Handle query editor (only when on Query tab)
        if self.selected_tab == 0 {
          // Handle different modes differently
          match self.vim_editor.mode() {
            Mode::Insert => {
              // In insert mode, most keys go directly to textarea except Esc and Ctrl+Enter
              match key.code {
                KeyCode::Esc => {
                  self.vim_editor = Vim::new(Mode::Normal);
                  self.query_input.set_cursor_style(Mode::Normal.cursor_style());
                },
                _ => {
                  // Pass all other keys to textarea for normal text input
                  let input = Input::from(key);
                  self.query_input.input(input);
                },
              }
            },
            _ => {
              // In Normal/Visual/Operator modes, handle VIM commands
              let input = Input::from(key);

              // Handle 'g' operator mode for gcc commenting
              if self.vim_editor.mode() == Mode::Operator('g') && key.code == KeyCode::Char('c') {
                self.toggle_comment_on_textarea();
                self.vim_editor = Vim::new(Mode::Normal);
                self.query_input.set_cursor_style(Mode::Normal.cursor_style());
                return Ok(None);
              }

              // Apply VIM operations directly to query_input textarea
              match key.code {
                // Handle visual mode
                KeyCode::Char('v') if self.vim_editor.mode() == Mode::Normal => {
                self.query_input.start_selection();
                self.vim_editor = Vim::new(Mode::Visual);
                self.query_input.set_cursor_style(Mode::Visual.cursor_style());
              },
                KeyCode::Char('V') if self.vim_editor.mode() == Mode::Normal => {
                  self.query_input.move_cursor(tui_textarea::CursorMove::Head);
                  self.query_input.start_selection();
                  self.query_input.move_cursor(tui_textarea::CursorMove::End);
                  self.vim_editor = Vim::new(Mode::Visual);
                  self.query_input.set_cursor_style(Mode::Visual.cursor_style());
                },
                KeyCode::Esc | KeyCode::Char('v') if self.vim_editor.mode() == Mode::Visual => {
                  self.query_input.cancel_selection();
                  self.vim_editor = Vim::new(Mode::Normal);
                  self.query_input.set_cursor_style(Mode::Normal.cursor_style());
                },
                // Handle movement commands
                KeyCode::Char('h') => self.query_input.move_cursor(tui_textarea::CursorMove::Back),
                KeyCode::Char('j') => self.query_input.move_cursor(tui_textarea::CursorMove::Down),
                KeyCode::Char('k') => self.query_input.move_cursor(tui_textarea::CursorMove::Up),
                KeyCode::Char('l') => self.query_input.move_cursor(tui_textarea::CursorMove::Forward),
                KeyCode::Char('w') => self.query_input.move_cursor(tui_textarea::CursorMove::WordForward),
                KeyCode::Char('b') => self.query_input.move_cursor(tui_textarea::CursorMove::WordBack),
                KeyCode::Char('^') => self.query_input.move_cursor(tui_textarea::CursorMove::Head),
                KeyCode::Char('0') => self.query_input.move_cursor(tui_textarea::CursorMove::Head),
                KeyCode::Char('$') => self.query_input.move_cursor(tui_textarea::CursorMove::End),
                // Handle edit commands
                KeyCode::Char('x') => {
                  self.query_input.delete_next_char();
                },
                KeyCode::Char('p') => {
                  self.query_input.paste();
                },
                KeyCode::Char('u') => {
                  self.query_input.undo();
                },
                // Handle 'g' for operator mode
                KeyCode::Char('g') if self.vim_editor.mode() == Mode::Normal => {
                  self.vim_editor = Vim::new(Mode::Operator('g'));
                  self.query_input.set_cursor_style(Mode::Operator('g').cursor_style());
                },
                // Handle mode transitions
                KeyCode::Char('i') if self.vim_editor.mode() == Mode::Normal => {
                  self.vim_editor = Vim::new(Mode::Insert);
                  self.query_input.set_cursor_style(Mode::Insert.cursor_style());
                },
                KeyCode::Char('a') if self.vim_editor.mode() == Mode::Normal => {
                  self.query_input.move_cursor(tui_textarea::CursorMove::Forward);
                  self.vim_editor = Vim::new(Mode::Insert);
                  self.query_input.set_cursor_style(Mode::Insert.cursor_style());
                },
                KeyCode::Char('A') if self.vim_editor.mode() == Mode::Normal => {
                  self.query_input.move_cursor(tui_textarea::CursorMove::End);
                  self.vim_editor = Vim::new(Mode::Insert);
                  self.query_input.set_cursor_style(Mode::Insert.cursor_style());
                },
                KeyCode::Char('o') if self.vim_editor.mode() == Mode::Normal => {
                  self.query_input.move_cursor(tui_textarea::CursorMove::End);
                  self.query_input.insert_newline();
                  self.vim_editor = Vim::new(Mode::Insert);
                  self.query_input.set_cursor_style(Mode::Insert.cursor_style());
                },
                KeyCode::Char('O') if self.vim_editor.mode() == Mode::Normal => {
                  self.query_input.move_cursor(tui_textarea::CursorMove::Head);
                  self.query_input.insert_newline();
                  self.query_input.move_cursor(tui_textarea::CursorMove::Up);
                  self.vim_editor = Vim::new(Mode::Insert);
                  self.query_input.set_cursor_style(Mode::Insert.cursor_style());
                },
                // Handle query execution
                KeyCode::Enter if self.vim_editor.mode() == Mode::Normal => {
                  let query_text = self.query_input.lines().join(" ");
                  let trimmed_query = query_text.trim();
                  if !trimmed_query.is_empty() {
                    return Ok(Some(Action::HandleQuery(trimmed_query.to_string())));
                  }
                },
                _ => {},
              }
            }
          }
        }

        if let KeyCode::Char('q') = key.code {
          if self.error_message.is_some() {
            self.error_message = None;
          }
        }
      },
      ComponentKind::Results => {
        match key.code {
          KeyCode::Char('y') => {
            if let Some(json_str) = self.json() {
              let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
              ctx.set_contents(json_str).unwrap();
            }
          },
          KeyCode::Char('r') => {
            return Ok(Some(Action::HandleQuery(self.query_input.lines().join(" "))));
          },
          KeyCode::Char(' ') => {
            self.row_is_selected = !self.row_is_selected;
          },
          _ => {},
        }
      },
    }

    Ok(None)
  }

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::TablesLoaded(tables) => {
        let tables = tables.iter().filter(|t| t.schema == "public").cloned().collect();
        self.tables = tables;
      },
      Action::TableMoveDown => {
        if self.selected_table_index < self.table_row_count() {
          self.selected_table_index += 1;
        } else {
          self.selected_table_index = 0;
        }
      },
      Action::TableMoveUp => {
        if self.selected_table_index > 0 {
          self.selected_table_index -= 1;
        } else {
          self.selected_table_index =
            (self.table_row_count() as i32 - 1i32).clamp(0, self.table_row_count() as i32 - 1) as usize;
        }
      },
      Action::ScrollTableLeft => {
        if self.selected_component == ComponentKind::Results && self.horizonal_scroll_offset > 0 {
          self.horizonal_scroll_offset -= 1;
        }
      },
      Action::ScrollTableRight => {
        if self.selected_component == ComponentKind::Results
          && self.column_count() > 0
          && self.horizonal_scroll_offset * VISIBLE_COLUMNS < self.column_count() - VISIBLE_COLUMNS
        {
          self.horizonal_scroll_offset += 1;
        }
      },
      Action::RowMoveDown => {
        if !self.query_results.is_empty() {
          if self.selected_component == ComponentKind::Results
            && !self.row_is_selected
            && self.selected_row_index < self.query_results.len() - 1
          {
            self.selected_row_index += 1;
          } else if self.selected_component == ComponentKind::Results
            && self.row_is_selected
            && self.detail_row_index < self.query_results[self.selected_row_index].len() - 1
          {
            self.detail_row_index += 1;
          }
        }
      },
      Action::RowMoveUp => {
        if self.selected_component == ComponentKind::Results && self.selected_row_index > 0 && !self.row_is_selected {
          self.selected_row_index -= 1;
        } else if self.selected_component == ComponentKind::Results && self.row_is_selected && self.detail_row_index > 0
        {
          self.detail_row_index -= 1;
        }
      },
      Action::LoadSelectedTable => {
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          let query = format!("SELECT * FROM {}", selected_table.name);
          self.query_input.select_all();
          self.query_input.cut();
          self.query_input.insert_str(&query);
          return Ok(Some(Action::HandleQuery(query)));
        } else {
          return Ok(None);
        }
      },
      Action::QueryResult(headers, results) => {
        self.selected_headers = headers;
        self.query_results = results;
        self.horizonal_scroll_offset = 0;
        self.selected_row_index = 0;
        self.detail_row_index = 0;
        
        // Add successful query to history
        if let Some(query) = self.last_executed_query.take() {
          self.add_to_history(&query, true);
        }
        
        // Don't automatically switch focus to results - stay in current component
      },
      Action::FocusQuery => {
        self.selected_component = ComponentKind::Query;
        return Ok(Some(Action::SelectComponent(ComponentKind::Query)));
      },
      Action::FocusResults => {
        self.selected_component = ComponentKind::Results;
        return Ok(Some(Action::SelectComponent(ComponentKind::Results)));
      },
      Action::FocusHome => {
        self.selected_component = ComponentKind::Home;
        return Ok(Some(Action::SelectComponent(ComponentKind::Home)));
      },
      Action::ExecuteQuery => {
        // Execute selected text or full query if no selection (Ctrl+Y via keybinding)
        let query_text = if self.query_input.is_selecting() {
          // Get actual selected text by using the selection range
          let selection = self.query_input.selection_range();
          
          if let Some((start, end)) = selection {
            // Calculate selected text manually from the lines
            let lines = self.query_input.lines();
            let all_text = lines.join("\n");
            
            // Convert line-based positions to character positions
            let mut char_pos = 0;
            let mut start_char_pos = None;
            let mut end_char_pos = None;
            
            for (line_idx, line) in lines.iter().enumerate() {
              if line_idx == start.0 {
                start_char_pos = Some(char_pos + start.1);
              }
              if line_idx == end.0 {
                end_char_pos = Some(char_pos + end.1);
                break;
              }
              char_pos += line.len() + 1; // +1 for newline
            }
            
            if let (Some(start_pos), Some(end_pos)) = (start_char_pos, end_char_pos) {
              let selected_text = if start_pos <= end_pos {
                all_text.chars().skip(start_pos).take(end_pos - start_pos).collect::<String>()
              } else {
                all_text.chars().skip(end_pos).take(start_pos - end_pos).collect::<String>()
              };
              
              if selected_text.trim().is_empty() {
                self.query_input.lines().join(" ")
              } else {
                // Clean up selected text
                selected_text.lines()
                  .map(|line| line.trim())
                  .filter(|line| !line.is_empty())
                  .collect::<Vec<_>>()
                  .join(" ")
              }
            } else {
              // Fallback to yank if position calculation fails
              let selected = self.query_input.yank_text();
              if selected.trim().is_empty() {
                self.query_input.lines().join(" ")
              } else {
                selected
              }
            }
          } else {
            // Fallback to yank if no selection range
            let selected = self.query_input.yank_text();
            if selected.trim().is_empty() {
              self.query_input.lines().join(" ")
            } else {
              selected
            }
          }
        } else {
          self.query_input.lines().join(" ")
        };

        let cleaned_query = query_text.trim();
        
        // Only execute if query is not empty
        if !cleaned_query.is_empty() {
          // Store the query for history tracking
          self.last_executed_query = Some(cleaned_query.to_string());
          return Ok(Some(Action::HandleQuery(cleaned_query.to_string())));
        }
      },
      Action::RowDetails => {
        self.show_row_details = !self.show_row_details;
      },
      Action::Error(e) => {
        self.error_message = Some(e);
        
        // Add failed query to history (but don't save failed queries)
        if let Some(query) = self.last_executed_query.take() {
          // We could optionally add failed queries with success=false
          // For now, we only add successful queries to history
          // self.add_to_history(&query, false);
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    // Create the layout sections.
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Length(3), Constraint::Min(1)])
      .split(f.area());

    let title_block = Block::default().borders(Borders::ALL).style(Style::default());

    let title = Paragraph::new(Text::styled("Query Crafter", Style::default().fg(Color::Green))).block(title_block);

    f.render_widget(title, chunks[0]);

    let table_chunks = self.render_table_list(f, chunks)?;

    let query_chunks = self.render_query_input(f, table_chunks)?;

    self.render_query_results(f, query_chunks)?;

    self.render_error(f)?;

    Ok(())
  }
}
