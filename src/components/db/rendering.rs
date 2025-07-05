use std::rc::Rc;

use chrono::{Local, TimeZone};
use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    text::{Line, Span},
    widgets::*,
};

use super::{Db, SelectionMode, VISIBLE_COLUMNS, helpers::create_header_cell};
use crate::{
    autocomplete::SuggestionKind,
    components::{ComponentKind, Frame},
};
use query_crafter_theme as theme;

impl Db {
    pub(super) fn draw(&mut self, f: &mut Frame<'_>, _area: Rect) -> Result<()> {
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

        // Create table headers
        let header_cells = vec!["#", "Time", "Query", "Rows", "Duration"]
            .iter()
            .map(|h| Cell::from(*h).style(theme::header()))
            .collect::<Vec<_>>();
        let header = ratatui::widgets::Row::new(header_cells)
            .height(1)
            .style(theme::header())
            .bottom_margin(1);

        // Create table rows from history (most recent first)
        let rows: Vec<ratatui::widgets::Row> = self
            .query_history
            .iter()
            .rev() // Show most recent first
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == self.selected_history_index;
                
                // Number column
                let num = format!("{}", idx + 1);
                
                // Format timestamp in local time with relative display for recent times
                let time_str = Local
                    .timestamp_opt(entry.timestamp as i64, 0)
                    .single()
                    .map(|dt| {
                        let now = Local::now();
                        let duration = now.signed_duration_since(dt);
                        
                        if duration.num_seconds() < 60 {
                            "Just now".to_string()
                        } else if duration.num_minutes() < 60 {
                            format!("{} min ago", duration.num_minutes())
                        } else if duration.num_hours() < 24 {
                            format!("{} hours ago", duration.num_hours())
                        } else if duration.num_days() == 1 {
                            format!("Yesterday {}", dt.format("%H:%M"))
                        } else if duration.num_days() < 7 {
                            format!("{} days ago", duration.num_days())
                        } else {
                            dt.format("%Y-%m-%d %H:%M:%S").to_string()
                        }
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                // Truncate query
                let truncated_query = if entry.query.len() > 60 {
                    format!("{}...", &entry.query[..57])
                } else {
                    entry.query.clone()
                };

                // Row count
                let row_count = entry.row_count
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "-".to_string());

                // Duration
                let duration = entry.execution_time_ms
                    .map(|ms| format!("{}ms", ms))
                    .unwrap_or_else(|| "-".to_string());

                let cells = vec![
                    Cell::from(num),
                    Cell::from(time_str),
                    Cell::from(truncated_query),
                    Cell::from(row_count),
                    Cell::from(duration),
                ];

                let style = if is_selected {
                    theme::selection_active()
                } else if entry.error.is_some() {
                    theme::error()
                } else {
                    theme::bg_primary()
                };

                ratatui::widgets::Row::new(cells).style(style)
            })
            .collect();

        let table = Table::new(rows, [
            Constraint::Length(3),   // #
            Constraint::Length(20),  // Time (YYYY-MM-DD HH:MM:SS)
            Constraint::Min(40),     // Query (flexible, takes remaining space)
            Constraint::Length(7),   // Rows
            Constraint::Length(8),   // Duration
        ])
        .header(header)
        .block(history_block)
        .row_highlight_style(theme::selection_active())
        .highlight_symbol("→ ");

        let mut table_state = TableState::default();
        table_state.select(Some(self.selected_history_index));

        f.render_stateful_widget(table, area, &mut table_state);

