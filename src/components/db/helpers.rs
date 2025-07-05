use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono;
use color_eyre::eyre::Result;
use directories::ProjectDirs;
use ratatui::{
    prelude::*,
    widgets::*,
};
use serde_json;

use super::{Db, QueryHistoryEntry, DbTable};
use crate::{
    action::Action,
    autocomplete::{SuggestionItem, SqlParser},
    autocomplete_engine::AutocompleteBackend,
};
use query_crafter_theme as theme;

// Query history management
pub(super) fn load_query_history() -> Vec<QueryHistoryEntry> {
    let history_file_path = get_history_file_path();
    if let Ok(contents) = fs::read_to_string(&history_file_path) {
        if let Ok(history) = serde_json::from_str::<Vec<QueryHistoryEntry>>(&contents) {
            return history;
        }
    }
    Vec::new()
}

fn get_history_file_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "query-crafter", "query-crafter") {
        proj_dirs.data_dir().join("query_history.json")
    } else {
        PathBuf::from(".query_history.json")
    }
}

impl Db {

    pub(super) fn save_query_history(&self) {
        let history_file_path = get_history_file_path();
        
        // Ensure directory exists
        if let Some(parent) = history_file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Save history to file
        if let Ok(json) = serde_json::to_string_pretty(&self.query_history) {
            let _ = fs::write(&history_file_path, json);
        }
    }

    pub(super) fn add_to_history(&mut self, query: &str, row_count: usize) {
        let query = query.trim().to_string();

        // Don't add empty queries
        if query.is_empty() {
            return;
        }

        // Check if this exact query already exists in recent history (last 10 entries)
        let recent_limit = 10.min(self.query_history.len());
        let recent_queries = &self.query_history[self.query_history.len().saturating_sub(recent_limit)..];

        if !recent_queries.iter().any(|entry| entry.query == query) {
            // Calculate execution time if we have a start time
            let execution_time_ms = self.query_start_time.map(|start_time| {
                let elapsed = start_time.elapsed();
                let millis = elapsed.as_millis() as u64;
                // Show at least 1ms for very fast queries (but not 0)
                if elapsed.as_micros() > 0 && millis == 0 {
                    1
                } else {
                    millis
                }
            });
            
            let entry = QueryHistoryEntry {
                query,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    .as_secs(),
                row_count: Some(row_count),
                execution_time_ms,
                error: None,
            };
            self.query_history.push(entry);

            // Keep only last 100 entries to prevent unlimited growth
            if self.query_history.len() > 100 {
                self.query_history.drain(0..self.query_history.len() - 100);
            }

            self.save_query_history();
        }
    }

    // Result data helpers
    pub(super) fn column_count(&self) -> usize {
        self.selected_headers.len()
    }

    pub(super) fn get_current_row(&self) -> Option<&Vec<String>> {
        let index = if self.is_searching_results && !self.filtered_results.is_empty() {
            self.filtered_results[self.filtered_results_index].0
        } else {
            self.selected_row_index
        };
        self.query_results.get(index)
    }

    pub(super) fn update_horizontal_scroll_for_cell(&mut self) {
        // Auto-scroll horizontally to keep selected cell visible
        let cell_page = self.selected_cell_index / super::VISIBLE_COLUMNS;
        if cell_page != self.horizonal_scroll_offset {
            self.horizonal_scroll_offset = cell_page;
        }
    }


    /// Filter query results - populate with all results when called from QueryResult action
    pub(super) fn filter_results(&mut self) {
        self.filtered_results.clear();
        
        // When search is empty, show all results
        if self.results_search_query.is_empty() {
            // Empty search shows all results
            for idx in 0..self.query_results.len() {
                self.filtered_results.push((idx, vec![]));
            }
            self.filtered_results_index = 0;
            return;
        }
        
        // Simple substring search for now
        let query_lower = self.results_search_query.to_lowercase();
        
        for (idx, row) in self.query_results.iter().enumerate() {
            // Check if any column contains the search query
            if row.iter().any(|cell| cell.to_lowercase().contains(&query_lower)) {
                self.filtered_results.push((idx, row.clone()));
            }
        }
        
        // Reset filtered results index and update selected row
        self.filtered_results_index = 0;
        if !self.filtered_results.is_empty() {
            self.selected_row_index = self.filtered_results[0].0;
        }
    }




