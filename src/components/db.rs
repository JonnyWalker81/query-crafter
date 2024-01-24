use std::{
  collections::{BTreeMap, HashMap},
  fmt::Display,
  rc::Rc,
  time::Duration,
};

use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Postgres, Row};
use strum::Display;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tui_popup::Popup;
use tui_textarea::{Input, TextArea};

use super::{
  vim::{Mode, Transition},
  Component, ComponentKind, Frame,
};
use crate::{
  action::Action,
  components::vim::Vim,
  config::{Config, KeyBindings},
};

const VISIBLE_COLUMNS: usize = 3;

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
}

impl<'a> Db<'a> {
  pub fn new() -> Self {
    Self::default()
  }

  fn column_count(&self) -> usize {
    self.selected_headers.len()
  }

  fn json(&self) -> Option<String> {
    if self.query_results.is_empty() {
      return None;
    }

    let row_data = self.query_results[self.selected_row_index].iter().zip(self.selected_headers.iter()).fold(
      BTreeMap::new(),
      |mut acc, (value, header)| {
        acc.insert(header, value);
        acc
      },
    );
    let json_str = serde_json::to_string_pretty(&row_data).unwrap();
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
        Paragraph::new(Text::styled(format!("{}", self.table_search_query), Style::default().fg(Color::Yellow)))
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
      .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
      .split(chunks[1]);

    let query_border_color = if self.selected_component == ComponentKind::Query { Color::Cyan } else { Color::White };
    let border_style = Style::default().fg(query_border_color);
    let input_block = Block::default().borders(Borders::ALL).border_style(border_style).title("Query");
    let style = ratatui::style::Style::default().bg(query_border_color).add_modifier(Modifier::REVERSED);
    self.query_input.set_block(input_block);

    f.render_widget(self.query_input.widget(), query_chunks[0]);

    Ok(query_chunks)
  }

  fn render_query_results(&mut self, f: &mut Frame<'_>, chunks: Rc<[Rect]>) -> Result<Rc<[Rect]>> {
    let table_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
      .split(chunks[1]);

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
    if !rows.is_empty() {
      f.render_widget(status_text, table_chunks[1]);
    }

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
      .widths(&[Constraint::Length(40), Constraint::Length(40), Constraint::Length(40)]);

    f.render_stateful_widget(result_table, table_chunks[0], &mut table_state);

    if self.show_row_details {
      if let Some(json_str) = self.json() {
        let popup = Popup::new("Row Details", json_str);
        f.render_widget(popup.to_widget(), f.size());
      }
    }

    Ok(chunks)
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
            self.is_searching_tables = false;
          },
          _ => {},
        }
      },
      ComponentKind::Query => {
        let transition = self.vim_editor.transition(Input::from(key), &mut self.query_input);
        match transition {
          Transition::Mode(mode) if self.vim_editor.mode() != mode => {
            self.query_input.set_cursor_style(mode.cursor_style());
            self.vim_editor = Vim::new(mode);
          },
          Transition::Nop | Transition::Mode(_) => {},
          Transition::Pending(ref input) => {
            let v = self.vim_editor.clone();
            let vim_editor = v.with_pending(input);
          },
          Transition::Quit => {},
        }
        if let Transition::Pending(ref input) = transition {
          if self.vim_editor.mode() == Mode::Normal && key.code == KeyCode::Enter {
            return Ok(Some(Action::HandleQuery(self.query_input.lines().join(" "))));
          }
        }
      },
      ComponentKind::Results => {
        match key.code {
          KeyCode::Char('y') => {
            if let Some(json_str) = self.json() {
              #[cfg(target_os = "linux")]
              Clipboard::new()?.set().wait().text(json_str)?;
            }
          },
          KeyCode::Char('r') => {
            return Ok(Some(Action::HandleQuery(self.query_input.lines().join(" "))));
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
          && self.horizonal_scroll_offset * VISIBLE_COLUMNS < self.column_count() - VISIBLE_COLUMNS
        {
          self.horizonal_scroll_offset += 1;
        }
      },
      Action::RowMoveDown => {
        if !self.query_results.is_empty() {
          if self.selected_component == ComponentKind::Results && self.selected_row_index < self.query_results.len() - 1
          {
            self.selected_row_index += 1;
          }
        }
      },
      Action::RowMoveUp => {
        if self.selected_component == ComponentKind::Results && self.selected_row_index > 0 {
          self.selected_row_index -= 1;
        }
      },
      Action::LoadSelectedTable => {
        if let Some(selected_table) = self.tables.get(self.selected_table_index) {
          let query = format!("SELECT * FROM {}", selected_table.name);
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
        self.selected_component = ComponentKind::Results;
        return Ok(Some(Action::SelectComponent(ComponentKind::Results)));
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
        println!("execute query");
        return Ok(Some(Action::HandleQuery(self.query_input.lines().join(" "))));
      },
      Action::RowDetails => {
        self.show_row_details = !self.show_row_details;
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
      .split(f.size());

    let title_block = Block::default().borders(Borders::ALL).style(Style::default());

    let title = Paragraph::new(Text::styled("Query Crafter", Style::default().fg(Color::Green))).block(title_block);

    f.render_widget(title, chunks[0]);

    let table_chunks = self.render_table_list(f, chunks)?;

    let query_chunks = self.render_query_input(f, table_chunks)?;

    self.render_query_results(f, query_chunks)?;

    Ok(())
  }
}
