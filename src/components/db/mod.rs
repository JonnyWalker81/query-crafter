pub mod editor;
pub mod handlers;
pub mod helpers;
pub mod models;
pub mod rendering;
pub mod state;

// Re-export commonly used types
pub use models::{DbColumn, DbTable, QueryHistoryEntry, SelectionMode};

use std::{
    collections::HashMap,
    time::Instant,
};

use color_eyre::eyre::Result;
use nucleo;
use ratatui::prelude::*;
use tokio::sync::mpsc::UnboundedSender;

use self::editor::EditorBackend;
use super::{Component, ComponentKind, Frame};
use crate::{
    action::Action,
    autocomplete::AutocompleteState,
    autocomplete_engine::AutocompleteEngine,
    config::Config,
};

const VISIBLE_COLUMNS: usize = 3;

pub struct Db {
    pub command_tx: Option<UnboundedSender<Action>>,
    pub config: Config,
    
    // Tables and navigation
    pub tables: Vec<DbTable>,
    pub selected_table_index: usize,
    pub is_searching_tables: bool,
    pub table_search_query: String,
    pub table_columns_cache: HashMap<String, Vec<DbColumn>>,
    
    // Query editor
    pub editor_backend: EditorBackend,
    pub query_history: Vec<QueryHistoryEntry>,
    pub selected_history_index: usize,
    pub selected_tab: usize,
    pub last_executed_query: Option<String>,
    
    // Query results
    pub selected_headers: Vec<String>,
    pub query_results: Vec<Vec<String>>,
    pub selected_row_index: usize,
    pub selected_cell_index: usize,
    pub detail_row_index: usize,
    pub horizonal_scroll_offset: usize,
    pub selection_mode: SelectionMode,
    
    // Results search/filter
    pub is_searching_results: bool,
    pub results_search_query: String,
    pub filtered_results: Vec<(usize, Vec<String>)>,
    pub filtered_results_index: usize,
    
    // Component state
    pub selected_component: ComponentKind,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub export_status: Option<(String, Instant)>,
    
    // Autocomplete
    pub autocomplete_state: AutocompleteState,
    pub autocomplete_engine: AutocompleteEngine,
    
    // Table info popup
    pub show_table_columns: bool,
    pub show_table_schema: bool,
    pub selected_table_columns: Vec<DbColumn>,
    pub selected_table_schema: String,
    pub table_info_scroll: usize,
    
    // Query loading state
    pub is_query_running: bool,
    pub query_start_time: Option<Instant>,
    pub last_query_execution_time: Option<u64>, // milliseconds from database
    
    // Preview popup state
    pub preview_scroll_offset: u16,
    pub preview_selected_index: usize,
    
    // Row details popup
    pub show_row_details: bool,
    
    // Results matcher for fuzzy search
    pub results_matcher: nucleo::Matcher,
    
    // EXPLAIN view state
    pub is_explain_view: bool,
    pub is_explain_query: bool,
}

impl Default for Db {
    fn default() -> Self {
        Self::new()
    }
}

impl Db {
    pub fn new() -> Self {
        Self::new_with_config(None)
    }
    
    pub fn new_with_config(config: Option<Config>) -> Self {
        let config = config.unwrap_or_default();
        Self {
            command_tx: None,
            config,
            tables: vec![],
            selected_table_index: 0,
            is_searching_tables: false,
            table_search_query: String::new(),
            table_columns_cache: HashMap::new(),
            editor_backend: EditorBackend::default(),
            query_history: helpers::load_query_history(),
            selected_history_index: 0,
            selected_tab: 0,
            last_executed_query: None,
            selected_headers: vec![],
            query_results: vec![],
            selected_row_index: 0,
            selected_cell_index: 0,
            detail_row_index: 0,
            horizonal_scroll_offset: 0,
            selection_mode: SelectionMode::Table,
            is_searching_results: false,
            results_search_query: String::new(),
            filtered_results: vec![],
            filtered_results_index: 0,
            selected_component: ComponentKind::Home,
            show_help: false,
            error_message: None,
            export_status: None,
            autocomplete_state: AutocompleteState::default(),
            autocomplete_engine: AutocompleteEngine::new_builtin(),
            show_table_columns: false,
            show_table_schema: false,
            selected_table_columns: vec![],
            selected_table_schema: String::new(),
            table_info_scroll: 0,
            is_query_running: false,
            query_start_time: None,
            last_query_execution_time: None,
            preview_scroll_offset: 0,
            preview_selected_index: 0,
            show_row_details: false,
            results_matcher: nucleo::Matcher::new(nucleo::Config::DEFAULT),
            is_explain_view: false,
            is_explain_query: false,
        }
    }
}

impl Component for Db {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.register_config_handler(config)
    }

    fn init(&mut self, area: Rect) -> Result<()> {
        self.editor_backend.init(area)?;
        Ok(())
    }

    fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        self.handle_events(event)
    }
    
    fn handle_key_events(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        self.handle_key_events(key)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        self.update(action)
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        self.draw(f, area)
    }
}