    // UI helpers
    pub(super) fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

    // Autocomplete functionality
    /// Manually triggers autocomplete at the current cursor position
    pub(super) fn trigger_autocomplete(&mut self) -> Option<Action> {
        // Get text up to cursor position for context analysis
        let text_up_to_cursor = self.editor_backend.get_text_up_to_cursor();
        let full_text = self.editor_backend.get_text();
        let cursor_pos = text_up_to_cursor.len();

        // Use the SQL parser to analyze context
        let (context, current_word) = SqlParser::analyze_context(&text_up_to_cursor, cursor_pos);

        // Always show suggestions when manually triggered, even for empty current word
        self.autocomplete_state.activate(cursor_pos, current_word.clone());
        
        // Get cursor position for LSP
        let (cursor_line, cursor_col) = self.editor_backend.get_cursor_position();
        
        match self.autocomplete_engine.backend_mut() {
            AutocompleteBackend::Builtin(provider) => {
                // Builtin provider can work synchronously
                let new_suggestions = provider.get_suggestions(context, &current_word);
                self.autocomplete_state.suggestions = new_suggestions;
                None
            }
            AutocompleteBackend::Lsp(_) => {
                // LSP requires async, send action to handle in main loop
                eprintln!("Requesting LSP autocomplete...");
                Some(Action::RequestAutocomplete {
                    text: full_text,
                    cursor_line,
                    cursor_col,
                    context: format!("{:?}", context), // Simple serialization
                })
            }
            AutocompleteBackend::Hybrid { builtin, .. } => {
                // In hybrid mode, try builtin first, then request LSP
                let builtin_suggestions = builtin.get_suggestions(context.clone(), &current_word);
                self.autocomplete_state.suggestions = builtin_suggestions;
                
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
    pub(super) fn apply_autocomplete_suggestion(&mut self, suggestion: SuggestionItem) {
        // Delete the current partial word before cursor
        if self.autocomplete_state.is_active {
            let text_up_to_cursor = self.editor_backend.get_text_up_to_cursor();
            let current_pos = text_up_to_cursor.len();
            
            // Find the start of the current word
            let word_start = text_up_to_cursor.rfind(|c: char| c.is_whitespace() || "(),;".contains(c))
                .map(|pos| pos + 1)
                .unwrap_or(0);
            
            // Delete from word start to current position
            for _ in word_start..current_pos {
                self.editor_backend.delete_char_before_cursor();
            }
        }

        // Insert the suggestion at the cursor position
        self.editor_backend.insert_text_at_cursor(&suggestion.text);
    }

    /// Generate schema information for a table
    pub(super) fn generate_table_schema(&self, table: &DbTable) -> String {
        let mut schema_info = format!("═══ TABLE SCHEMA ═══\n\n");
        schema_info.push_str(&format!("Table: {}.{}\n", table.schema, table.name));
        schema_info.push_str(&format!("{}\n\n", "─".repeat(50)));
        
        // Get columns from cache
        let columns = self.table_columns_cache.get(&table.name)
            .map(|c| c.as_slice())
            .unwrap_or(&[]);
        
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
            schema_info.push_str("Press Enter to load column details first.\n");
        }
        
        schema_info
    }

    /// Export query results to CSV file
    pub(super) fn export_results_to_csv(&mut self) -> Result<()> {
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


// Helper functions for creating table cells
pub(super) fn create_header_cell(header: &str, _height: u16) -> Cell<'static> {
    Cell::from(
        header
            .chars()
            .take(20)
            .collect::<String>()
            .to_owned()
    )
    .style(theme::header())
}