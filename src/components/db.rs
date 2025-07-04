use std::{
  collections::{BTreeMap, HashMap},
  fs,
  path::PathBuf,
  rc::Rc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono;
use clipboard::{ClipboardContext, ClipboardProvider};
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use directories::ProjectDirs;
use nucleo::Utf32Str;
use ratatui::{
  prelude::*,
  text::{Line, Span},
  widgets::*,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, ComponentKind, Frame};
use crate::{
  action::Action,
  autocomplete::AutocompleteState,
  autocomplete_engine::AutocompleteEngine,
  components::vim::Vim,
  config::Config,
  editor_common::Mode,
  editor_component::EditorComponent,
};
use query_crafter_theme as theme;

const VISIBLE_COLUMNS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
  Table,   // Normal table navigation
  Row,     // Row is selected for detail view
  Cell,    // Individual cell selection
  Preview, // Popup preview mode
}

#[derive(Debug)]
pub enum EditorBackend {
  TuiTextarea(Vim),
}

impl Default for EditorBackend {
  fn default() -> Self {
    Self::TuiTextarea(Vim::new(Mode::Normal))
  }
}

impl EditorBackend {
  pub fn new_from_config(_backend_name: &str) -> Self {
    // For now, always use TuiTextarea
    // In the future, match on backend_name to support different editors
    Self::TuiTextarea(Vim::new(Mode::Normal))
  }

  pub fn as_editor_component(&mut self) -> &mut dyn EditorComponent {
    match self {
      Self::TuiTextarea(vim) => vim,
    }
  }

  pub fn get_text(&self) -> String {
    match self {
      Self::TuiTextarea(vim) => vim.get_text(),
    }
  }

  pub fn get_selected_text(&self) -> Option<String> {
    match self {
      Self::TuiTextarea(vim) => vim.get_selected_text(),
    }
  }

  pub fn set_text(&mut self, text: &str) {
    match self {
      Self::TuiTextarea(vim) => vim.set_text(text),
    }
  }

  pub fn get_cursor_position(&self) -> (usize, usize) {
    match self {
      Self::TuiTextarea(vim) => vim.get_cursor_position(),
    }
  }

  pub fn get_text_up_to_cursor(&self) -> String {
    match self {
      Self::TuiTextarea(vim) => vim.get_text_up_to_cursor(),
    }
  }

  pub fn insert_text_at_cursor(&mut self, text: &str) {
    match self {
      Self::TuiTextarea(vim) => vim.insert_text_at_cursor(text),
    }
  }

  pub fn delete_word_before_cursor(&mut self) {
    match self {
      Self::TuiTextarea(vim) => vim.delete_word_before_cursor(),
    }
  }

  // Compatibility methods for existing vim_editor usage
  pub fn mode(&self) -> Mode {
    match self {
      Self::TuiTextarea(vim) => vim.mode(),
    }
  }
  
  pub fn format_query(&mut self, selection_only: bool) -> Result<(), String> {
    match self {
      Self::TuiTextarea(vim) => vim.format_query(selection_only),
    }
  }
  
  pub fn format_all(&mut self) -> Result<(), String> {
    match self {
      Self::TuiTextarea(vim) => vim.format_all(),
    }
  }
  
  pub fn toggle_auto_format(&mut self) {
    match self {
      Self::TuiTextarea(vim) => vim.toggle_auto_format(),
    }
  }
  
  pub fn is_auto_format_enabled(&self) -> bool {
    match self {
      Self::TuiTextarea(vim) => vim.is_auto_format_enabled(),
    }
  }

  pub fn set_mode(&mut self, mode: Mode) {
    match self {
      Self::TuiTextarea(vim) => {
        *vim = Vim::new(mode);
      },
    }
  }

  pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    self.as_editor_component().on_key_event(key)
  }

  pub fn draw(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    self.as_editor_component().draw(f, area)
  }

  pub fn draw_with_focus(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool) {
    self.as_editor_component().draw_with_focus(f, area, is_focused)
  }

  pub fn init(&mut self, area: ratatui::layout::Rect) -> Result<()> {
    self.as_editor_component().init(area)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueryHistoryEntry {
  pub query: String,
  pub timestamp: u64,
  pub success: bool,
}

impl QueryHistoryEntry {
  pub fn new(query: String, success: bool) -> Self {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    Self { query, timestamp, success }
  }

  pub fn formatted_time(&self) -> String {
    // Convert timestamp to readable format
    let datetime = UNIX_EPOCH + Duration::from_secs(self.timestamp);
    format!("{datetime:?}")
      .split_once('.')
      .map(|(date_time, _)| date_time.replace("SystemTime { tv_sec: ", "").replace(", tv_nsec: 0 }", ""))
      .unwrap_or_else(|| "Unknown".to_string())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DbColumn {
  pub name: String,
  pub data_type: String,
  pub is_nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DbTable {
  pub name: String,
  pub schema: String,
  pub columns: Vec<DbColumn>,
}

pub struct Db {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
  tables: Vec<DbTable>,
  selected_table_index: usize,
  selected_row_index: usize,
  selected_headers: Vec<String>,
  query_results: Vec<Vec<String>>,
  selected_component: ComponentKind,
  editor_backend: EditorBackend,
  horizonal_scroll_offset: usize,
  show_row_details: bool,
  table_search_query: String,
  is_searching_tables: bool,
  selection_mode: SelectionMode,
  detail_row_index: usize,
  selected_cell_index: usize, // For cell selection mode
  error_message: Option<String>,
  // Query history functionality
  query_history: Vec<QueryHistoryEntry>,
  selected_history_index: usize,
  selected_tab: usize, // 0 = Query, 1 = History
  history_file_path: PathBuf,
  last_executed_query: Option<String>, // Track last query for history saving
  // Autocomplete functionality
  autocomplete_state: AutocompleteState,
  autocomplete_engine: AutocompleteEngine,
  table_columns_cache: HashMap<String, Vec<DbColumn>>,
  // Results search functionality
  results_search_query: String,
  is_searching_results: bool,
  filtered_results: Vec<(usize, u32)>, // (original_index, fuzzy_score)
  filtered_results_index: usize,
  results_matcher: nucleo::Matcher,
  // Help overlay
  show_help: bool,
  // Query loading state
  is_query_running: bool,
  query_start_time: Option<std::time::Instant>,
  // Table info display
  show_table_columns: bool,
  show_table_schema: bool,
  selected_table_columns: Vec<DbColumn>,
  selected_table_schema: String,
  table_info_scroll: u16, // For scrolling in the popup
  // CSV export status
  export_status: Option<(String, std::time::Instant)>, // (message, timestamp)
}

/// Creates a header cell with special styling and centering
fn create_header_cell(text: &str, row_height: u16) -> Cell<'_> {
  create_centered_cell(text, row_height)
}

/// Creates a `Cell` with text that is vertically centered.
///
/// # Arguments
///
/// * `text` - The text content for the cell (can be multi-line).
/// * `row_height` - The total height of the `Row` this cell will be in.
fn create_centered_cell(text: &str, row_height: u16) -> Cell<'_> {
  // Handle empty text
  if text.is_empty() {
    let padding = "\n".repeat((row_height / 2) as usize);
    return Cell::from(padding);
  }

  // Count the number of lines in the input text
  let text_lines: Vec<&str> = text.lines().collect();
  let text_height = text_lines.len() as u16;

  // Calculate the vertical padding required for centering
  let total_padding = row_height.saturating_sub(text_height);
  let padding_top = total_padding / 2;
  let padding_bottom = total_padding - padding_top;

  // Create centered text with top and bottom padding
  let mut centered_lines = vec![];

  // Add top padding
  for _ in 0..padding_top {
    centered_lines.push("".to_string());
  }

  // Add the actual text lines with horizontal centering
  for line in text_lines {
    // For horizontal centering in table cells, we'll use the alignment feature
    centered_lines.push(line.to_string());
  }

  // Add bottom padding to ensure proper height
  for _ in 0..padding_bottom {
    centered_lines.push("".to_string());
  }

  // Join all lines and create the cell
  let padded_text = centered_lines.join("\n");
  Cell::from(padded_text)
}

impl Default for Db {
  fn default() -> Self {
    Self {
      command_tx: None,
      config: Config::default(),
      tables: Vec::new(),
      selected_table_index: 0,
      selected_row_index: 0,
      selected_headers: Vec::new(),
      query_results: Vec::new(),
      selected_component: ComponentKind::Home,
      editor_backend: EditorBackend::default(),
      horizonal_scroll_offset: 0,
      show_row_details: false,
      table_search_query: String::new(),
      is_searching_tables: false,
      selection_mode: SelectionMode::Table,
      detail_row_index: 0,
      selected_cell_index: 0,
      error_message: None,
      query_history: Vec::new(),
      selected_history_index: 0,
      selected_tab: 0,
      history_file_path: PathBuf::new(),
      last_executed_query: None,
      autocomplete_state: AutocompleteState::new(),
      autocomplete_engine: AutocompleteEngine::new_builtin(),
      table_columns_cache: HashMap::new(),
      results_search_query: String::new(),
      is_searching_results: false,
      filtered_results: Vec::new(),
      filtered_results_index: 0,
      results_matcher: nucleo::Matcher::new(nucleo::Config::DEFAULT),
      show_help: false,
      is_query_running: false,
      query_start_time: None,
      show_table_columns: false,
      show_table_schema: false,
      selected_table_columns: Vec::new(),
      selected_table_schema: String::new(),
      table_info_scroll: 0,
      export_status: None,
    }
  }
}

impl Db {
  pub fn new() -> Self {
    Self::new_with_config(None)
  }

  pub fn new_with_config(config: Option<Config>) -> Self {
    let mut instance = Self::default();

    // Initialize editor backend based on config
    if let Some(ref config) = config {
      let backend = &config.editor.backend;
      instance.editor_backend = EditorBackend::new_from_config(backend);
      instance.config = config.clone();
    }

    // Initialize history file path in user's config directory
    if let Some(proj_dirs) = ProjectDirs::from("com", "query-crafter", "query-crafter") {
      instance.history_file_path = proj_dirs.config_dir().join("query_history.json");
    } else {
      // Fallback to current directory if config dir not available
      instance.history_file_path = PathBuf::from("query_history.json");
    }

    // Initialize autocomplete functionality
    instance.autocomplete_state = AutocompleteState::new();
    
    // Check config for autocomplete backend preference
    if let Some(ref config) = config {
      let backend = &config.autocomplete.backend;
      eprintln!("Initializing autocomplete with backend: {}", backend);
      
      match backend.as_str() {
        "lsp" => {
          eprintln!("Initializing LSP backend...");
          // Create LSP client
          let lsp_config = config.lsp.clone();
          let lsp_client = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::lsp::LspClient::new(lsp_config)
          ));
          
          // Try to create LSP engine
          match AutocompleteEngine::new_lsp(lsp_client.clone()) {
            Ok(engine) => {
              instance.autocomplete_engine = engine;
              eprintln!("LSP autocomplete engine created successfully");
              
              // Start LSP server in background
              let client_clone = lsp_client.clone();
              tokio::spawn(async move {
                let mut client = client_clone.lock().await;
                if let Err(e) = client.start().await {
                  eprintln!("Failed to start LSP server: {}", e);
                }
              });
            },
            Err(e) => {
              eprintln!("Failed to create LSP engine: {} - using builtin", e);
              instance.autocomplete_engine = AutocompleteEngine::new_builtin();
            }
          }
        },
        "hybrid" => {
          eprintln!("Initializing hybrid backend...");
          // Create LSP client
          let lsp_config = config.lsp.clone();
          let lsp_client = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::lsp::LspClient::new(lsp_config)
          ));
          
          // Try to create hybrid engine
          match AutocompleteEngine::new_hybrid(lsp_client.clone()) {
            Ok(engine) => {
              instance.autocomplete_engine = engine;
              eprintln!("Hybrid autocomplete engine created successfully");
              
              // Start LSP server in background
              let client_clone = lsp_client.clone();
              tokio::spawn(async move {
                let mut client = client_clone.lock().await;
                if let Err(e) = client.start().await {
                  eprintln!("Failed to start LSP server: {}", e);
                }
              });
            },
            Err(e) => {
              eprintln!("Failed to create hybrid engine: {} - using builtin", e);
              instance.autocomplete_engine = AutocompleteEngine::new_builtin();
            }
          }
        },
        _ => {
          instance.autocomplete_engine = AutocompleteEngine::new_builtin();
        }
      }
    } else {
      instance.autocomplete_engine = AutocompleteEngine::new_builtin();
    }
    
    eprintln!("Autocomplete engine initialized: {}", instance.autocomplete_engine.backend_name());
    instance.table_columns_cache = HashMap::new();

    // Load existing history
    instance.load_query_history();

    instance
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

    // Get the actual row index from filtered results if searching
    let actual_row_index =
      if !self.filtered_results.is_empty() && self.filtered_results_index < self.filtered_results.len() {
        self.filtered_results[self.filtered_results_index].0
      } else {
        self.selected_row_index
      };

    let json_str = if self.selection_mode == SelectionMode::Row {
      if let Some(selected_row) = self.query_results.get(actual_row_index) {
        if let Some(selected_cell) = selected_row.get(self.detail_row_index) {
          selected_cell.to_string()
        } else {
          String::new()
        }
      } else {
        String::new()
      }
    } else if let Some(row) = self.query_results.get(actual_row_index) {
      let row_data =
        row.iter().zip(self.selected_headers.iter()).fold(BTreeMap::new(), |mut acc, (value, header)| {
          acc.insert(header, value);
          acc
        });
      serde_json::to_string_pretty(&row_data).unwrap()
    } else {
      String::new()
    };

    Some(json_str)
  }

  /// Filter query results using fuzzy search across all columns
  fn filter_results_fuzzy(&mut self) {
    self.filtered_results.clear();

    if self.results_search_query.is_empty() {
      // Empty search shows all results
      for idx in 0..self.query_results.len() {
        self.filtered_results.push((idx, 100));
      }
      return;
    }

    let mut query_buf = Vec::new();
    let query_utf32 = Utf32Str::new(&self.results_search_query, &mut query_buf);

    for (idx, row) in self.query_results.iter().enumerate() {
      // Concatenate all column values with spaces for full-row searching
      let row_text = row.join(" ");
      let mut text_buf = Vec::new();
      let text_utf32 = Utf32Str::new(&row_text, &mut text_buf);

      if let Some(score) = self.results_matcher.fuzzy_match(text_utf32, query_utf32) {
        self.filtered_results.push((idx, score as u32));
      }
    }

    // Sort by score descending (best matches first)
    self.filtered_results.sort_by(|a, b| b.1.cmp(&a.1));

    // Reset filtered index if out of bounds
    if self.filtered_results_index >= self.filtered_results.len() {
      self.filtered_results_index = 0;
    }
  }

  fn render_table_list(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    // Keep the same layout regardless of info display (info will be a popup)
    let table_chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
      .split(chunks[1]);

    let is_focused = self.selected_component == ComponentKind::Home;
    let tables = Block::default()
      .borders(Borders::ALL)
      .style(if is_focused { theme::border_focused() } else { theme::border_normal() })
      .title("[1] Tables")
      .title_style(theme::title())
      .border_type(BorderType::Rounded);

    let table_list_chunks = if self.is_searching_tables {
      Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(table_chunks[0])
    } else {
      table_chunks.clone()
    };

    if self.is_searching_tables {
      let search_block = Block::default()
        .borders(Borders::ALL)
        .title("Search")
        .title_style(theme::title())
        .border_style(theme::border_focused())
        .border_type(BorderType::Rounded);
      let search_text =
        Paragraph::new(Text::styled(self.table_search_query.to_string(), theme::warning()))
          .block(search_block)
          .style(theme::input());
      f.render_widget(search_text, table_list_chunks[0]);
    }

    let table_render_chunk = if self.is_searching_tables { table_list_chunks[1] } else { table_list_chunks[0] };

    // Check if we have tables to display
    if self.tables.is_empty() && self.is_searching_tables && !self.table_search_query.is_empty() {
      // Show "No tables found" message for empty search results
      let no_results_msg = format!("No tables found matching '{}'", self.table_search_query);
      let no_results = Paragraph::new(Text::styled(no_results_msg, theme::warning()))
        .block(tables)
        .style(theme::bg_primary())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
      f.render_widget(no_results, table_render_chunk);
    } else {
      // Render normal table list
      let mut table_list_state = ListState::default();
      if !self.tables.is_empty() {
        table_list_state.select(Some(self.selected_table_index));
      }
      let items: Vec<ListItem> = self.tables.iter().map(|t| ListItem::new(t.name.to_string())).collect();

      let list = List::new(items)
        .block(tables)
        .style(theme::bg_primary())
        .highlight_style(theme::selection_active());
      f.render_stateful_widget(list, table_render_chunk, &mut table_list_state);
    }

    Ok(table_chunks)
  }

  fn render_query_input(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let query_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Length(2), Constraint::Length(20), Constraint::Min(1)].as_ref())
      .split(chunks[1]);

    // Render tabs
    let tabs = Tabs::new(["Query [t]", "History [t]"])
      .style(theme::tab_normal())
      .highlight_style(theme::tab_selected())
      .select(self.selected_tab)
      .padding("", "")
      .divider(" ");
    f.render_widget(tabs, query_chunks[0]);

    // Render content based on selected tab
    let is_query_focused = self.selected_component == ComponentKind::Query;
    match self.selected_tab {
      0 => {
        // Query tab - show the editor backend with focus state
        self.editor_backend.draw_with_focus(f, query_chunks[1], is_query_focused);

        // Render autocomplete popup if active
        if self.autocomplete_state.is_active && is_query_focused {
          self.render_autocomplete_popup(f, query_chunks[1])?;
        }
      },
      1 => {
        // History tab - show the history list
        self.render_history_list(f, query_chunks[1])?;
      },
      _ => {
        // Default to query tab
        self.editor_backend.draw_with_focus(f, query_chunks[1], is_query_focused);

        // Render autocomplete popup if active
        if self.autocomplete_state.is_active && is_query_focused {
          self.render_autocomplete_popup(f, query_chunks[1])?;
        }
      },
    }

    Ok(query_chunks)
  }

  fn render_history_list(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    let is_focused = self.selected_component == ComponentKind::Query;
    let history_block = Block::default()
      .borders(Borders::ALL)
      .border_style(if is_focused {
        theme::border_focused()
      } else {
        theme::border_normal()
      })
      .title("[2] Query History - [Enter] Execute [y] Copy [c] Edit [d] Delete")
      .title_style(theme::title())
      .border_type(BorderType::Rounded);

    if self.query_history.is_empty() {
      let empty_msg = Paragraph::new("No query history available")
        .block(history_block)
        .style(theme::muted())
        .alignment(Alignment::Center);
      f.render_widget(empty_msg, area);
      return Ok(());
    }

    // Create list items from history (most recent first)
    let items: Vec<ListItem> = self.query_history
      .iter()
      .rev() // Show most recent first
      .map(|entry| {
        let truncated_query = if entry.query.len() > 60 {
          format!("{}...", &entry.query[..57])
        } else {
          entry.query.clone()
        };

        let time_str = chrono::DateTime::from_timestamp(entry.timestamp as i64, 0)
          .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
          .unwrap_or_else(|| "Unknown".to_string());

        let display_text = format!("{time_str} | {truncated_query}");

        ListItem::new(display_text)
          .style(if entry.success {
            theme::success()
          } else {
            theme::error()
          })
      })
      .collect();

    let mut list_state = ListState::default();
    // selected_history_index maps directly to display position (0 = most recent = top)
    if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() {
      list_state.select(Some(self.selected_history_index));
    }

    let history_list = List::new(items)
      .block(history_block)
      .style(theme::bg_primary())
      .highlight_style(theme::selection_active());

    f.render_stateful_widget(history_list, area, &mut list_state);

    Ok(())
  }

  fn render_query_results(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    match self.selection_mode {
      SelectionMode::Row => self.render_split_view(f, chunks),
      SelectionMode::Preview => self.render_preview_popup(f, chunks),
      SelectionMode::Cell => self.render_cell_selection_view(f, chunks),
      SelectionMode::Table => self.render_query_results_table(f, chunks),
    }
  }

  fn render_query_results_table(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    // Use the last chunk for results (either index 1 or 2 depending on layout)
    let results_area = chunks[chunks.len() - 1];
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(results_area);

    let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;

    // Define consistent row heights for better vertical centering
    const HEADER_HEIGHT: u16 = 2;
    const ROW_HEIGHT: u16 = 3;

    let header_cells: Vec<_> = self
      .selected_headers
      .iter()
      .skip(skip_count)
      .take(VISIBLE_COLUMNS)
      .map(|h| create_header_cell(h, HEADER_HEIGHT))
      .collect();
    let header = ratatui::widgets::Row::new(header_cells)
      .height(HEADER_HEIGHT)
      .style(theme::header())
      .bottom_margin(1);

    // Use filtered results if available
    let rows = if self.filtered_results.is_empty() {
      // No search active or no results - show all rows
      self
        .query_results
        .iter()
        .map(|r| {
          let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| create_centered_cell(c, ROW_HEIGHT));
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    } else {
      // Show only filtered rows
      self
        .filtered_results
        .iter()
        .filter_map(|(idx, _score)| self.query_results.get(*idx))
        .map(|r| {
          let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| create_centered_cell(c, ROW_HEIGHT));
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    };

    // Build status text with export status if present
    let mut status_parts = vec![];
    
    // Add row count
    if !self.results_search_query.is_empty() {
      let filtered_count = self.filtered_results.len();
      let total_count = self.query_results.len();
      status_parts.push(format!("Rows: {filtered_count}/{total_count} (search: '{}')", self.results_search_query));
    } else {
      status_parts.push(format!("Rows: {}", self.query_results.len()));
    }
    
    // Add export status if recent (show for 5 seconds)
    if let Some((message, timestamp)) = &self.export_status {
      if timestamp.elapsed().as_secs() < 5 {
        status_parts.push(format!(" | {}", message));
      } else {
        // Clear old status
        self.export_status = None;
      }
    }
    
    let status_text = Paragraph::new(Text::styled(
      status_parts.join(""),
      theme::info(),
    ));
    f.render_widget(status_text, table_chunks[1]);

    let is_results_focused = self.selected_component == ComponentKind::Results;
    let mut table_state = TableState::default();

    // Update table state to use filtered index if searching
    if !self.filtered_results.is_empty() {
      table_state.select(Some(self.filtered_results_index));
    } else {
      table_state.select(Some(self.selected_row_index));
    }

    // Update title based on search mode and loading state
    let title = if self.is_query_running {
      let elapsed = self.query_start_time
        .map(|start| start.elapsed().as_secs_f32())
        .unwrap_or(0.0);
      let spinner = match (elapsed * 4.0) as usize % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
      };
      format!("[3] Results - {} Loading... ({:.1}s)", spinner, elapsed)
    } else if self.is_searching_results {
      format!("[3] Results - Search: {}_", self.results_search_query)
    } else if !self.results_search_query.is_empty() {
      format!("[3] Results - Filter: {}", self.results_search_query)
    } else {
      "[3] Results".to_string()
    };

    let result_table = Table::default()
      .rows(rows)
      .header(header)
      .column_spacing(10)
      .block(
        Block::default()
          .borders(Borders::ALL)
          .title(title)
          .title_style(if self.is_searching_results {
            theme::warning()
          } else {
            theme::title()
          })
          .border_style(if is_results_focused {
            theme::border_focused()
          } else {
            theme::border_normal()
          })
          .border_type(BorderType::Rounded),
      )
      .style(theme::bg_primary())
      .highlight_symbol("\n▶ ")
      .row_highlight_style(theme::selection_active())
      .widths((0..VISIBLE_COLUMNS).map(|_| Constraint::Percentage((100 / VISIBLE_COLUMNS) as u16)).collect::<Vec<_>>());

    f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);

    if self.show_row_details {
      if let Some(json_str) = self.json() {
        let area = self.centered_rect(80, 60, f.area());
        let block = Block::default()
          .title("Row Details")
          .title_style(theme::title())
          .borders(Borders::ALL)
          .border_style(theme::border_focused())
          .border_type(BorderType::Rounded);
        let paragraph = Paragraph::new(json_str.as_str())
          .block(block)
          .style(theme::bg_secondary())
          .wrap(Wrap { trim: true });
        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
      }
    }

    Ok(chunks)
  }

  fn render_split_view(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    // Use the last chunk for results (either index 1 or 2 depending on layout)
    let results_area = chunks[chunks.len() - 1];
    // Split the results area into two panes: table (60%) and details (40%)
    let split_chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
      .split(results_area);

    // Render the table on the left
    self.render_query_results_table_in_area(f, split_chunks[0])?;

    // Render the details on the right
    self.render_detail_pane(f, split_chunks[1])?;

    Ok(chunks)
  }

  fn render_preview_popup(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    // First render the normal table view
    self.render_query_results_table(f, chunks.clone())?;

    // Then render a popup overlay with row details
    if let Some(selected_row) = self.get_selected_row() {
      let area = self.centered_rect(70, 70, f.area());
      let block = Block::default()
        .title("Row Preview (ESC to close)")
        .title_style(theme::title())
        .borders(Borders::ALL)
        .border_style(theme::border_focused())
        .border_type(BorderType::Rounded);

      // Format row data nicely
      let mut text_lines = vec![];
      for (header, value) in self.selected_headers.iter().zip(selected_row.iter()) {
        text_lines
          .push(Line::from(vec![Span::styled(format!("{header}: "), theme::info()), Span::raw(value)]));
      }

      let paragraph =
        Paragraph::new(text_lines).block(block).style(theme::bg_secondary()).wrap(Wrap { trim: true });

      f.render_widget(Clear, area);
      f.render_widget(paragraph, area);
    }

    Ok(chunks)
  }

  fn render_query_results_table_in_area(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(area);

    let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;

    // Define consistent row heights for better vertical centering
    const HEADER_HEIGHT: u16 = 2;
    const ROW_HEIGHT: u16 = 3;

    let header_cells: Vec<_> = self
      .selected_headers
      .iter()
      .skip(skip_count)
      .take(VISIBLE_COLUMNS)
      .map(|h| create_header_cell(h, HEADER_HEIGHT))
      .collect();
    let header = ratatui::widgets::Row::new(header_cells)
      .height(HEADER_HEIGHT)
      .style(theme::header())
      .bottom_margin(1);

    // Use filtered results if available
    let rows = if self.filtered_results.is_empty() {
      // No search active or no results - show all rows
      self
        .query_results
        .iter()
        .map(|r| {
          let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| create_centered_cell(c, ROW_HEIGHT));
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    } else {
      // Show only filtered rows
      self
        .filtered_results
        .iter()
        .filter_map(|(idx, _score)| self.query_results.get(*idx))
        .map(|r| {
          let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| create_centered_cell(c, ROW_HEIGHT));
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    };

    // Build status text with export status if present
    let mut status_parts = vec![];
    
    // Add row count
    if !self.results_search_query.is_empty() {
      let filtered_count = self.filtered_results.len();
      let total_count = self.query_results.len();
      status_parts.push(format!("Rows: {filtered_count}/{total_count} (search: '{}')", self.results_search_query));
    } else {
      status_parts.push(format!("Rows: {}", self.query_results.len()));
    }
    
    // Add export status if recent (show for 5 seconds)
    if let Some((message, timestamp)) = &self.export_status {
      if timestamp.elapsed().as_secs() < 5 {
        status_parts.push(format!(" | {}", message));
      } else {
        // Clear old status
        self.export_status = None;
      }
    }
    
    let status_text = Paragraph::new(Text::styled(
      status_parts.join(""),
      theme::info(),
    ));
    f.render_widget(status_text, table_chunks[1]);

    let is_results_focused = self.selected_component == ComponentKind::Results;
    let mut table_state = TableState::default();

    // Update table state to use filtered index if searching
    if !self.filtered_results.is_empty() {
      table_state.select(Some(self.filtered_results_index));
    } else {
      table_state.select(Some(self.selected_row_index));
    }

    // Update title based on search mode and loading state
    let title = if self.is_query_running {
      let elapsed = self.query_start_time
        .map(|start| start.elapsed().as_secs_f32())
        .unwrap_or(0.0);
      let spinner = match (elapsed * 4.0) as usize % 4 {
        0 => "⠋",
        1 => "⠙",
        2 => "⠹",
        _ => "⠸",
      };
      format!("[3] Results - {} Loading... ({:.1}s)", spinner, elapsed)
    } else if self.is_searching_results {
      format!("[3] Results - Search: {}_", self.results_search_query)
    } else if !self.results_search_query.is_empty() {
      format!("[3] Results - Filter: {}", self.results_search_query)
    } else {
      "[3] Results".to_string()
    };

    let result_table = Table::default()
      .rows(rows)
      .header(header)
      .column_spacing(10)
      .block(
        Block::default()
          .borders(Borders::ALL)
          .title(title)
          .title_style(if self.is_searching_results {
            theme::warning()
          } else {
            theme::title()
          })
          .border_style(if is_results_focused && self.selection_mode == SelectionMode::Table {
            theme::border_focused()
          } else {
            theme::border_normal()
          })
          .border_type(BorderType::Rounded),
      )
      .style(theme::bg_primary())
      .highlight_symbol("\n▶ ")
      .row_highlight_style(theme::selection_active())
      .widths((0..VISIBLE_COLUMNS).map(|_| Constraint::Percentage((100 / VISIBLE_COLUMNS) as u16)).collect::<Vec<_>>());

    f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);

    Ok(())
  }

  fn render_detail_pane(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    if let Some(selected_row) = self.get_selected_row() {
      let block = Block::default()
        .borders(Borders::ALL)
        .title("Row Details")
        .title_style(theme::title())
        .border_style(
          if self.selected_component == ComponentKind::Results && self.selection_mode == SelectionMode::Row {
            theme::border_focused()
          } else {
            theme::border_normal()
          },
        )
        .border_type(BorderType::Rounded);

      // Create a table showing column names and values
      let header_cells =
        ["Column", "Value"].iter().map(|h| Cell::from(h.to_string()).style(theme::header()));
      let header = ratatui::widgets::Row::new(header_cells).style(theme::bg_primary()).height(1);

      let rows = selected_row
        .iter()
        .zip(self.selected_headers.iter())
        .map(|(value, column)| {
          let cells = [Cell::from(column.to_string()), Cell::from(value.to_string())];
          ratatui::widgets::Row::new(cells).height(1)
        })
        .collect::<Vec<_>>();

      let mut table_state = TableState::default();
      table_state.select(Some(self.detail_row_index));

      let detail_table = Table::default()
        .rows(rows)
        .header(header)
        .block(block)
        .style(theme::bg_primary())
        .highlight_symbol("▶ ")
        .row_highlight_style(theme::selection_active())
        .widths([Constraint::Percentage(30), Constraint::Percentage(70)]);

      f.render_stateful_widget(detail_table, area, &mut table_state);
    }

    Ok(())
  }

  fn render_cell_selection_view(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    // Use the last chunk for results (either index 1 or 2 depending on layout)
    let results_area = chunks[chunks.len() - 1];
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(results_area);

    // Auto-scroll to keep selected cell in view
    let cell_page = self.selected_cell_index / VISIBLE_COLUMNS;
    if cell_page != self.horizonal_scroll_offset {
      self.horizonal_scroll_offset = cell_page;
    }

    let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;

    // Define consistent row heights for better vertical centering
    const HEADER_HEIGHT: u16 = 2;
    const ROW_HEIGHT: u16 = 3;

    let header_cells: Vec<_> = self
      .selected_headers
      .iter()
      .skip(skip_count)
      .take(VISIBLE_COLUMNS)
      .map(|h| create_header_cell(h, HEADER_HEIGHT))
      .collect();
    let header = ratatui::widgets::Row::new(header_cells)
      .height(HEADER_HEIGHT)
      .style(theme::header())
      .bottom_margin(1);

    // Use filtered results if available
    let rows = if self.filtered_results.is_empty() {
      // No search active or no results - show all rows
      self
        .query_results
        .iter()
        .enumerate()
        .map(|(row_idx, r)| {
          let cells = r.iter().enumerate().skip(skip_count).take(VISIBLE_COLUMNS).map(|(actual_col_idx, c)| {
            let cell = create_centered_cell(c, ROW_HEIGHT);
            // Highlight the selected cell
            if row_idx == self.selected_row_index && actual_col_idx == self.selected_cell_index {
              cell.style(theme::selection_active())
            } else {
              cell
            }
          });
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    } else {
      // Show only filtered rows
      self
        .filtered_results
        .iter()
        .filter_map(|(idx, _score)| self.query_results.get(*idx).map(|row| (*idx, row)))
        .map(|(row_idx, r)| {
          let cells = r.iter().enumerate().skip(skip_count).take(VISIBLE_COLUMNS).map(|(actual_col_idx, c)| {
            let cell = create_centered_cell(c, ROW_HEIGHT);
            // Highlight the selected cell
            if row_idx == self.selected_row_index && actual_col_idx == self.selected_cell_index {
              cell.style(theme::selection_active())
            } else {
              cell
            }
          });
          ratatui::widgets::Row::new(cells).height(ROW_HEIGHT)
        })
        .collect::<Vec<_>>()
    };

    // Update status text to show cell position
    let column_name = self.selected_headers.get(self.selected_cell_index).cloned().unwrap_or_else(|| "?".to_string());
    let status_text = Paragraph::new(Text::styled(
      format!(
        "Cell Mode - Row: {}/{}, Col: {} ({}/{}) - Press ESC to exit",
        self.selected_row_index + 1,
        self.query_results.len(),
        column_name,
        self.selected_cell_index + 1,
        self.selected_headers.len()
      ),
      theme::warning(),
    ));
    f.render_widget(status_text, table_chunks[1]);

    let is_results_focused = self.selected_component == ComponentKind::Results;
    let mut table_state = TableState::default();

    // Update table state to use filtered index if searching
    if !self.filtered_results.is_empty() {
      table_state.select(Some(self.filtered_results_index));
    } else {
      table_state.select(Some(self.selected_row_index));
    }

    let result_table = Table::default()
      .rows(rows)
      .header(header)
      .column_spacing(10)
      .block(
        Block::default()
          .borders(Borders::ALL)
          .title("[3] Results - Cell Selection")
          .title_style(theme::warning())
          .border_style(if is_results_focused {
            theme::border_focused()
          } else {
            theme::border_normal()
          })
          .border_type(BorderType::Rounded),
      )
      .style(theme::bg_primary())
      .highlight_symbol("\n▶ ")
      .row_highlight_style(Style::default()) // Don't use row highlight style in cell mode
      .widths(
        (0..VISIBLE_COLUMNS)
          .map(|_| Constraint::Percentage((100 / VISIBLE_COLUMNS) as u16))
          .collect::<Vec<_>>()
      );

    f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);

    Ok(chunks)
  }

  fn get_selected_row(&self) -> Option<&Vec<String>> {
    // Get the actual row index from filtered results if searching
    let actual_row_index =
      if !self.filtered_results.is_empty() && self.filtered_results_index < self.filtered_results.len() {
        self.filtered_results[self.filtered_results_index].0
      } else {
        self.selected_row_index
      };

    self.query_results.get(actual_row_index)
  }

  fn get_copy_content(&self, full_row: bool, as_json: bool) -> Option<String> {
    let selected_row = self.get_selected_row()?;

    if as_json {
      // Copy as JSON
      let row_data =
        selected_row.iter().zip(self.selected_headers.iter()).fold(BTreeMap::new(), |mut acc, (value, header)| {
          acc.insert(header.clone(), value.clone());
          acc
        });
      serde_json::to_string_pretty(&row_data).ok()
    } else if full_row {
      // Copy entire row as TSV
      Some(selected_row.join("\t"))
    } else {
      // Copy based on selection mode
      match self.selection_mode {
        SelectionMode::Cell => {
          // Copy specific cell
          selected_row.get(self.selected_cell_index).cloned()
        },
        SelectionMode::Row => {
          // Copy the currently selected detail cell
          selected_row.get(self.detail_row_index).cloned()
        },
        _ => {
          // In table mode, copy the whole row as TSV
          Some(selected_row.join("\t"))
        },
      }
    }
  }

  fn render_error(&mut self, f: &mut Frame<'_>) -> Result<()> {
    if let Some(error_message) = &self.error_message {
      let area = self.centered_rect(60, 20, f.area());
      let block = Block::default()
        .title("Error")
        .title_style(theme::error())
        .borders(Borders::ALL)
        .border_style(theme::error())
        .border_type(BorderType::Rounded);
      let paragraph = Paragraph::new(error_message.as_str())
        .block(block)
        .style(theme::bg_secondary())
        .wrap(Wrap { trim: true });
      f.render_widget(Clear, area);
      f.render_widget(paragraph, area);
    }

    Ok(())
  }

  fn render_help(&mut self, f: &mut Frame<'_>) -> Result<()> {
    if !self.show_help {
      return Ok(());
    }

    let area = self.centered_rect(80, 80, f.area());
    let block = Block::default()
      .title("Keyboard Shortcuts (ESC to close)")
      .title_style(theme::title())
      .borders(Borders::ALL)
      .border_style(theme::border_focused())
      .border_type(BorderType::Rounded);

    let help_text = vec![
      Line::from(vec![Span::styled("Navigation", theme::header())]),
      Line::from(vec![
        Span::styled("1/2/3", theme::info()),
        Span::raw(" - Switch between Tables/Query/Results"),
      ]),
      Line::from(vec![
        Span::styled("↑/↓/←/→", theme::info()),
        Span::raw(" - Navigate tables/rows/columns"),
      ]),
      Line::from(vec![Span::styled("j/k/h/l", theme::info()), Span::raw(" - Vim-style navigation")]),
      Line::from(vec![Span::styled("gg", theme::info()), Span::raw(" - Jump to first item")]),
      Line::from(vec![Span::styled("G (Shift+G)", theme::info()), Span::raw(" - Jump to last item")]),
      Line::from(vec![Span::styled("Ctrl+B", theme::info()), Span::raw(" - Page up (10 items)")]),
      Line::from(vec![Span::styled("Ctrl+F", theme::info()), Span::raw(" - Page down (10 items)")]),
      Line::from(""),
      Line::from(vec![Span::styled("Tables (Panel 1)", theme::header())]),
      Line::from(vec![Span::styled("/", theme::info()), Span::raw(" - Search tables")]),
      Line::from(vec![Span::styled("Enter", theme::info()), Span::raw(" - Load selected table")]),
      Line::from(vec![Span::styled("c", theme::info()), Span::raw(" - View/hide table columns")]),
      Line::from(vec![Span::styled("s", theme::info()), Span::raw(" - View/hide table schema")]),
      Line::from(""),
      Line::from(vec![Span::styled("Query Editor (Panel 2)", theme::header())]),
      Line::from(vec![Span::styled("Ctrl+Enter", theme::info()), Span::raw(" - Execute query")]),
      Line::from(vec![Span::styled("Ctrl+u", theme::info()), Span::raw(" - Clear query editor")]),
      Line::from(vec![Span::styled("Ctrl+Space", theme::info()), Span::raw(" - Trigger autocomplete")]),
      Line::from(vec![
        Span::styled("Tab", theme::info()),
        Span::raw(" - Switch between Query/History tabs"),
      ]),
      Line::from(""),
      Line::from(vec![Span::styled("Query Formatting", theme::header())]),
      Line::from(vec![Span::styled("==", theme::info()), Span::raw(" - Format entire query")]),
      Line::from(vec![Span::styled("= (visual mode)", theme::info()), Span::raw(" - Format selection")]),
      Line::from(vec![Span::styled("=G", theme::info()), Span::raw(" - Format to end of file")]),
      Line::from(vec![Span::styled("=a", theme::info()), Span::raw(" - Toggle auto-format on execute")]),
      Line::from(""),
      Line::from(vec![Span::styled("History Tab", theme::header())]),
      Line::from(vec![Span::styled("Enter", theme::info()), Span::raw(" - Execute selected query")]),
      Line::from(vec![Span::styled("c", theme::info()), Span::raw(" - Copy query to editor")]),
      Line::from(vec![Span::styled("y", theme::info()), Span::raw(" - Copy query to clipboard")]),
      Line::from(vec![Span::styled("d", theme::info()), Span::raw(" - Delete query from history")]),
      Line::from(""),
      Line::from(vec![Span::styled("Results (Panel 3)", theme::header())]),
      Line::from(vec![Span::styled("/", theme::info()), Span::raw(" - Search results")]),
      Line::from(vec![Span::styled("Space", theme::info()), Span::raw(" - Toggle row detail view")]),
      Line::from(vec![Span::styled("p", theme::info()), Span::raw(" - Preview row in popup")]),
      Line::from(vec![Span::styled("v", theme::info()), Span::raw(" - Enter cell selection mode")]),
      Line::from(vec![Span::styled("r", theme::info()), Span::raw(" - Re-run last query")]),
      Line::from(vec![Span::styled("Ctrl+S", theme::info()), Span::raw(" - Export results to CSV")]),
      Line::from(""),
      Line::from(vec![Span::styled("Copy Commands", theme::header())]),
      Line::from(vec![Span::styled("y", theme::info()), Span::raw(" - Copy current cell/row")]),
      Line::from(vec![Span::styled("Y", theme::info()), Span::raw(" - Copy entire row as TSV")]),
      Line::from(vec![Span::styled("Ctrl+y", theme::info()), Span::raw(" - Copy row as JSON")]),
      Line::from(""),
      Line::from(vec![Span::styled("General", theme::header())]),
      Line::from(vec![Span::styled("?", theme::info()), Span::raw(" - Show this help")]),
      Line::from(vec![Span::styled("q", theme::info()), Span::raw(" - Quit application")]),
      Line::from(vec![Span::styled("ESC", theme::info()), Span::raw(" - Exit current mode/close popup")]),
    ];

    let paragraph = Paragraph::new(help_text)
      .block(block)
      .style(theme::bg_secondary())
      .wrap(Wrap { trim: false })
      .scroll((0, 0));

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);

    Ok(())
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

impl Component for Db {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    // Check if editor backend has changed
    let current_backend_name = match &self.editor_backend {
      EditorBackend::TuiTextarea(_) => "tui-textarea",
    };

    if current_backend_name != config.editor.backend {
      // Save current text before switching
      let current_text = self.editor_backend.get_text();

      // Switch to new backend
      self.editor_backend = EditorBackend::new_from_config(&config.editor.backend);

      // Restore text content
      self.editor_backend.set_text(&current_text);
    }

    // Update autocomplete backend if changed
    let requested_backend = &config.autocomplete.backend;
    eprintln!("Autocomplete backend requested: {}", requested_backend);
    
    match requested_backend.as_str() {
      "builtin" => {
        // Only switch if not already builtin
        if !matches!(self.autocomplete_engine.backend_name(), "builtin") {
          self.autocomplete_engine = AutocompleteEngine::new_builtin();
          eprintln!("Switched to builtin autocomplete engine");
        }
      },
      "lsp" => {
        // Only switch if not already lsp
        if !matches!(self.autocomplete_engine.backend_name(), "lsp") {
          eprintln!("Switching to LSP backend...");
          let lsp_config = config.lsp.clone();
          let lsp_client = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::lsp::LspClient::new(lsp_config)
          ));
          
          match AutocompleteEngine::new_lsp(lsp_client.clone()) {
            Ok(engine) => {
              self.autocomplete_engine = engine;
              eprintln!("Switched to LSP autocomplete engine");
              
              // Start LSP server in background
              let client_clone = lsp_client.clone();
              tokio::spawn(async move {
                let mut client = client_clone.lock().await;
                if let Err(e) = client.start().await {
                  eprintln!("Failed to start LSP server: {}", e);
                }
              });
            },
            Err(e) => {
              eprintln!("Failed to create LSP engine: {} - keeping current", e);
            }
          }
        }
      },
      "hybrid" => {
        // Only switch if not already hybrid
        if !matches!(self.autocomplete_engine.backend_name(), "hybrid") {
          eprintln!("Switching to hybrid backend...");
          let lsp_config = config.lsp.clone();
          let lsp_client = std::sync::Arc::new(tokio::sync::Mutex::new(
            crate::lsp::LspClient::new(lsp_config)
          ));
          
          match AutocompleteEngine::new_hybrid(lsp_client.clone()) {
            Ok(engine) => {
              self.autocomplete_engine = engine;
              eprintln!("Switched to hybrid autocomplete engine");
              
              // Start LSP server in background
              let client_clone = lsp_client.clone();
              tokio::spawn(async move {
                let mut client = client_clone.lock().await;
                if let Err(e) = client.start().await {
                  eprintln!("Failed to start LSP server: {}", e);
                }
              });
            },
            Err(e) => {
              eprintln!("Failed to create hybrid engine: {} - keeping current", e);
            }
          }
        }
      },
      _ => {
        // Unknown backend, keep current
        eprintln!("Unknown autocomplete backend '{}' - keeping current", requested_backend);
      }
    }
    
    eprintln!("Current autocomplete engine: {}", self.autocomplete_engine.backend_name());

    self.config = config;
    Ok(())
  }

  fn init(&mut self, area: ratatui::layout::Rect) -> Result<()> {
    // Initialize the editor backend
    self.editor_backend.init(area)?;
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
    // Global navigation keys (work from any component except when editing text)
    let is_editing = self.selected_component == ComponentKind::Query
      && self.selected_tab == 0
      && self.editor_backend.mode() == Mode::Insert;

    // Global key handling for error dismissal, help, and popups (highest priority)
    if let KeyCode::Esc = key.code {
      if self.error_message.is_some() {
        self.error_message = None;
        return Ok(None);
      }
      if self.show_help {
        self.show_help = false;
        return Ok(None);
      }
      if self.show_table_columns || self.show_table_schema {
        self.show_table_columns = false;
        self.show_table_schema = false;
        self.table_info_scroll = 0;
        return Ok(None);
      }
    }

    // Handle keys when table info popup is open
    if self.show_table_columns || self.show_table_schema {
      match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
          if self.show_table_schema {
            self.table_info_scroll = self.table_info_scroll.saturating_add(1);
          }
          return Ok(None);
        },
        KeyCode::Up | KeyCode::Char('k') => {
          if self.show_table_schema {
            self.table_info_scroll = self.table_info_scroll.saturating_sub(1);
          }
          return Ok(None);
        },
        KeyCode::PageDown => {
          if self.show_table_schema {
            self.table_info_scroll = self.table_info_scroll.saturating_add(10);
          }
          return Ok(None);
        },
        KeyCode::PageUp => {
          if self.show_table_schema {
            self.table_info_scroll = self.table_info_scroll.saturating_sub(10);
          }
          return Ok(None);
        },
        KeyCode::Char('c') => {
          // Allow toggling between column and schema view
          self.show_table_columns = !self.show_table_columns;
          self.show_table_schema = false;
          if self.show_table_columns && self.selected_table_columns.is_empty() {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              return Ok(Some(Action::LoadTable(selected_table.name.clone())));
            }
          }
          return Ok(None);
        },
        KeyCode::Char('s') => {
          // Allow toggling between schema and column view
          self.show_table_schema = !self.show_table_schema;
          self.show_table_columns = false;
          self.table_info_scroll = 0;
          if self.show_table_schema {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              self.selected_table_schema = self.generate_table_schema(selected_table);
            }
          }
          return Ok(None);
        },
        _ => {
          // Block all other keys when popup is open
          return Ok(None);
        },
      }
    }
    
    // Show help overlay
    if let KeyCode::Char('?') = key.code {
      if !is_editing && !self.is_searching_tables && !self.is_searching_results {
        self.show_help = !self.show_help;
        return Ok(None);
      }
    }

    if !is_editing && !self.is_searching_tables {
      match key.code {
        KeyCode::Char('1') => {
          self.selected_component = ComponentKind::Home;
          return Ok(Some(Action::SelectComponent(ComponentKind::Home)));
        },
        KeyCode::Char('2') => {
          self.selected_component = ComponentKind::Query;
          return Ok(Some(Action::SelectComponent(ComponentKind::Query)));
        },
        KeyCode::Char('3') => {
          self.selected_component = ComponentKind::Results;
          return Ok(Some(Action::SelectComponent(ComponentKind::Results)));
        },
        _ => {},
      }
    }

    match self.selected_component {
      ComponentKind::Home => {
        // Searching for a table
        match key.code {
          KeyCode::Char(c) => {
            if c == '/' {
              self.is_searching_tables = true;
            } else if self.is_searching_tables && c != '/' && !"123".contains(c) {
              // Allow typing search but prevent global navigation keys during search
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
            if self.is_searching_tables {
              self.table_search_query.pop();
              return Ok(Some(Action::LoadTables(self.table_search_query.clone())));
            }
          },
          KeyCode::Esc => {
            if self.is_searching_tables {
              // Clear search and exit search mode
              self.table_search_query.clear();
              self.is_searching_tables = false;
              return Ok(Some(Action::LoadTables(String::new())));
            }
          },
          _ => {},
        }
        
        // Handle non-search keys for Home component
        if !self.is_searching_tables {
          match key.code {
            KeyCode::Char('c') => {
              return Ok(Some(Action::ViewTableColumns));
            },
            KeyCode::Char('s') => {
              return Ok(Some(Action::ViewTableSchema));
            },
            _ => {},
          }
        }
      },
      ComponentKind::Query => {
        // Handle tab switching first (works in all modes except insert mode)
        if self.editor_backend.mode() != Mode::Insert {
          match key.code {
            KeyCode::Char('t') => {
              // Toggle between Query (0) and History (1) tabs
              self.selected_tab = if self.selected_tab == 0 { 1 } else { 0 };
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
            _ => {},
          }
        }

        // Handle history tab navigation
        if self.selected_tab == 1 {
          match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
              // Move up in display (toward more recent) = decrease index in reversed list
              if !self.query_history.is_empty() {
                if self.selected_history_index > 0 {
                  self.selected_history_index -= 1;
                } else {
                  self.selected_history_index = self.query_history.len() - 1; // Wrap to bottom
                }
              }
              return Ok(None);
            },
            KeyCode::Down | KeyCode::Char('j') => {
              // Move down in display (toward older) = increase index in reversed list
              if !self.query_history.is_empty() {
                if self.selected_history_index < self.query_history.len() - 1 {
                  self.selected_history_index += 1;
                } else {
                  self.selected_history_index = 0; // Wrap to top
                }
              }
              return Ok(None);
            },
            KeyCode::Enter => {
              // Execute selected history item
              // Convert display index to actual array index (most recent = index 0 in display)
              if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() {
                let actual_index = self.query_history.len() - 1 - self.selected_history_index;
                if let Some(entry) = self.query_history.get(actual_index) {
                  return Ok(Some(Action::HandleQuery(entry.query.clone())));
                }
              }
              return Ok(None);
            },
            KeyCode::Char('c') => {
              // Copy selected history item to query editor
              if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() {
                let actual_index = self.query_history.len() - 1 - self.selected_history_index;
                if let Some(entry) = self.query_history.get(actual_index) {
                  self.editor_backend.set_text(&entry.query);
                  self.selected_tab = 0; // Switch back to query tab
                }
              }
              return Ok(None);
            },
            KeyCode::Char('y') => {
              // Copy selected history item to clipboard
              if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() {
                let actual_index = self.query_history.len() - 1 - self.selected_history_index;
                if let Some(entry) = self.query_history.get(actual_index) {
                  let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                  if let Err(e) = ctx.set_contents(entry.query.clone()) {
                    eprintln!("Failed to copy to clipboard: {e}");
                  }
                }
              }
              return Ok(None);
            },
            KeyCode::Char('d') => {
              // Delete selected history item
              if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() {
                let actual_index = self.query_history.len() - 1 - self.selected_history_index;
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
          // Handle autocomplete navigation first
          if self.autocomplete_state.is_active {
            match key.code {
              KeyCode::Tab | KeyCode::Down => {
                self.autocomplete_state.select_next();
                return Ok(None);
              },
              KeyCode::BackTab | KeyCode::Up => {
                self.autocomplete_state.select_previous();
                return Ok(None);
              },
              KeyCode::Enter => {
                // Only apply suggestion if we have valid suggestions and user is in insert mode
                if self.editor_backend.mode() == Mode::Insert
                  && !self.autocomplete_state.suggestions.is_empty()
                  && self.autocomplete_state.selected_index < self.autocomplete_state.suggestions.len()
                {
                  if let Some(suggestion) = self.autocomplete_state.get_selected_suggestion() {
                    self.apply_autocomplete_suggestion(suggestion.clone());
                    self.autocomplete_state.deactivate();
                    return Ok(None);
                  }
                }
                // Otherwise, deactivate autocomplete and let Enter work normally
                self.autocomplete_state.deactivate();
              },
              KeyCode::Esc => {
                self.autocomplete_state.deactivate();
                return Ok(None);
              },
              _ => {
                // For other keys in insert mode, continue normal processing
                // but deactivate autocomplete to avoid interference
                if self.editor_backend.mode() == Mode::Insert {
                  self.autocomplete_state.deactivate();
                }
              },
            }
          }

          // Handle manual autocomplete trigger (Ctrl+Space)
          if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) && self.editor_backend.mode() == Mode::Insert {
            return Ok(Some(Action::TriggerAutocomplete));
          }

          // Delegate key handling to the editor backend
          if let Ok(Some(action)) = self.editor_backend.handle_key_event(key) {
            // If in insert mode and using LSP, send document update
            if self.editor_backend.mode() == Mode::Insert {
              match self.autocomplete_engine.backend_name() {
                "lsp" | "hybrid" => {
                  // Send document update action for LSP synchronization
                  let text = self.editor_backend.get_text();
                  self.command_tx.as_ref().map(|tx| {
                    let _ = tx.send(Action::UpdateAutocompleteDocument(text));
                  });
                }
                _ => {}
              }
            }
            return Ok(Some(action));
          }

          // Handle query execution for Enter key in normal mode
          if key.code == KeyCode::Enter && self.editor_backend.mode() == Mode::Normal {
            let query_text = self.editor_backend.get_text();
            let trimmed_query = query_text.trim();
            if !trimmed_query.is_empty() {
              return Ok(Some(Action::HandleQuery(trimmed_query.to_string())));
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
        // Handle search mode first
        if self.is_searching_results {
          match key.code {
            KeyCode::Esc => {
              // Exit search mode and clear search
              self.is_searching_results = false;
              self.results_search_query.clear();
              self.filter_results_fuzzy();
              return Ok(None);
            },
            KeyCode::Enter => {
              // Confirm search (stay in filtered view)
              self.is_searching_results = false;
              return Ok(None);
            },
            KeyCode::Backspace => {
              // Delete character from search query
              self.results_search_query.pop();
              self.filter_results_fuzzy();
              return Ok(None);
            },
            KeyCode::Char(c) => {
              // Add character to search query
              self.results_search_query.push(c);
              self.filter_results_fuzzy();
              return Ok(None);
            },
            _ => return Ok(None),
          }
        }

        // Handle cell selection mode navigation first
        if self.selection_mode == SelectionMode::Cell {
          // Handle all keys in cell selection mode to prevent them from triggering other actions
          match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
              if self.selected_cell_index > 0 {
                self.selected_cell_index -= 1;
                // Auto-scroll if needed
                let cell_page = self.selected_cell_index / VISIBLE_COLUMNS;
                if cell_page < self.horizonal_scroll_offset {
                  self.horizonal_scroll_offset = cell_page;
                }
              }
              return Ok(Some(Action::Render)); // Force render but consume the key
            },
            KeyCode::Right | KeyCode::Char('l') => {
              if let Some(row) = self.get_selected_row() {
                if self.selected_cell_index < row.len() - 1 {
                  self.selected_cell_index += 1;
                  // Auto-scroll if needed
                  let cell_page = self.selected_cell_index / VISIBLE_COLUMNS;
                  if cell_page > self.horizonal_scroll_offset {
                    self.horizonal_scroll_offset = cell_page;
                  }
                }
              }
              return Ok(Some(Action::Render)); // Force render but consume the key
            },
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
              // Allow these to be handled by the normal row navigation
              // Don't return here, let them fall through
            },
            KeyCode::Esc => {
              // Exit cell selection mode
              self.selection_mode = SelectionMode::Table;
              return Ok(Some(Action::Render));
            },
            KeyCode::Char('y') => {
              // Copy in cell mode
              if let Some(content) = self.get_copy_content(false, false) {
                let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                ctx.set_contents(content).unwrap();
              }
              return Ok(None);
            },
            _ => {
              // Consume all other keys in cell selection mode
              return Ok(None);
            },
          }
        }

        // Normal results mode key handling
        match key.code {
          KeyCode::Char('/') => {
            // Enter search mode
            self.is_searching_results = true;
            self.results_search_query.clear();
            self.filter_results_fuzzy(); // Initialize with all results
            return Ok(None);
          },
          KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Copy as JSON
            if let Some(content) = self.get_copy_content(true, true) {
              let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
              ctx.set_contents(content).unwrap();
            }
          },
          KeyCode::Char('y') => {
            // Copy current cell or row based on selection mode
            if let Some(content) = self.get_copy_content(false, false) {
              let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
              ctx.set_contents(content).unwrap();
            }
          },
          KeyCode::Char('Y') => {
            // Copy entire row as TSV
            if let Some(content) = self.get_copy_content(true, false) {
              let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
              ctx.set_contents(content).unwrap();
            }
          },
          KeyCode::Char('r') => {
            return Ok(Some(Action::HandleQuery(self.editor_backend.get_text())));
          },
          KeyCode::Char(' ') => {
            self.selection_mode = match self.selection_mode {
              SelectionMode::Table => SelectionMode::Row,
              SelectionMode::Row => SelectionMode::Table,
              _ => SelectionMode::Table,
            };
          },
          KeyCode::Char('p') => {
            // Toggle preview popup
            self.selection_mode = match self.selection_mode {
              SelectionMode::Preview => SelectionMode::Table,
              _ => SelectionMode::Preview,
            };
          },
          KeyCode::Char('v') => {
            // Enter cell selection mode
            self.selection_mode = SelectionMode::Cell;
            // Start at the first visible column
            self.selected_cell_index = self.horizonal_scroll_offset * VISIBLE_COLUMNS;
          },
          KeyCode::Esc => {
            // Exit any special selection mode
            if self.selection_mode != SelectionMode::Table {
              self.selection_mode = SelectionMode::Table;
              return Ok(None);
            }
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

        // Update autocomplete provider with all tables
        self.autocomplete_engine.update_tables(self.tables.clone());

        // Reset table selection index to prevent out-of-bounds access after filtering
        if !self.tables.is_empty() {
          self.selected_table_index = self.selected_table_index.min(self.tables.len() - 1);
        } else {
          self.selected_table_index = 0;
        }
      },
      Action::TableMoveDown => {
        if !self.tables.is_empty() {
          if self.selected_table_index < self.tables.len() - 1 {
            self.selected_table_index += 1;
          } else {
            self.selected_table_index = 0; // Wrap to top
          }
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      Action::TableMoveUp => {
        if !self.tables.is_empty() {
          if self.selected_table_index > 0 {
            self.selected_table_index -= 1;
          } else {
            self.selected_table_index = self.tables.len() - 1; // Wrap to bottom
          }
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      Action::RowMoveDown => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          match self.selection_mode {
            SelectionMode::Table => {
              // Navigate through filtered result rows
              if !self.filtered_results.is_empty() {
                if self.filtered_results_index < self.filtered_results.len() - 1 {
                  self.filtered_results_index += 1;
                } else {
                  self.filtered_results_index = 0; // Wrap to top
                }
                // Update the actual selected row index
                self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
              }
            },
            SelectionMode::Cell => {
              // Move to next row in cell selection mode
              if !self.filtered_results.is_empty() {
                if self.filtered_results_index < self.filtered_results.len() - 1 {
                  self.filtered_results_index += 1;
                  self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
                }
              } else if self.selected_row_index < self.query_results.len() - 1 {
                self.selected_row_index += 1;
              }
            },
            SelectionMode::Row => {
              // Navigate through detail columns
              if self.detail_row_index < self.query_results[self.selected_row_index].len() - 1 {
                self.detail_row_index += 1;
              } else {
                self.detail_row_index = 0; // Wrap to top
              }
            },
            _ => {},
          }
        }
      },
      Action::RowMoveUp => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          match self.selection_mode {
            SelectionMode::Table => {
              // Navigate through filtered result rows
              if !self.filtered_results.is_empty() {
                if self.filtered_results_index > 0 {
                  self.filtered_results_index -= 1;
                } else {
                  self.filtered_results_index = self.filtered_results.len() - 1;
                  // Wrap to bottom
                }
                // Update the actual selected row index
                self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
              }
            },
            SelectionMode::Cell => {
              // Move to previous row in cell selection mode
              if !self.filtered_results.is_empty() {
                if self.filtered_results_index > 0 {
                  self.filtered_results_index -= 1;
                  self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
                }
              } else if self.selected_row_index > 0 {
                self.selected_row_index -= 1;
              }
            },
            SelectionMode::Row => {
              // Navigate through detail columns
              if self.detail_row_index > 0 {
                self.detail_row_index -= 1;
              } else {
                self.detail_row_index = self.query_results[self.selected_row_index].len() - 1;
                // Wrap to bottom
              }
            },
            _ => {},
          }
        }
      },
      Action::ScrollTableLeft => {
        if self.selected_component == ComponentKind::Results && self.selection_mode != SelectionMode::Cell {
          // Normal horizontal scrolling (not in cell selection mode)
          if self.horizonal_scroll_offset > 0 {
            self.horizonal_scroll_offset -= 1;
          }
        }
      },
      Action::ScrollTableRight => {
        if self.selected_component == ComponentKind::Results && self.selection_mode != SelectionMode::Cell {
          // Normal horizontal scrolling (not in cell selection mode)
          if self.column_count() > 0
            && self.horizonal_scroll_offset * VISIBLE_COLUMNS < self.column_count() - VISIBLE_COLUMNS
          {
            self.horizonal_scroll_offset += 1;
          }
        }
      },
      Action::LoadSelectedTable => {
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          let query = format!("SELECT * FROM {}", selected_table.name);
          self.editor_backend.set_text(&query);
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

        // Reset search state
        self.is_searching_results = false;
        self.results_search_query.clear();
        self.filtered_results_index = 0;

        // Initialize filtered results with all rows
        self.filter_results_fuzzy();

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
        // Auto-format before execution if enabled
        if self.editor_backend.is_auto_format_enabled() {
          // Try to format, but continue even if formatting fails
          let _ = self.editor_backend.format_all();
        }
        
        // Execute query text from editor backend
        let query_text = if let Some(selected_text) = self.editor_backend.get_selected_text() {
          selected_text.trim().to_string()
        } else {
          self.editor_backend.get_text()
        };

        let cleaned_query = query_text.trim();

        // Only execute if query is not empty
        if !cleaned_query.is_empty() {
          // Store the query for history tracking
          self.last_executed_query = Some(cleaned_query.to_string());
          return Ok(Some(Action::HandleQuery(cleaned_query.to_string())));
        }
      },
      Action::QueryStarted => {
        self.is_query_running = true;
        self.query_start_time = Some(std::time::Instant::now());
        self.error_message = None;
      },
      Action::QueryCompleted => {
        self.is_query_running = false;
        self.query_start_time = None;
      },
      Action::RowDetails => {
        self.show_row_details = !self.show_row_details;
      },
      Action::Error(e) => {
        self.error_message = Some(e);
        self.is_query_running = false;
        self.query_start_time = None;

        // Add failed query to history (but don't save failed queries)
        if let Some(_query) = self.last_executed_query.take() {
          // We could optionally add failed queries with success=false
          // For now, we only add successful queries to history
          // self.add_to_history(&query, false);
        }
      },
      Action::ClearQuery => {
        // Clear the query editor
        self.editor_backend.set_text("");
      },
      Action::TriggerAutocomplete => {
        // Trigger autocomplete with current context
        return Ok(self.trigger_autocomplete());
      },
      Action::UpdateAutocompleteDocument(_text) => {
        // For now, we'll ignore LSP document updates since they need async handling
        // This would be properly implemented with a channel to communicate back
        // to the main thread when the update is complete
      },
      Action::AutocompleteResults(results) => {
        eprintln!("Received {} autocomplete results", results.len());
        
        // Convert results back to SuggestionItems
        let suggestions: Vec<crate::autocomplete::SuggestionItem> = results
          .into_iter()
          .map(|(text, kind)| {
            let suggestion_kind = match kind.as_str() {
              "table" => crate::autocomplete::SuggestionKind::Table,
              "column" => crate::autocomplete::SuggestionKind::Column,
              _ => crate::autocomplete::SuggestionKind::Keyword,
            };
            crate::autocomplete::SuggestionItem {
              text,
              kind: suggestion_kind,
              score: 100, // Default score
              table_context: None,
            }
          })
          .collect();
        
        // Update autocomplete state with LSP results
        if self.autocomplete_state.is_active {
          self.autocomplete_state.update_suggestions(suggestions);
        }
      },
      Action::SetTunnelMode(is_tunnel) => {
        if is_tunnel && !matches!(self.autocomplete_engine.backend_name(), "builtin") {
          eprintln!("Tunnel mode detected - switching to builtin autocomplete for better compatibility");
          self.autocomplete_engine = AutocompleteEngine::new_builtin();
          
          // Re-populate the engine with current tables
          self.autocomplete_engine.update_tables(self.tables.clone());
          
          // Update cached columns if any
          for (table_name, columns) in &self.table_columns_cache {
            self.autocomplete_engine.update_table_columns(table_name.clone(), columns.clone());
          }
          
          eprintln!("Switched to {} autocomplete", self.autocomplete_engine.backend_name());
        }
      },
      Action::TableColumnsLoaded(table_name, columns) => {
        // Cache columns for autocomplete and future use
        self.table_columns_cache.insert(table_name.clone(), columns.clone());
        self.autocomplete_engine.update_table_columns(table_name.clone(), columns.clone());
        
        // Store the loaded columns if column view is active
        if self.show_table_columns {
          if let Some(selected_table) = self.tables.get(self.selected_table_index) {
            if selected_table.name == table_name {
              self.selected_table_columns = columns;
            }
          }
        }
        
        // If schema view was requested and this is the selected table, generate schema now
        if self.show_table_schema {
          if let Some(selected_table) = self.tables.get(self.selected_table_index) {
            if selected_table.name == table_name {
              self.selected_table_schema = self.generate_table_schema(selected_table);
            }
          }
        }
      },
      Action::ViewTableColumns => {
        // Toggle column view for selected table
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          if self.show_table_columns {
            // Hide columns
            self.show_table_columns = false;
            self.selected_table_columns.clear();
          } else {
            // Show columns - we'll need to load them asynchronously
            self.show_table_columns = true;
            self.show_table_schema = false; // Hide schema if it was shown
            self.table_info_scroll = 0; // Reset scroll
            // Return an action to load columns (will be handled in app.rs)
            return Ok(Some(Action::LoadTable(selected_table.name.clone())));
          }
        }
      },
      Action::ViewTableSchema => {
        // Toggle schema view for selected table
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          if self.show_table_schema {
            // Hide schema
            self.show_table_schema = false;
            self.selected_table_schema.clear();
          } else {
            // Show schema
            self.show_table_schema = true;
            self.show_table_columns = false; // Hide columns if they were shown
            self.table_info_scroll = 0; // Reset scroll
            
            // Check if we have columns cached for this table
            if !self.table_columns_cache.contains_key(&selected_table.name) {
              // Need to load columns first, then generate schema
              return Ok(Some(Action::LoadTable(selected_table.name.clone())));
            }
            
            // Generate schema information
            self.selected_table_schema = self.generate_table_schema(selected_table);
          }
        }
      },
      Action::ExportResultsToCsv => {
        if !self.query_results.is_empty() {
          if let Err(e) = self.export_results_to_csv() {
            self.error_message = Some(format!("Failed to export CSV: {}", e));
          }
        } else {
          self.error_message = Some("No results to export".to_string());
        }
      },
      Action::FormatQuery => {
        if self.selected_component == ComponentKind::Query {
          if let Err(e) = self.editor_backend.format_all() {
            self.error_message = Some(e);
          } else {
            self.error_message = Some("Query formatted".to_string());
          }
        }
      },
      Action::FormatSelection => {
        if self.selected_component == ComponentKind::Query {
          if let Err(e) = self.editor_backend.format_query(true) {
            self.error_message = Some(e);
          } else {
            self.error_message = Some("Selection formatted".to_string());
          }
        }
      },
      Action::ToggleAutoFormat => {
        self.editor_backend.toggle_auto_format();
        let enabled = self.editor_backend.is_auto_format_enabled();
        self.error_message = Some(format!("Auto-format {}", if enabled { "enabled" } else { "disabled" }));
      },
      Action::RowJumpToTop => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          if !self.filtered_results.is_empty() {
            // Jump to first filtered result
            self.filtered_results_index = 0;
            self.selected_row_index = self.filtered_results[0].0;
          } else {
            // Jump to first row
            self.selected_row_index = 0;
          }
        }
      },
      Action::RowJumpToBottom => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          if !self.filtered_results.is_empty() {
            // Jump to last filtered result
            self.filtered_results_index = self.filtered_results.len() - 1;
            self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
          } else {
            // Jump to last row
            self.selected_row_index = self.query_results.len() - 1;
          }
        }
      },
      Action::TableJumpToTop => {
        if !self.tables.is_empty() {
          self.selected_table_index = 0;
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.first() {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      Action::TableJumpToBottom => {
        if !self.tables.is_empty() {
          self.selected_table_index = self.tables.len() - 1;
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.last() {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      Action::RowPageUp => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          // Calculate page size (roughly 10 rows per page as a reasonable default)
          let page_size = 10;
          
          if !self.filtered_results.is_empty() {
            // Page up in filtered results
            if self.filtered_results_index >= page_size {
              self.filtered_results_index -= page_size;
            } else {
              self.filtered_results_index = 0;
            }
            self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
          } else {
            // Page up in all results
            if self.selected_row_index >= page_size {
              self.selected_row_index -= page_size;
            } else {
              self.selected_row_index = 0;
            }
          }
        }
      },
      Action::RowPageDown => {
        if !self.query_results.is_empty() && self.selected_component == ComponentKind::Results {
          // Calculate page size (roughly 10 rows per page as a reasonable default)
          let page_size = 10;
          
          if !self.filtered_results.is_empty() {
            // Page down in filtered results
            let max_index = self.filtered_results.len() - 1;
            if self.filtered_results_index + page_size <= max_index {
              self.filtered_results_index += page_size;
            } else {
              self.filtered_results_index = max_index;
            }
            self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
          } else {
            // Page down in all results
            let max_index = self.query_results.len() - 1;
            if self.selected_row_index + page_size <= max_index {
              self.selected_row_index += page_size;
            } else {
              self.selected_row_index = max_index;
            }
          }
        }
      },
      Action::TablePageUp => {
        if !self.tables.is_empty() {
          let page_size = 10;
          if self.selected_table_index >= page_size {
            self.selected_table_index -= page_size;
          } else {
            self.selected_table_index = 0;
          }
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      Action::TablePageDown => {
        if !self.tables.is_empty() {
          let page_size = 10;
          let max_index = self.tables.len() - 1;
          if self.selected_table_index + page_size <= max_index {
            self.selected_table_index += page_size;
          } else {
            self.selected_table_index = max_index;
          }
          
          // Update popup content if it's visible
          if self.show_table_columns || self.show_table_schema {
            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
              // Check if we have cached columns for this table
              if let Some(columns) = self.table_columns_cache.get(&selected_table.name) {
                if self.show_table_columns {
                  self.selected_table_columns = columns.clone();
                }
                if self.show_table_schema {
                  self.selected_table_schema = self.generate_table_schema(selected_table);
                }
              } else {
                // Need to load columns for this table
                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
              }
            }
          }
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, _area: Rect) -> Result<()> {
    // Create the layout sections.
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Length(3), Constraint::Min(1)])
      .split(f.area());

    let title_block = Block::default()
      .borders(Borders::ALL)
      .border_style(theme::border_normal())
      .border_type(BorderType::Rounded)
      .style(theme::bg_primary());

    let title =
      Paragraph::new(Text::styled("Query Crafter - [1] Tables [2] Query [3] Results", theme::title()))
        .block(title_block);

    f.render_widget(title, chunks[0]);

    let table_chunks = self.render_table_list(f, chunks)?;

    let query_chunks = self.render_query_input(f, table_chunks)?;

    self.render_query_results(f, query_chunks)?;

    self.render_error(f)?;

    self.render_help(f)?;
    
    // Render table info popup last so it appears on top
    self.render_table_info_popup(f)?;

    Ok(())
  }
}

// Additional impl block for autocomplete functionality
impl Db {
  /// Manually triggers autocomplete at the current cursor position
  fn trigger_autocomplete(&mut self) -> Option<Action> {
    // Get text up to cursor position for context analysis
    let text_up_to_cursor = self.editor_backend.get_text_up_to_cursor();
    let full_text = self.editor_backend.get_text();
    let cursor_pos = text_up_to_cursor.len();

    // Use the SQL parser to analyze context
    let (context, current_word) = crate::autocomplete::SqlParser::analyze_context(&text_up_to_cursor, cursor_pos);

    // Always show suggestions when manually triggered, even for empty current word
    self.autocomplete_state.activate(cursor_pos, current_word.clone());
    
    // Get cursor position for LSP
    let (cursor_line, cursor_col) = self.editor_backend.get_cursor_position();
    
    match self.autocomplete_engine.backend_mut() {
      crate::autocomplete_engine::AutocompleteBackend::Builtin(provider) => {
        // Builtin provider can work synchronously
        let suggestions = provider.get_suggestions(context, &current_word);
        self.autocomplete_state.update_suggestions(suggestions);
        None
      }
      crate::autocomplete_engine::AutocompleteBackend::Lsp(_) => {
        // LSP requires async, send action to handle in main loop
        eprintln!("Requesting LSP autocomplete...");
        Some(Action::RequestAutocomplete {
          text: full_text,
          cursor_line,
          cursor_col,
          context: format!("{:?}", context), // Simple serialization
        })
      }
      crate::autocomplete_engine::AutocompleteBackend::Hybrid { builtin, .. } => {
        // In hybrid mode, try builtin first, then request LSP
        let suggestions = builtin.get_suggestions(context.clone(), &current_word);
        self.autocomplete_state.update_suggestions(suggestions);
        
        // Also request LSP suggestions
        Some(Action::RequestAutocomplete {
          text: full_text,
          cursor_line,
          cursor_col,
          context: format!("{:?}", context),
        })
      }
    }
  }

  /// Applies the selected autocomplete suggestion to the editor
  fn apply_autocomplete_suggestion(&mut self, suggestion: crate::autocomplete::SuggestionItem) {
    // Delete the current partial word before cursor
    if !self.autocomplete_state.current_word.is_empty() {
      self.editor_backend.delete_word_before_cursor();
    }

    // Insert the suggestion at the cursor position
    self.editor_backend.insert_text_at_cursor(&suggestion.text);
  }

  /// Renders the autocomplete popup widget
  fn render_autocomplete_popup(&mut self, f: &mut Frame<'_>, editor_area: Rect) -> Result<()> {
    if !self.autocomplete_state.is_active || self.autocomplete_state.suggestions.is_empty() {
      return Ok(());
    }

    // Create a popup positioned near the cursor (simplified positioning)
    let popup_height = std::cmp::min(10, self.autocomplete_state.suggestions.len() as u16 + 2);
    let popup_width = 40;

    // Position popup in the lower part of the editor area
    let popup_area = Rect {
      x: editor_area.x + 2,
      y: editor_area.y + editor_area.height.saturating_sub(popup_height + 1),
      width: std::cmp::min(popup_width, editor_area.width.saturating_sub(4)),
      height: popup_height,
    };

    // Clear the area first
    f.render_widget(Clear, popup_area);

    // Use the autocomplete popup widget
    let autocomplete_popup = crate::autocomplete_widget::AutocompletePopup::new(&self.autocomplete_state)
      .max_height(popup_height.saturating_sub(2))
      .max_width(popup_width);

    f.render_widget(autocomplete_popup, popup_area);

    Ok(())
  }
  

  /// Render table information popup (columns or schema)
  fn render_table_info_popup(&mut self, f: &mut Frame<'_>) -> Result<()> {
    if !self.show_table_columns && !self.show_table_schema {
      return Ok(());
    }

    // Create a centered popup area (80% width, 70% height)
    let area = self.centered_rect(80, 70, f.area());
    
    // Clear the area behind the popup
    f.render_widget(Clear, area);
    
    let title = if self.show_table_columns {
      format!("Table Columns - {} (ESC to close)", 
        self.tables.get(self.selected_table_index).map(|t| &t.name).unwrap_or(&"Unknown".to_string()))
    } else if self.show_table_schema {
      format!("Table Schema - {} (ESC to close)", 
        self.tables.get(self.selected_table_index).map(|t| &t.name).unwrap_or(&"Unknown".to_string()))
    } else {
      "Table Info".to_string()
    };
    
    let block = Block::default()
      .borders(Borders::ALL)
      .border_style(theme::border_focused())
      .title(title)
      .title_style(theme::title())
      .border_type(BorderType::Rounded);
    
    if self.show_table_columns && !self.selected_table_columns.is_empty() {
      // Create a table widget for columns
      let header_cells = ["Column Name", "Data Type", "Nullable"]
        .iter()
        .map(|h| Cell::from(*h).style(theme::header()));
      let header = Row::new(header_cells)
        .style(theme::bg_primary())
        .height(1)
        .bottom_margin(1);
      
      let rows = self.selected_table_columns.iter().map(|col| {
        let nullable = if col.is_nullable { "YES" } else { "NO" };
        let cells = [
          Cell::from(col.name.clone()),
          Cell::from(col.data_type.clone()),
          Cell::from(nullable),
        ];
        Row::new(cells).height(1)
      });
      
      let table = Table::new(rows, [Constraint::Percentage(40), Constraint::Percentage(40), Constraint::Percentage(20)])
        .header(header)
        .block(block)
        .style(theme::bg_primary())
        .row_highlight_style(theme::selection_active())
        .widths(&[Constraint::Percentage(40), Constraint::Percentage(40), Constraint::Percentage(20)]);
      
      f.render_widget(table, area);
    } else if self.show_table_schema {
      // Show schema as scrollable text
      let paragraph = Paragraph::new(self.selected_table_schema.as_str())
        .block(block)
        .style(theme::bg_primary())
        .wrap(Wrap { trim: false })
        .scroll((self.table_info_scroll, 0));
      f.render_widget(paragraph, area);
    } else if self.show_table_columns {
      // Loading message
      let loading_msg = Paragraph::new("Loading columns...")
        .block(block)
        .style(theme::muted())
        .alignment(Alignment::Center);
      f.render_widget(loading_msg, area);
    }
    
    Ok(())
  }
  
  /// Generate schema information for a table
  fn generate_table_schema(&self, table: &DbTable) -> String {
    let mut schema_info = format!("═══ TABLE SCHEMA ═══\n\n");
    schema_info.push_str(&format!("Table: {}.{}\n", table.schema, table.name));
    schema_info.push_str(&format!("{}\n\n", "─".repeat(50)));
    
    // Get columns from cache or table
    let columns = self.table_columns_cache.get(&table.name)
      .map(|c| c.as_slice())
      .unwrap_or(&table.columns);
    
    if !columns.is_empty() {
      // Generate CREATE TABLE statement
      schema_info.push_str("CREATE TABLE Statement:\n");
      schema_info.push_str(&format!("{}\n", "─".repeat(50)));
      schema_info.push_str(&format!("CREATE TABLE {} (\n", table.name));
      
      for (i, col) in columns.iter().enumerate() {
        let nullable = if col.is_nullable { "" } else { " NOT NULL" };
        let comma = if i < columns.len() - 1 { "," } else { "" };
        schema_info.push_str(&format!("    {} {}{}{}\n", col.name, col.data_type, nullable, comma));
      }
      schema_info.push_str(");\n\n");
      
      // Column Details
      schema_info.push_str("Column Details:\n");
      schema_info.push_str(&format!("{}\n", "─".repeat(50)));
      schema_info.push_str(&format!("{:<20} {:<20} {:<10}\n", "Name", "Type", "Nullable"));
      schema_info.push_str(&format!("{:<20} {:<20} {:<10}\n", "────", "────", "────────"));
      
      for col in columns {
        let nullable = if col.is_nullable { "YES" } else { "NO" };
        schema_info.push_str(&format!("{:<20} {:<20} {:<10}\n", col.name, col.data_type, nullable));
      }
      
      schema_info.push_str(&format!("\n{}\n", "─".repeat(50)));
      
      // Add placeholders for additional info
      schema_info.push_str("\nIndexes:\n");
      schema_info.push_str("  (Index information not available in current implementation)\n");
      
      schema_info.push_str("\nForeign Keys:\n");
      schema_info.push_str("  (Foreign key information not available in current implementation)\n");
      
      schema_info.push_str("\nConstraints:\n");
      schema_info.push_str("  (Constraint information not available in current implementation)\n");
      
      schema_info.push_str(&format!("\n{}\n", "─".repeat(50)));
      schema_info.push_str(&format!("Total Columns: {}\n", columns.len()));
      
      // Add note about getting full schema
      schema_info.push_str("\nNote: For complete schema information including indexes,\n");
      schema_info.push_str("foreign keys, and constraints, execute a schema query\n");
      schema_info.push_str("in the Query panel (e.g., \\d table_name for PostgreSQL).\n");
    } else {
      schema_info.push_str("\nColumn information not loaded.\n");
      schema_info.push_str("Press 'c' to load column details first.\n");
    }
    
    schema_info
  }
  
  /// Export query results to CSV file
  fn export_results_to_csv(&mut self) -> Result<()> {
    use std::io::Write;
    
    // Generate filename based on current timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("query_results_{}.csv", timestamp);
    
    // Create or open the file
    let mut file = fs::File::create(&filename)?;
    
    // Write headers
    let headers = self.selected_headers.join(",");
    writeln!(file, "{}", headers)?;
    
    // Write data rows
    for row in &self.query_results {
      // Escape fields that contain commas, quotes, or newlines
      let escaped_row: Vec<String> = row.iter().map(|field| {
        if field.contains(',') || field.contains('"') || field.contains('\n') {
          // Escape quotes by doubling them and wrap in quotes
          format!("\"{}\"", field.replace('"', "\"\""))
        } else {
          field.clone()
        }
      }).collect();
      
      let row_str = escaped_row.join(",");
      writeln!(file, "{}", row_str)?;
    }
    
    // Store success status with timestamp
    self.export_status = Some((
      format!("Exported to: {}", filename),
      std::time::Instant::now()
    ));
    
    Ok(())
  }
}
