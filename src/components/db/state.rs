use std::time::Instant;

use color_eyre::eyre::Result;

use super::{Db, SelectionMode, VISIBLE_COLUMNS};
use crate::{action::Action, autocomplete_engine::AutocompleteEngine, components::ComponentKind, config::Config};

impl Db {
  pub(super) fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config.clone();

    // Initialize autocomplete engine based on configuration
    // For now, always use builtin since LSP requires async setup
    self.autocomplete_engine = AutocompleteEngine::new_builtin();

    // Initialize editor backend
    self.editor_backend = super::EditorBackend::new_from_config(&config.editor.backend);

    Ok(())
  }

  pub(super) fn update(&mut self, action: Action) -> Result<Option<Action>> {
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
              } else {
                // No filtering - navigate normally
                if self.selected_row_index < self.query_results.len() - 1 {
                  self.selected_row_index += 1;
                } else {
                  self.selected_row_index = 0; // Wrap to top
                }
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
              // Row mode no longer used - behave like Table mode
              if self.is_searching_results && !self.filtered_results.is_empty() {
                if self.filtered_results_index < self.filtered_results.len() - 1 {
                  self.filtered_results_index += 1;
                  self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
                }
              } else if self.selected_row_index < self.query_results.len() - 1 {
                self.selected_row_index += 1;
              }
            },
            SelectionMode::Preview => {
              // In preview mode, move down through the fields
              if self.preview_selected_index < self.selected_headers.len().saturating_sub(1) {
                self.preview_selected_index += 1;
                // Update scroll if needed
                let visible_rows = 20; // Estimate
                if self.preview_selected_index >= self.preview_scroll_offset as usize + visible_rows {
                  self.preview_scroll_offset = (self.preview_selected_index + 1).saturating_sub(visible_rows) as u16;
                }
              }
            },
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
              } else {
                // No filtering - navigate normally
                if self.selected_row_index > 0 {
                  self.selected_row_index -= 1;
                } else {
                  self.selected_row_index = self.query_results.len() - 1; // Wrap to bottom
                }
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
              // Row mode no longer used - behave like Table mode
              if self.is_searching_results && !self.filtered_results.is_empty() {
                if self.filtered_results_index > 0 {
                  self.filtered_results_index -= 1;
                  self.selected_row_index = self.filtered_results[self.filtered_results_index].0;
                }
              } else if self.selected_row_index > 0 {
                self.selected_row_index -= 1;
              }
            },
            SelectionMode::Preview => {
              // In preview mode, move up through the fields
              if self.preview_selected_index > 0 {
                self.preview_selected_index -= 1;
                // Update scroll if needed
                if self.preview_selected_index < self.preview_scroll_offset as usize {
                  self.preview_scroll_offset = self.preview_selected_index as u16;
                }
              }
            },
          }
        }
      },
      Action::ScrollTableLeft => {
        if self.selected_component == ComponentKind::Results {
          match self.selection_mode {
            SelectionMode::Cell => {
              // In cell mode, move cell selection left
              if self.selected_cell_index > 0 {
                self.selected_cell_index -= 1;
                self.update_horizontal_scroll_for_cell();
              }
            },
            _ => {
              // Normal horizontal scrolling
              if self.horizonal_scroll_offset > 0 {
                self.horizonal_scroll_offset -= 1;
              }
            },
          }
        }
      },
      Action::ScrollTableRight => {
        if self.selected_component == ComponentKind::Results {
          match self.selection_mode {
            SelectionMode::Cell => {
              // In cell mode, move cell selection right
              if let Some(row) = self.get_current_row() {
                if self.selected_cell_index < row.len() - 1 {
                  self.selected_cell_index += 1;
                  self.update_horizontal_scroll_for_cell();
                }
              }
            },
            _ => {
              // Normal horizontal scrolling
              if self.column_count() > 0
                && self.horizonal_scroll_offset < self.column_count().saturating_sub(VISIBLE_COLUMNS)
              {
                self.horizonal_scroll_offset += 1;
              }
            },
          }
        }
      },
      Action::LoadSelectedTable => {
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          let query = format!("SELECT * FROM {}", selected_table.name);
          self.editor_backend.set_text(&query);
          // Set query start time when loading table
          self.last_executed_query = Some(query.clone());
          self.is_query_running = true;
          self.query_start_time = Some(Instant::now());
          self.error_message = None;
          return Ok(Some(Action::HandleQuery(query)));
        } else {
          return Ok(None);
        }
      },
      Action::QueryExecutionTime(elapsed_ms) => {
        // Store the actual database execution time
        self.last_query_execution_time = Some(elapsed_ms);
      },
      Action::QueryResult(headers, results) => {
        // Use the database-reported execution time if available, otherwise calculate from UI timing
        let elapsed_time = self.last_query_execution_time.or_else(|| {
          self.query_start_time.map(|start_time| {
            let elapsed = start_time.elapsed();
            let millis = elapsed.as_millis() as u64;
            // Show at least 1ms for very fast queries
            if elapsed.as_micros() > 0 && millis == 0 {
              1
            } else {
              millis
            }
          })
        });
        
        // Clear the last execution time for next query
        self.last_query_execution_time = None;
        
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
        self.filter_results();

        // Check if this was an EXPLAIN query
        if let Some(ref query) = self.last_executed_query {
          let query_upper = query.trim().to_uppercase();
          self.is_explain_query = query_upper.starts_with("EXPLAIN");
          // Auto-enable EXPLAIN view for EXPLAIN queries
          if self.is_explain_query {
            self.is_explain_view = true;
          }
        } else {
          self.is_explain_query = false;
        }

        // Add successful query to history with the captured elapsed time
        if let Some(ref query) = self.last_executed_query {
          let query_clone = query.clone();
          let result_count = self.query_results.len();
          
          // Add to history with the captured elapsed time
          self.add_to_history_with_time(&query_clone, result_count, elapsed_time);
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
        // Execute query text from editor backend
        let query = if let Some(selected_text) = self.editor_backend.get_selected_text() {
          selected_text.trim().to_string()
        } else {
          self.editor_backend.get_text()
        };

        // Auto-format the query if enabled
        if self.editor_backend.is_auto_format_enabled() {
          if let Ok(_) = self.editor_backend.format_query(false) {
            // Query has been formatted in the editor
          }
        }

        // Store the query for history tracking
        self.last_executed_query = Some(query.clone());
        self.is_query_running = true;
        self.query_start_time = Some(Instant::now());
        self.error_message = None;
        return Ok(Some(Action::HandleQuery(query)));
      },
      Action::ExplainQuery => {
        // Get the query and prefix with EXPLAIN
        let query = if let Some(selected_text) = self.editor_backend.get_selected_text() {
          selected_text.trim().to_string()
        } else {
          self.editor_backend.get_text()
        };

        // Check if query already starts with EXPLAIN
        let trimmed = query.trim().to_uppercase();
        let explain_query = if trimmed.starts_with("EXPLAIN") { query } else { format!("EXPLAIN {}", query) };

        // Update editor with EXPLAIN query
        self.editor_backend.set_text(&explain_query);

        // Execute the EXPLAIN query
        self.last_executed_query = Some(explain_query.clone());
        self.is_query_running = true;
        self.query_start_time = Some(Instant::now());
        self.error_message = None;
        return Ok(Some(Action::HandleQuery(explain_query)));
      },
      Action::ExplainAnalyzeQuery => {
        // Get the query and prefix with EXPLAIN ANALYZE
        let query = if let Some(selected_text) = self.editor_backend.get_selected_text() {
          selected_text.trim().to_string()
        } else {
          self.editor_backend.get_text()
        };

        // Check if query already starts with EXPLAIN
        let trimmed_upper = query.trim().to_uppercase();
        let trimmed = query.trim();

        let explain_query = if trimmed_upper.starts_with("EXPLAIN") {
          // Parse existing EXPLAIN
          let mut remaining = &trimmed[7..]; // Skip "EXPLAIN"
          remaining = remaining.trim();

          if remaining.starts_with('(') {
            // Has options, check if ANALYZE is already there
            if let Some(close_paren) = remaining.find(')') {
              let options = &remaining[1..close_paren];
              if options.to_uppercase().contains("ANALYZE") {
                // Already has ANALYZE
                query
              } else {
                // Add ANALYZE to options
                let query_part = &remaining[close_paren + 1..].trim();
                format!("EXPLAIN ({}, ANALYZE) {}", options, query_part)
              }
            } else {
              // Malformed, use standard format
              format!("EXPLAIN (ANALYZE) {}", remaining)
            }
          } else {
            // No options, add them
            format!("EXPLAIN (ANALYZE) {}", remaining)
          }
        } else {
          // No EXPLAIN at all
          format!("EXPLAIN (ANALYZE) {}", trimmed)
        };

        // Update editor with EXPLAIN ANALYZE query
        self.editor_backend.set_text(&explain_query);

        // Execute the EXPLAIN ANALYZE query
        self.last_executed_query = Some(explain_query.clone());
        self.is_query_running = true;
        self.query_start_time = Some(Instant::now());
        self.error_message = None;
        return Ok(Some(Action::HandleQuery(explain_query)));
      },
      Action::QueryStarted => {
        self.is_query_running = true;
        self.query_start_time = Some(Instant::now());
        self.error_message = None;
      },
      Action::QueryCompleted => {
        self.is_query_running = false;
        self.query_start_time = None;
      },
      Action::RowDetails => {
        // Show preview popup (same as pressing 'p' or space)
        if self.selected_component == ComponentKind::Results {
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
        }
      },
      Action::Error(e) => {
        self.error_message = Some(e);
        self.is_query_running = false;
        self.query_start_time = None;

        // Keep the query for potential retry/toggle
        // We don't add failed queries to history for now
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

        // Update autocomplete state with results
        if self.autocomplete_state.is_active {
          use crate::autocomplete::{SuggestionItem, SuggestionKind};

          let suggestions: Vec<SuggestionItem> = results
            .into_iter()
            .map(|(text, kind_str)| {
              let kind = match kind_str.as_str() {
                "table" => SuggestionKind::Table,
                "column" => SuggestionKind::Column,
                _ => SuggestionKind::Keyword,
              };
              SuggestionItem { text, kind, score: 100, table_context: None }
            })
            .collect();

          self.autocomplete_state.suggestions = suggestions;
          self.autocomplete_state.selected_index = 0;
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
      Action::LoadTable(table_name) => {
        // This action should be handled by the App component
        return Ok(Some(Action::LoadTable(table_name)));
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
          if let Err(e) = self.editor_backend.format_query(false) {
            self.error_message = Some(e.to_string());
          } else {
            self.error_message = Some("Query formatted".to_string());
          }
        }
      },
      Action::FormatSelection => {
        if self.selected_component == ComponentKind::Query {
          if let Err(e) = self.editor_backend.format_query(true) {
            self.error_message = Some(e.to_string());
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
      Action::ToggleExplainView => {
        // Toggle between EXPLAIN and regular query execution
        if let Some(ref last_query) = self.last_executed_query {
          let trimmed = last_query.trim();
          let query_upper = trimmed.to_uppercase();

          let new_query = if query_upper.starts_with("EXPLAIN") {
            // Remove EXPLAIN and any options like (ANALYZE, BUFFERS)
            let mut remaining = &trimmed[7..]; // Skip "EXPLAIN"
            remaining = remaining.trim();

            // Skip parenthesized options if present
            if remaining.starts_with('(') {
              if let Some(close_paren) = remaining.find(')') {
                remaining = &remaining[close_paren + 1..];
                remaining = remaining.trim();
              }
            }
            remaining.to_string()
          } else {
            // Add EXPLAIN prefix
            format!("EXPLAIN {}", trimmed)
          };

          // Execute the toggled query
          self.last_executed_query = Some(new_query.clone());
          self.is_query_running = true;
          self.query_start_time = Some(Instant::now());
          self.error_message = None;
          return Ok(Some(Action::HandleQuery(new_query)));
        } else {
          self.error_message = Some("No query to toggle EXPLAIN for".to_string());
        }
      },
      Action::ToggleExplainAnalyze => {
        // Toggle between EXPLAIN ANALYZE and regular query execution
        if let Some(ref last_query) = self.last_executed_query {
          let trimmed = last_query.trim();
          let query_upper = trimmed.to_uppercase();

          let new_query = if query_upper.starts_with("EXPLAIN") {
            // Parse existing EXPLAIN to modify it
            let mut remaining = &trimmed[7..]; // Skip "EXPLAIN"
            remaining = remaining.trim();

            // Check if it has parenthesized options
            if remaining.starts_with('(') {
              if let Some(close_paren) = remaining.find(')') {
                let options = &remaining[1..close_paren];
                let query_part = &remaining[close_paren + 1..].trim();

                // Check if ANALYZE is already in options
                if options.to_uppercase().contains("ANALYZE") {
                  // Remove entire EXPLAIN statement
                  query_part.to_string()
                } else {
                  // Add ANALYZE to existing options
                  format!("EXPLAIN ({}, ANALYZE) {}", options, query_part)
                }
              } else {
                // Malformed, just add EXPLAIN ANALYZE
                format!("EXPLAIN ANALYZE {}", remaining)
              }
            } else {
              // No parentheses, add ANALYZE
              format!("EXPLAIN (ANALYZE) {}", remaining)
            }
          } else {
            // Add EXPLAIN ANALYZE prefix
            format!("EXPLAIN (ANALYZE) {}", trimmed)
          };

          // Execute the toggled query
          self.last_executed_query = Some(new_query.clone());
          self.is_query_running = true;
          self.query_start_time = Some(Instant::now());
          self.error_message = None;
          return Ok(Some(Action::HandleQuery(new_query)));
        } else {
          self.error_message = Some("No query to toggle EXPLAIN ANALYZE for".to_string());
        }
      },
      Action::CopyExplainResults => {
        // Copy EXPLAIN results to clipboard
        if self.is_explain_query && !self.query_results.is_empty() {
          use clipboard::{ClipboardContext, ClipboardProvider};

          // Build the EXPLAIN output as text
          let mut output = String::new();

          // Check if it's a single-column EXPLAIN (PostgreSQL text format)
          if self.selected_headers.len() == 1 && self.selected_headers[0].to_lowercase().contains("query plan") {
            // PostgreSQL text format - just concatenate rows
            for row in &self.query_results {
              if let Some(cell) = row.first() {
                output.push_str(cell);
                output.push('\n');
              }
            }
          } else {
            // Multi-column format - format as table
            // Add headers
            output.push_str(&self.selected_headers.join(" | "));
            output.push('\n');
            output.push_str(&"-".repeat(80));
            output.push('\n');

            // Add rows
            for row in &self.query_results {
              output.push_str(&row.join(" | "));
              output.push('\n');
            }
          }

          // Copy to clipboard
          match ClipboardContext::new() {
            Ok(mut ctx) => {
              match ctx.set_contents(output) {
                Ok(_) => self.export_status = Some(("EXPLAIN results copied to clipboard".to_string(), Instant::now())),
                Err(e) => self.error_message = Some(format!("Failed to copy: {}", e)),
              }
            },
            Err(e) => self.error_message = Some(format!("Failed to access clipboard: {}", e)),
          }
        } else {
          self.error_message = Some("No EXPLAIN results to copy".to_string());
        }
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
}