        Ok(())
    }

    fn render_query_results(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
        match self.selection_mode {
            SelectionMode::Row => self.render_query_results_table(f, chunks), // Row mode no longer used
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

        self.render_query_results_table_inner(f, table_chunks)?;
        Ok(chunks)
    }

    fn render_query_results_table_inner(&mut self, f: &mut Frame<'_>, table_chunks: Rc<[Rect]>) -> Result<()> {
        // Check if we should render in EXPLAIN view mode
        if self.is_explain_view && self.is_explain_query {
            return self.render_explain_output(f, table_chunks);
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
                .filter_map(|(idx, _row)| self.query_results.get(*idx))
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
        let row_count = if !self.filtered_results.is_empty() {
            format!("{}/{}", self.filtered_results.len(), self.query_results.len())
        } else {
            self.query_results.len().to_string()
        };
        let col_count = self.selected_headers.len();
        
        let title = if self.is_query_running {
            let elapsed = self.query_start_time
                .map(|start| start.elapsed().as_secs_f32())
                .unwrap_or(0.0);
            format!("[3] Results - Loading... ({:.1}s)", elapsed)
        } else if self.is_searching_results {
            format!("[3] Results ({} rows, {} cols) - Search: {}_", row_count, col_count, self.results_search_query)
        } else if !self.results_search_query.is_empty() {
            format!("[3] Results ({} rows, {} cols) - Filter: {}", row_count, col_count, self.results_search_query)
        } else {
            let base_info = format!("[3] Results ({} rows, {} cols)", row_count, col_count);
            match self.selection_mode {
                SelectionMode::Row => base_info.clone(), // Row mode no longer used
                SelectionMode::Cell => format!("{} - Cell Selection - ESC to exit", base_info),
                SelectionMode::Preview => format!("{} - Preview Mode - ESC to exit", base_info),
                SelectionMode::Table => {
                    if self.is_explain_query {
                        format!("{} - [x] Table View [a] EXPLAIN ANALYZE [c] Copy EXPLAIN", base_info)
                    } else {
                        format!("{} - [Space/Enter] Preview [v] Cell Mode [x] EXPLAIN [a] EXPLAIN ANALYZE", base_info)
                    }
                },
            }
        };

        let table_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_results_focused {
                theme::border_focused()
            } else {
                theme::border_normal()
            })
            .title(title)
            .title_style(theme::title())
            .border_type(BorderType::Rounded);

        // Handle empty results
        if self.query_results.is_empty() {
            let empty_message = if self.is_query_running {
                "Executing query..."
            } else {
                "No results. Execute a query to see data."
            };
            let empty_paragraph = Paragraph::new(empty_message)
                .block(table_block)
                .style(theme::muted())
                .alignment(Alignment::Center);
            f.render_widget(empty_paragraph, table_chunks[0]);
            
            // Render loading overlay
            if self.is_query_running {
                self.render_loading_overlay(f, table_chunks[0]);
            }
            
            return Ok(());
        }

        // Determine column widths with even distribution and min width
        let available_width = table_chunks[0].width.saturating_sub(4); // Account for borders and padding
        let base_width = available_width / VISIBLE_COLUMNS as u16;
        let widths: Vec<Constraint> = (0..VISIBLE_COLUMNS)
            .map(|_| Constraint::Length(base_width.max(15)))
            .collect();

        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(theme::selection_active())
            .style(theme::bg_primary())
            .column_spacing(1);

        f.render_stateful_widget(table, table_chunks[0], &mut table_state);

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

        // Highlight the selected cell in the header
        let header_cells: Vec<_> = self
            .selected_headers
            .iter()
            .enumerate()
            .skip(skip_count)
            .take(VISIBLE_COLUMNS)
            .map(|(idx, h)| {
                let is_selected = idx == self.selected_cell_index;
                let cell = create_header_cell(h, HEADER_HEIGHT);
                if is_selected {
                    cell.style(theme::selection_active())
                } else {
                    cell
                }
            })
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
                    let is_current_row = row_idx == self.selected_row_index;
                    let cells = r.iter().enumerate().skip(skip_count).take(VISIBLE_COLUMNS).map(|(col_idx, c)| {
                        let is_selected_cell = is_current_row && col_idx == self.selected_cell_index;
                        let cell = create_centered_cell(c, ROW_HEIGHT);
                        if is_selected_cell {
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
                .filter_map(|(idx, _row)| self.query_results.get(*idx).map(|row| (*idx, row)))
                .map(|(row_idx, r)| {
                    let is_current_row = row_idx == self.selected_row_index;
                    let cells = r.iter().enumerate().skip(skip_count).take(VISIBLE_COLUMNS).map(|(col_idx, c)| {
                        let is_selected_cell = is_current_row && col_idx == self.selected_cell_index;
                        let cell = create_centered_cell(c, ROW_HEIGHT);
                        if is_selected_cell {
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

        let table_block = Block::default()
            .borders(Borders::ALL)
            .title("[3] Results - Cell Selection")
            .title_style(theme::warning())
            .border_style(if is_results_focused {
                theme::border_focused()
            } else {
                theme::border_normal()
            })
            .border_type(BorderType::Rounded);

        // Determine column widths with even distribution and min width
        let available_width = table_chunks[0].width.saturating_sub(4); // Account for borders and padding
        let base_width = available_width / VISIBLE_COLUMNS as u16;
        let widths: Vec<Constraint> = (0..VISIBLE_COLUMNS)
            .map(|_| Constraint::Length(base_width.max(15)))
            .collect();

        let table = Table::new(rows, widths)
            .header(header)
            .block(table_block)
            .row_highlight_style(Style::default()) // Don't use row highlight style in cell mode
            .style(theme::bg_primary())
            .column_spacing(1);

        f.render_stateful_widget(table, table_chunks[0], &mut table_state);

        Ok(chunks)
    }

    fn render_preview_popup(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
        // First render the underlying table
        self.render_query_results_table(f, chunks.clone())?;

        // Calculate popup area (centered, 70% width, 80% height)
        let area = chunks[chunks.len() - 1];
        let popup_width = (area.width as f32 * 0.7) as u16;
        let popup_height = (area.height as f32 * 0.8) as u16;
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        };

        // Clear the popup area
        f.render_widget(Clear, popup_area);

        // Render the preview
        let preview_block = Block::default()
            .title(format!("Row {} of {} - [j/k] Scroll [Enter/Space/y] Copy Value [Y] Copy JSON [Esc] Close", 
                          self.selected_row_index + 1, self.query_results.len()))
            .borders(Borders::ALL)
            .border_style(theme::border_focused())
            .border_type(BorderType::Rounded);

        let inner = preview_block.inner(popup_area);
        f.render_widget(preview_block, popup_area);

        if let Some(row) = self.query_results.get(self.selected_row_index) {
            // Create a vertical table with two columns: Column Name | Value
            let skip_count = self.preview_scroll_offset as usize;
            let visible_rows = inner.height.saturating_sub(2) as usize; // Account for header
            
            // Build table rows for each column/value pair
            let table_rows: Vec<ratatui::widgets::Row> = self.selected_headers
                .iter()
                .zip(row.iter())
                .skip(skip_count)
                .take(visible_rows)
                .enumerate()
                .map(|(idx, (header, value))| {
                    let cells = vec![
                        Cell::from(header.as_str()),
                        Cell::from(if value.is_empty() {
                            "(empty)"
                        } else if value == "NULL" {
                            "NULL"
                        } else {
                            value.as_str()
                        })
                    ];
                    
                    // Highlight the selected row
                    if skip_count + idx == self.preview_selected_index {
                        ratatui::widgets::Row::new(cells).height(1).style(theme::selection_active())
                    } else {
                        ratatui::widgets::Row::new(cells).height(1)
                    }
                })
                .collect();
            
            // Create header row
            let header = ratatui::widgets::Row::new(vec![
                Cell::from("Column").style(theme::header()),
                Cell::from("Value").style(theme::header())
            ]).height(1);
            
            // Calculate column widths (30% for column names, 70% for values)
            let widths = vec![
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ];
            
            // Create table
            let table = Table::new(table_rows, widths)
                .header(header)
                .block(Block::default())
                .column_spacing(1);
            
            f.render_widget(table, inner);
            
            // Show scroll indicator with selected row
            let total_rows = self.selected_headers.len();
            let scroll_info = format!(" {} of {} ", 
                                    self.preview_selected_index + 1,
                                    total_rows);
            let scroll_info_len = scroll_info.len() as u16;
            let scroll_pos = inner.x + inner.width.saturating_sub(scroll_info_len + 1);
            
            f.render_widget(
                Paragraph::new(scroll_info).style(theme::muted()),
                Rect {
                    x: scroll_pos,
                    y: inner.y + inner.height - 1,
                    width: scroll_info_len,
                    height: 1,
                }
            );
        }

        Ok(chunks)
    }

    fn render_error(&mut self, f: &mut Frame<'_>) -> Result<()> {
        if let Some(error) = &self.error_message {
            let area = self.centered_rect(60, 20, f.area());
            f.render_widget(Clear, area);

            let error_block = Block::default()
                .title("Error")
                .borders(Borders::ALL)
                .border_style(theme::error())
                .border_type(BorderType::Rounded);

            let error_text = Paragraph::new(error.as_str())
                .block(error_block)
                .style(theme::error())
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);

            f.render_widget(error_text, area);
        }
        Ok(())
    }

    fn render_help(&mut self, f: &mut Frame<'_>) -> Result<()> {
        if self.show_help {
            let area = self.centered_rect(80, 80, f.area());
            f.render_widget(Clear, area);

            let help_block = Block::default()
                .title("Help - Press ? to close")
                .borders(Borders::ALL)
                .border_style(theme::border_focused())
                .border_type(BorderType::Rounded);

            let help_text = vec![
                Line::from(vec![Span::styled("Navigation", theme::header())]),
                Line::from(""),
                Line::from("1 - Switch to Tables tab"),
                Line::from("2 - Switch to Query tab"),
                Line::from("3 - Switch to Results tab"),
                Line::from("t - Toggle between Query and History (in Query tab)"),
                Line::from(""),
                Line::from(vec![Span::styled("Tables Tab", theme::header())]),
                Line::from(""),
                Line::from("↑/↓, k/j - Navigate tables"),
                Line::from("Enter - View table columns"),
                Line::from("i - View table schema"),
                Line::from("s - Select table for query"),
                Line::from("/ - Search tables"),
                Line::from(""),
                Line::from(vec![Span::styled("Query Tab", theme::header())]),
                Line::from(""),
                Line::from("Enter - Execute query (in Normal mode)"),
                Line::from("Ctrl+Space - Trigger autocomplete"),
                Line::from("Ctrl+u - Clear query editor"),
                Line::from("t - Toggle to History"),
                Line::from(""),
                Line::from(vec![Span::styled("Query Formatting", theme::header())]),
                Line::from("== - Format entire query"),
                Line::from("= (visual mode) - Format selection"),
                Line::from("=G - Format to end of file"),
                Line::from("=a - Toggle auto-format on execute"),
                Line::from(""),
                Line::from(vec![Span::styled("History Tab", theme::header())]),
                Line::from(""),
                Line::from("Enter - Execute selected query"),
                Line::from("c - Copy query to editor"),
                Line::from("y - Copy query to clipboard"),
                Line::from("d - Delete query from history"),
                Line::from(""),
                Line::from(vec![Span::styled("Results Tab", theme::header())]),
                Line::from(""),
                Line::from("↑/↓, k/j - Navigate rows"),
                Line::from("←/→, h/l - Navigate columns (scroll in Table mode)"),
                Line::from("Space - Toggle row detail view"),
                Line::from("p - Preview row in popup"),
                Line::from("v - Enter cell selection mode"),
                Line::from("r - Re-run last query"),
                Line::from("/ - Search results"),
                Line::from("e - Export results to CSV"),
                Line::from(""),
                Line::from(vec![Span::styled("Copy Commands", theme::header())]),
                Line::from(""),
                Line::from("y - Copy current cell/row"),
                Line::from("Y - Copy entire row as TSV"),
                Line::from(""),
                Line::from(vec![Span::styled("General", theme::header())]),
                Line::from(""),
                Line::from("? - Toggle this help"),
                Line::from("Esc - Cancel/Go back"),
            ];

            let help_paragraph = Paragraph::new(help_text)
                .block(help_block)
                .style(theme::bg_primary())
                .wrap(Wrap { trim: false });

            f.render_widget(help_paragraph, area);
        }
        Ok(())
    }

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
                .row_highlight_style(theme::selection_active());
            
            f.render_widget(table, area);
        } else if self.show_table_schema {
            // Show schema as scrollable text
            let paragraph = Paragraph::new(self.selected_table_schema.as_str())
                .block(block)
                .style(theme::bg_primary())
                .wrap(Wrap { trim: false })
                .scroll((self.table_info_scroll as u16, 0));
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

    fn render_loading_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        // Calculate centered position for loading indicator
        let popup_area = self.centered_rect(40, 30, area);
        
        // Clear the area
        f.render_widget(Clear, popup_area);
        
        // Create loading block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_focused())
            .border_type(BorderType::Rounded)
            .style(theme::bg_primary());
        
        // Calculate elapsed time
        let elapsed = self.query_start_time
            .map(|start| start.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        
        // Create spinner animation
        let spinner = match (elapsed * 4.0) as usize % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        };
        
        let loading_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(spinner, theme::warning()),
                Span::raw(" Executing query..."),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{:.1}s", elapsed), theme::muted()),
            ]),
        ];
        
        let paragraph = Paragraph::new(loading_text)
            .block(block)
            .alignment(Alignment::Center);
        
        f.render_widget(paragraph, popup_area);
    }

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

        // Create the autocomplete block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_normal())
            .border_type(BorderType::Rounded);

        let inner = block.inner(popup_area);

        // Create list items with icons
        let items: Vec<ListItem> = self.autocomplete_state.suggestions
            .iter()
            .enumerate()
            .map(|(idx, suggestion)| {
                let icon = match suggestion.kind {
                    SuggestionKind::Table => "T",
                    SuggestionKind::Column => "C", 
                    SuggestionKind::Keyword => "K",
                };
                
                let style = if idx == self.autocomplete_state.selected_index {
                    theme::selection_active()
                } else {
                    theme::bg_primary()
                };
                
                ListItem::new(Line::from(vec![
                    Span::raw(format!("[{}] ", icon)),
                    Span::styled(&suggestion.text, style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .style(theme::bg_primary());

        f.render_widget(block, popup_area);
        f.render_widget(list, inner);
        
        Ok(())
    }

    fn render_explain_output(&mut self, f: &mut Frame<'_>, table_chunks: Rc<[Rect]>) -> Result<()> {
        let is_focused = self.selected_component == ComponentKind::Results;
        
        // Create a block for EXPLAIN output
        let explain_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_focused { theme::border_focused() } else { theme::border_normal() })
            .title("[3] Query Results - EXPLAIN View [x] Toggle View [c] Copy")
            .title_style(theme::title())
            .border_type(BorderType::Rounded);

        // Render the EXPLAIN output
        if self.selected_headers.len() == 1 && self.selected_headers[0].to_lowercase().contains("query plan") {
            // PostgreSQL-style EXPLAIN output - render as a formatted text
            self.render_explain_text_output(f, table_chunks[0], explain_block)?;
        } else {
            // SQLite-style or EXPLAIN ANALYZE with multiple columns - render as table
            self.render_explain_table_output(f, table_chunks[0], explain_block)?;
        }
        
        // Render status line (including export_status for copy success message)
        self.render_explain_status_line(f, table_chunks[1])?;
        
        Ok(())
    }

    fn render_explain_text_output(&mut self, f: &mut Frame<'_>, area: Rect, block: Block) -> Result<()> {
        // Collect all rows into a single text with proper indentation
        let mut lines: Vec<Line> = Vec::new();
        
        for row in &self.query_results {
            if let Some(plan_text) = row.first() {
                // Parse indentation level from leading spaces/dashes
                let _indent_level = plan_text.chars().take_while(|c| c.is_whitespace() || *c == '-' || *c == '>').count();
                
                // Apply color based on keywords in the line
                let style = if plan_text.contains("Seq Scan") || plan_text.contains("Sequential Scan") {
                    theme::warning() // Yellow for sequential scans
                } else if plan_text.contains("Index Scan") || plan_text.contains("Bitmap Index Scan") {
                    theme::success() // Green for index usage
                } else if plan_text.contains("Hash Join") || plan_text.contains("Nested Loop") {
                    theme::info() // Blue for joins
                } else if plan_text.contains("Sort") || plan_text.contains("ORDER BY") {
                    theme::muted() // Dim for sorts
                } else {
                    theme::bg_primary()
                };
                
                lines.push(Line::from(Span::styled(plan_text.clone(), style)));
            }
        }
        
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((self.selected_row_index as u16, 0));
            
        f.render_widget(paragraph, area);
        
        // Render scroll indicator
        let max_scroll = self.query_results.len().saturating_sub(area.height as usize - 2);
        if max_scroll > 0 {
            let scroll_indicator = format!(" {}/{} ", self.selected_row_index + 1, self.query_results.len());
            let indicator_area = Rect {
                x: area.x + area.width - scroll_indicator.len() as u16 - 2,
                y: area.y,
                width: scroll_indicator.len() as u16,
                height: 1,
            };
            f.render_widget(Span::styled(scroll_indicator, theme::muted()), indicator_area);
        }
        
        Ok(())
    }

    fn render_explain_table_output(&mut self, f: &mut Frame<'_>, area: Rect, block: Block) -> Result<()> {
        // Render as a regular table but with special highlighting for EXPLAIN columns
        let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;
        
        // Create header with special styling
        let header_cells: Vec<_> = self
            .selected_headers
            .iter()
            .skip(skip_count)
            .take(VISIBLE_COLUMNS)
            .map(|h| Cell::from(h.as_str()).style(theme::header()))
            .collect();
        let header = ratatui::widgets::Row::new(header_cells)
            .height(1)
            .style(theme::header())
            .bottom_margin(1);

        // Create rows with special coloring for performance metrics
        let rows: Vec<ratatui::widgets::Row> = self
            .query_results
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let is_selected = idx == self.selected_row_index;
                let cells: Vec<Cell> = row
                    .iter()
                    .skip(skip_count)
                    .take(VISIBLE_COLUMNS)
                    .enumerate()
                    .map(|(col_idx, value)| {
                        let mut style = theme::bg_primary();
                        
                        // Apply special styling based on column content
                        if self.selected_headers.get(col_idx + skip_count).map(|h| h.to_lowercase().contains("time")).unwrap_or(false) {
                            // Highlight slow operations in red
                            if let Ok(time_ms) = value.trim_end_matches("ms").trim().parse::<f64>() {
                                if time_ms > 1000.0 {
                                    style = theme::error();
                                } else if time_ms > 100.0 {
                                    style = theme::warning();
                                }
                            }
                        }
                        
                        Cell::from(value.as_str()).style(style)
                    })
                    .collect();
                    
                let row_style = if is_selected {
                    theme::selection_active()
                } else {
                    theme::bg_primary()
                };
                
                ratatui::widgets::Row::new(cells).style(row_style)
            })
            .collect();

        let table = Table::new(rows, [Constraint::Percentage(100 / VISIBLE_COLUMNS as u16); VISIBLE_COLUMNS])
            .header(header)
            .block(block)
            .row_highlight_style(theme::selection_active())
            .highlight_symbol("→ ");

        let mut table_state = TableState::default();
        table_state.select(Some(self.selected_row_index));

        f.render_stateful_widget(table, area, &mut table_state);
        
        Ok(())
    }
    
    fn render_explain_status_line(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let mut status_parts = vec![];
        
        // Add row count
        status_parts.push(format!("Rows: {}", self.query_results.len()));
        
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
        f.render_widget(status_text, area);
        
        Ok(())
    }
}

/// Creates a `Cell` with text that is vertically centered.
fn create_centered_cell(text: &str, row_height: u16) -> Cell<'static> {
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

    // Add the actual text lines
    for line in text_lines {
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

