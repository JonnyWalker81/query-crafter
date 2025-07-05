use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use clipboard::ClipboardProvider;

use super::{Db, SelectionMode, VISIBLE_COLUMNS};
use crate::{
    action::Action,
    components::ComponentKind,
    editor_common::Mode,
};

impl Db {
    pub(super) fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(crate::tui::Event::Key(key)) = event {
            self.handle_key_events(key)
        } else {
            Ok(None)
        }
    }

    pub(super) fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
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

        // Handle preview mode globally (before component-specific handling)
        if self.selection_mode == SelectionMode::Preview {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    // Move selection up
                    if self.preview_selected_index > 0 {
                        self.preview_selected_index -= 1;
                        // Update scroll if needed
                        if self.preview_selected_index < self.preview_scroll_offset as usize {
                            self.preview_scroll_offset = self.preview_selected_index as u16;
                        }
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::Down | KeyCode::Char('j') => {
                    // Move selection down
                    if self.preview_selected_index < self.selected_headers.len().saturating_sub(1) {
                        self.preview_selected_index += 1;
                        // Update scroll if needed
                        let visible_rows = 20; // Estimate, should be calculated from actual height
                        if self.preview_selected_index >= self.preview_scroll_offset as usize + visible_rows {
                            self.preview_scroll_offset = (self.preview_selected_index + 1).saturating_sub(visible_rows) as u16;
                        }
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::PageUp => {
                    // Page up
                    self.preview_selected_index = self.preview_selected_index.saturating_sub(10);
                    self.preview_scroll_offset = self.preview_selected_index as u16;
                    return Ok(Some(Action::Render));
                },
                KeyCode::PageDown => {
                    // Page down
                    let max_index = self.selected_headers.len().saturating_sub(1);
                    self.preview_selected_index = (self.preview_selected_index + 10).min(max_index);
                    
                    // Update scroll to show selected item
                    let visible_rows = 20; // Estimate
                    if self.preview_selected_index >= self.preview_scroll_offset as usize + visible_rows {
                        self.preview_scroll_offset = (self.preview_selected_index + 1).saturating_sub(visible_rows) as u16;
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::Home => {
                    // Go to first row
                    self.preview_selected_index = 0;
                    self.preview_scroll_offset = 0;
                    return Ok(Some(Action::Render));
                },
                KeyCode::End => {
                    // Go to last row
                    self.preview_selected_index = self.selected_headers.len().saturating_sub(1);
                    self.preview_scroll_offset = self.preview_selected_index.saturating_sub(20) as u16;
                    return Ok(Some(Action::Render));
                },
                KeyCode::Esc | KeyCode::Char('p') => {
                    // Exit preview mode
                    self.selection_mode = SelectionMode::Table;
                    self.preview_scroll_offset = 0;
                    self.preview_selected_index = 0;
                    return Ok(Some(Action::Render));
                },
                KeyCode::Char('y') => {
                    // Copy the VALUE (not column name) of the selected row in preview
                    self.copy_preview_cell_value();
                    return Ok(Some(Action::Render));
                },
                KeyCode::Char('Y') => {
                    // Copy entire row as JSON
                    self.copy_row_as_json();
                    return Ok(Some(Action::Render));
                },
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Copy selected value (same as 'y')
                    self.copy_preview_cell_value();
                    return Ok(Some(Action::Render));
                },
                _ => return Ok(None),
            }
        }

        // Handle keys when table info popup is open
        if self.show_table_columns || self.show_table_schema {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    let max_scroll = if self.show_table_columns {
                        self.selected_table_columns.len().saturating_sub(17) // popup_height - 3
                    } else {
                        self.selected_table_schema.lines().count().saturating_sub(17)
                    };
                    if self.table_info_scroll < max_scroll {
                        self.table_info_scroll = self.table_info_scroll.saturating_add(1);
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::Up | KeyCode::Char('k') => {
                    self.table_info_scroll = self.table_info_scroll.saturating_sub(1);
                    return Ok(Some(Action::Render));
                },
                KeyCode::PageDown => {
                    let max_scroll = if self.show_table_columns {
                        self.selected_table_columns.len().saturating_sub(17)
                    } else {
                        self.selected_table_schema.lines().count().saturating_sub(17)
                    };
                    self.table_info_scroll = (self.table_info_scroll + 10).min(max_scroll);
                    return Ok(Some(Action::Render));
                },
                KeyCode::PageUp => {
                    self.table_info_scroll = self.table_info_scroll.saturating_sub(10);
                    return Ok(Some(Action::Render));
                },
                KeyCode::Char('c') => {
                    // Allow copying column info to clipboard
                    if self.show_table_columns {
                        let columns_text = self.selected_table_columns
                            .iter()
                            .map(|col| format!("{}: {} {}", 
                                col.name, 
                                col.data_type,
                                if col.is_nullable { "(nullable)" } else { "(not null)" }
                            ))
                            .collect::<Vec<_>>()
                            .join("\n");
                        
                        if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                            let _ = ctx.set_contents(columns_text).ok();
                        }
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::Char('i') => {
                    // Toggle to schema view
                    if self.show_table_columns {
                        self.show_table_columns = false;
                        self.show_table_schema = true;
                        self.table_info_scroll = 0;
                        if self.selected_table_schema.is_empty() {
                            if let Some(selected_table) = self.tables.get(self.selected_table_index) {
                                return Ok(Some(Action::LoadTable(selected_table.name.clone())));
                            }
                        }
                    }
                    return Ok(Some(Action::Render));
                },
                KeyCode::Enter => {
                    // Toggle to columns view
                    if self.show_table_schema {
                        self.show_table_schema = false;
                        self.show_table_columns = true;
                        self.table_info_scroll = 0;
                    }
                    return Ok(Some(Action::Render));
                },
                _ => {
                    // Block all other keys when popup is open
                    return Ok(Some(Action::Render));
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

        if !is_editing && !self.is_searching_tables && !self.is_searching_results {
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
            ComponentKind::Home => self.handle_table_keys(key),
            ComponentKind::Query => self.handle_query_component_keys(key),
            ComponentKind::Results => self.handle_results_keys(key),
        }
    }

    fn handle_table_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Handle search mode
        if self.is_searching_tables {
            match key.code {
                KeyCode::Esc => {
                    // Clear search and exit search mode
                    self.table_search_query.clear();
                    self.is_searching_tables = false;
                    return Ok(Some(Action::Render));
                },
                KeyCode::Enter => {
                    self.is_searching_tables = false;
                    return Ok(Some(Action::Render));
                },
                KeyCode::Backspace => {
                    self.table_search_query.pop();
                    return Ok(Some(Action::Render));
                },
                KeyCode::Char(c) => {
                    self.table_search_query.push(c);
                    return Ok(Some(Action::Render));
                },
                _ => return Ok(None),
            }
        }

        // Normal table navigation
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_tables_up();
                Ok(None)
            },
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_tables_down();
                Ok(None)
            },
            KeyCode::Char('/') => {
                self.is_searching_tables = true;
                self.table_search_query.clear();
                Ok(None)
            },
            KeyCode::Enter => {
                // Show table columns
                self.show_table_columns = true;
                self.table_info_scroll = 0;
                if let Some(selected_table) = self.tables.get(self.selected_table_index) {
                    Ok(Some(Action::LoadTable(selected_table.name.clone())))
                } else {
                    Ok(None)
                }
            },
            KeyCode::Char('i') => {
                // Show table schema
                self.show_table_schema = true;
                self.table_info_scroll = 0;
                if let Some(selected_table) = self.tables.get(self.selected_table_index) {
                    Ok(Some(Action::LoadTable(selected_table.name.clone())))
                } else {
                    Ok(None)
                }
            },
            KeyCode::Char('s') => {
                // Select table for query
                if let Some(selected_table) = self.tables.get(self.selected_table_index) {
                    let query = format!("SELECT * FROM {} LIMIT 100", selected_table.name);
                    self.editor_backend.set_text(&query);
                    self.selected_tab = 0; // Ensure we're on Query tab
                    self.selected_component = ComponentKind::Query;
                }
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    fn handle_query_component_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Handle tab switching first (works in all modes except insert mode)
        if self.editor_backend.mode() != Mode::Insert {
            match key.code {
                KeyCode::Char('t') => {
                    // Toggle between Query (0) and History (1) tabs
                    self.selected_tab = if self.selected_tab == 0 { 1 } else { 0 };
                    return Ok(Some(Action::Render));
                },
                _ => {},
            }
        }

        // Handle history tab navigation
        if self.selected_tab == 1 {
            return self.handle_history_tab_keys(key);
        }

        // Otherwise, handle query tab
        self.handle_query_tab_keys(key)
    }

    fn handle_query_tab_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Handle autocomplete navigation
        if self.autocomplete_state.is_active {
            match key.code {
                KeyCode::Tab | KeyCode::Down => {
                    self.autocomplete_state.select_next();
                    return Ok(Some(Action::Render));
                },
                KeyCode::BackTab | KeyCode::Up => {
                    self.autocomplete_state.select_previous();
                    return Ok(Some(Action::Render));
                },
                KeyCode::Enter => {
                    // Apply selected suggestion
                    if self.autocomplete_state.is_active {
                        if self.autocomplete_state.selected_index < self.autocomplete_state.suggestions.len() {
                            let suggestion = self.autocomplete_state.suggestions[self.autocomplete_state.selected_index].clone();
                            self.apply_autocomplete_suggestion(suggestion);
                            self.autocomplete_state.deactivate();
                            return Ok(Some(Action::Render));
                        }
                    }
                },
                KeyCode::Esc => {
                    self.autocomplete_state.deactivate();
                    return Ok(Some(Action::Render));
                },
                _ => {
                    // For other keys, deactivate autocomplete and let editor handle it
                    if self.editor_backend.mode() == Mode::Insert {
                        self.autocomplete_state.deactivate();
                    }
                },
            }
        }

        // Handle manual autocomplete trigger (Ctrl+Space)
        if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if self.editor_backend.mode() == Mode::Insert {
                return Ok(Some(Action::TriggerAutocomplete));
            }
        }

        // Delegate to editor backend
        if let Some(action) = self.editor_backend.handle_key_event(key)? {
            // Update LSP document if needed
            if self.editor_backend.mode() == Mode::Insert {
                match self.autocomplete_engine.backend_name() {
                    "lsp" | "hybrid" => {
                        let text = self.editor_backend.get_text();
                        if let Some(tx) = &self.command_tx {
                            let _ = tx.send(Action::UpdateAutocompleteDocument(text));
                        }
                    }
                    _ => {}
                }
            }
            return Ok(Some(action));
        }

        // Handle query execution
        if key.code == KeyCode::Enter && self.editor_backend.mode() == Mode::Normal {
            let query_text = self.editor_backend.get_text();
            let trimmed_query = query_text.trim();
            if !trimmed_query.is_empty() {
                return Ok(Some(Action::HandleQuery(trimmed_query.to_string())));
            }
        }

        Ok(None)
    }

    fn handle_history_tab_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.query_history.is_empty() && self.selected_history_index > 0 {
                    self.selected_history_index -= 1;
                }
                Ok(None)
            },
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.query_history.is_empty() && self.selected_history_index < self.query_history.len() - 1 {
                    self.selected_history_index += 1;
                }
                Ok(None)
            },
            KeyCode::Enter => {
                // Load selected query into editor and execute it
                if self.selected_history_index < self.query_history.len() {
                    let entry = &self.query_history[self.query_history.len() - 1 - self.selected_history_index];
                    self.editor_backend.set_text(&entry.query);
                    
                    // Auto-format the query if enabled
                    if self.editor_backend.is_auto_format_enabled() {
                        if let Ok(_) = self.editor_backend.format_query(false) {
                            // Query has been formatted in the editor
                        }
                    }
                    
                    self.selected_tab = 0; // Switch to Query tab
                    self.selected_component = ComponentKind::Query; // Switch to Query component
                    // Store the query for history tracking and set start time
                    self.last_executed_query = Some(entry.query.clone());
                    self.is_query_running = true;
                    self.query_start_time = Some(std::time::Instant::now());
                    self.error_message = None;
                    // Execute the query
                    return Ok(Some(Action::HandleQuery(entry.query.clone())));
                }
                Ok(None)
            },
            KeyCode::Char('c') => {
                // Copy query to clipboard
                if self.selected_history_index < self.query_history.len() {
                    let entry = &self.query_history[self.query_history.len() - 1 - self.selected_history_index];
                    if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                        let _ = ctx.set_contents(entry.query.clone()).ok();
                    }
                }
                Ok(None)
            },
            KeyCode::Char('d') => {
                // Delete history entry
                if self.selected_history_index < self.query_history.len() {
                    let actual_index = self.query_history.len() - 1 - self.selected_history_index;
                    self.query_history.remove(actual_index);
                    self.save_query_history();
                    if self.selected_history_index >= self.query_history.len() && self.selected_history_index > 0 {
                        self.selected_history_index = self.query_history.len() - 1;
                    }
                }
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    fn handle_results_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Handle search mode
        if self.is_searching_results {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching_results = false;
                    self.results_search_query.clear();
                    self.filtered_results.clear();
                    Ok(None)
                },
                KeyCode::Enter => {
                    self.is_searching_results = false;
                    Ok(None)
                },
                KeyCode::Backspace => {
                    self.results_search_query.pop();
                    self.filter_results();
                    Ok(None)
                },
                KeyCode::Char(c) => {
                    self.results_search_query.push(c);
                    self.filter_results();
                    Ok(None)
                },
                _ => Ok(None),
            }
        } else {
            self.handle_results_navigation(key)
        }
    }

    fn handle_results_navigation(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Preview mode is now handled globally in handle_key_events
        
        // Handle cell selection mode navigation
        if self.selection_mode == SelectionMode::Cell {
            // Handle all keys in cell selection mode to prevent them from triggering other actions
            match key.code {
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.selected_cell_index > 0 {
                        self.selected_cell_index -= 1;
                        self.update_horizontal_scroll_for_cell();
                    }
                    return Ok(Some(Action::Render)); // Force render but consume the key
                },
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(row) = self.get_current_row() {
                        if self.selected_cell_index < row.len() - 1 {
                            self.selected_cell_index += 1;
                            self.update_horizontal_scroll_for_cell();
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
                KeyCode::Char('y') | KeyCode::Char('c') => {
                    // Copy in cell mode
                    self.copy_cell_value();
                    return Ok(Some(Action::Render));
                },
                _ => {
                    // Consume all other keys in cell selection mode
                    return Ok(Some(Action::Render));
                },
            }
        }

        // Handle other modes and navigation
        self.handle_normal_results_keys(key)
    }


    fn handle_normal_results_keys(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        // Normal results mode key handling
        match key.code {
            KeyCode::Char('/') => {
                // Enter search mode
                self.is_searching_results = true;
                self.results_search_query.clear();
                self.filter_results(); // Initialize with all results
                return Ok(None);
            },
            KeyCode::Char('y') => {
                // Copy current cell or row based on selection mode
                match self.selection_mode {
                    SelectionMode::Cell => self.copy_cell_value(),
                    _ => self.copy_row_values(),
                }
                Ok(None)
            },
            KeyCode::Char('Y') => {
                // Copy entire row as TSV
                self.copy_row_values();
                Ok(None)
            },
            KeyCode::Char('r') => {
                // Re-run last query
                Ok(Some(Action::ExecuteQuery))
            },
            KeyCode::Char(' ') | KeyCode::Enter => {
                // Show row details popup (same as 'p')
                // Only enter preview mode if we have results
                if !self.query_results.is_empty() && !self.selected_headers.is_empty() {
                    self.selection_mode = match self.selection_mode {
                        SelectionMode::Preview => {
                            self.preview_scroll_offset = 0;
                            self.preview_selected_index = 0;
                            SelectionMode::Table
                        },
                        _ => {
                            self.preview_scroll_offset = 0;
                            self.preview_selected_index = 0;
                            SelectionMode::Preview
                        },
                    };
                    Ok(Some(Action::Render))
                } else {
                    Ok(None)
                }
            },
            KeyCode::Char('p') => {
                // Toggle preview popup
                // Only enter preview mode if we have results
                if !self.query_results.is_empty() && !self.selected_headers.is_empty() {
                    self.selection_mode = match self.selection_mode {
                        SelectionMode::Preview => {
                            self.preview_scroll_offset = 0;
                            self.preview_selected_index = 0;
                            SelectionMode::Table
                        },
                        _ => {
                            self.preview_scroll_offset = 0;
                            self.preview_selected_index = 0;
                            SelectionMode::Preview
                        },
                    };
                    Ok(Some(Action::Render))
                } else {
                    Ok(None)
                }
            },
            KeyCode::Char('v') => {
                // Enter cell selection mode
                self.selection_mode = SelectionMode::Cell;
                // Start at the first visible column based on current horizontal scroll
                self.selected_cell_index = self.horizonal_scroll_offset;
                Ok(None)
            },
            KeyCode::Char('e') => {
                Ok(Some(Action::ExportResultsToCsv))
            },
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_results_up();
                Ok(None)
            },
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_results_down();
                Ok(None)
            },
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selection_mode != SelectionMode::Cell {
                    if self.horizonal_scroll_offset > 0 {
                        self.horizonal_scroll_offset -= 1;
                    }
                }
                Ok(None)
            },
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selection_mode != SelectionMode::Cell {
                    let max_offset = self.selected_headers.len().saturating_sub(VISIBLE_COLUMNS) / VISIBLE_COLUMNS;
                    if self.horizonal_scroll_offset < max_offset {
                        self.horizonal_scroll_offset += 1;
                    }
                }
                Ok(None)
            },
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.navigate_results_up();
                }
                Ok(None)
            },
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.navigate_results_down();
                }
                Ok(None)
            },
            KeyCode::Home => {
                self.selected_row_index = 0;
                if self.is_searching_results && !self.filtered_results.is_empty() {
                    self.filtered_results_index = 0;
                }
                Ok(None)
            },
            KeyCode::End => {
                let max_index = if self.is_searching_results && !self.filtered_results.is_empty() {
                    self.filtered_results.len() - 1
                } else {
                    self.query_results.len().saturating_sub(1)
                };
                self.selected_row_index = max_index;
                if self.is_searching_results {
                    self.filtered_results_index = max_index;
                }
                Ok(None)
            },
            KeyCode::Esc => {
                // Exit any special selection mode
                if self.selection_mode != SelectionMode::Table {
                    self.selection_mode = SelectionMode::Table;
                    return Ok(Some(Action::Render));
                }
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    // Helper methods for navigation
    fn navigate_tables_up(&mut self) {
        if self.selected_table_index > 0 {
            self.selected_table_index -= 1;
        }
    }

    fn navigate_tables_down(&mut self) {
        let max_index = self.get_filtered_tables_count().saturating_sub(1);
        if self.selected_table_index < max_index {
            self.selected_table_index += 1;
        }
    }

    fn navigate_results_up(&mut self) {
        if self.is_searching_results && !self.filtered_results.is_empty() {
            if self.filtered_results_index > 0 {
                self.filtered_results_index -= 1;
                self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
            }
        } else if self.selected_row_index > 0 {
            self.selected_row_index -= 1;
        }
    }

    fn navigate_results_down(&mut self) {
        if self.is_searching_results && !self.filtered_results.is_empty() {
            if self.filtered_results_index < self.filtered_results.len() - 1 {
                self.filtered_results_index += 1;
                self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
            }
        } else if self.selected_row_index < self.query_results.len().saturating_sub(1) {
            self.selected_row_index += 1;
        }
    }




    fn get_filtered_tables_count(&self) -> usize {
        if self.table_search_query.is_empty() {
            self.tables.len()
        } else {
            self.tables
                .iter()
                .filter(|table| {
                    table.name.to_lowercase().contains(&self.table_search_query.to_lowercase())
                })
                .count()
        }
    }

    fn copy_cell_value(&mut self) {
        if let Some(row) = self.get_current_row() {
            if let Some(value) = row.get(self.selected_cell_index) {
                if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                    let _ = ctx.set_contents(value.clone()).ok();
                }
            }
        }
    }

    fn copy_row_values(&mut self) {
        if let Some(row) = self.get_current_row() {
            let content = row.join("\t");
            if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                let _ = ctx.set_contents(content).ok();
            }
        }
    }
    
    fn copy_preview_cell_value(&mut self) {
        if let Some(row) = self.get_current_row() {
            // In preview mode, preview_selected_index tracks which row in the preview table
            // Each row shows a column name and its value, so we copy the value
            if self.preview_selected_index < row.len() {
                let value = &row[self.preview_selected_index];
                if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                    let _ = ctx.set_contents(value.clone()).ok();
                }
            }
        }
    }
    
    fn copy_row_as_json(&mut self) {
        if let Some(row) = self.get_current_row() {
            use std::collections::HashMap;
            
            // Create a JSON object with column names as keys
            let mut json_map = HashMap::new();
            for (header, value) in self.selected_headers.iter().zip(row.iter()) {
                json_map.insert(header.as_str(), value.as_str());
            }
            
            // Convert to JSON string
            if let Ok(json_string) = serde_json::to_string_pretty(&json_map) {
                if let Ok(mut ctx) = clipboard::ClipboardContext::new() {
                    let _ = ctx.set_contents(json_string).ok();
                }
            }
        }
    }

}