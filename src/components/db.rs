use std::{
  collections::{BTreeMap, HashMap},
  fmt::Display,
  time::Duration,
};

use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use ratatui_textarea::{Input, TextArea};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Postgres, Row};
use strum::Display;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use tui_popup::Popup;

use super::{Component, ComponentKind, Frame};
use crate::{
  action::Action,
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
  horizonal_scroll_offset: usize,
  show_row_details: bool,
  table_search_query: String,
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
    // println!("key: {:?}", key);
    match self.selected_component {
      ComponentKind::Home => {
        // Searching for a table
      },
      ComponentKind::Query => {
        // println!("input: {}", c);
        // self.query_input.input(Input { key: , ctrl: false, alt: false });
        match key {
          KeyEvent { modifiers: KeyModifiers::CONTROL, code: KeyCode::Enter, .. } => {
            println!("execute query, ctrl-enter");
            return Ok(Some(Action::HandleQuery(self.query_input.lines().join(" "))));
          },
          _ => {
            let input = Input::from(key);
            // println!("input: {:?}", input);
            self.query_input.input(input);
          },
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
      _ => {},
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
        if self.selected_table_index < self.tables.len() {
          self.selected_table_index += 1;
        } else {
          self.selected_table_index = 0;
        }
      },
      Action::TableMoveUp => {
        if self.selected_table_index > 0 {
          self.selected_table_index -= 1;
        } else {
          self.selected_table_index = self.tables.len() - 1;
        }
      },
      Action::ScrollTableLeft => {
        if self.selected_component == ComponentKind::Results && self.horizonal_scroll_offset > 0 {
          self.horizonal_scroll_offset -= 1;
        }
      },
      Action::ScrollTableRight => {
        // if self.horizonal_scroll_offset < self.column_count() - VISIBLE_COLUMNS {
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
          // return Ok(Some(Action::LoadTable(selected_table.name.clone())));
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
        self.selected_component = ComponentKind::Results;
        return Ok(Some(Action::SelectComponent(ComponentKind::Results)));
      },
      Action::FocusQuery => {
        // println!("focus query");
        self.selected_component = ComponentKind::Query;
        return Ok(Some(Action::SelectComponent(ComponentKind::Query)));
      },
      Action::FocusResults => {
        // println!("focus results");
        self.selected_component = ComponentKind::Results;
        return Ok(Some(Action::SelectComponent(ComponentKind::Results)));
      },
      Action::FocusHome => {
        // println!("focus home");
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
    // f.render_widget(Paragraph::new("db"), area);
    // Create the layout sections.
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(3),
        Constraint::Min(1),
        // Constraint::Length(3),
      ])
      .split(f.size());

    let title_block = Block::default().borders(Borders::ALL).style(Style::default());

    let title = Paragraph::new(Text::styled("Query Crafter", Style::default().fg(Color::Green))).block(title_block);

    f.render_widget(title, chunks[0]);

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

    let mut table_list_state = ListState::default();
    table_list_state.select(Some(self.selected_table_index));
    let items: Vec<ListItem> = self.tables.iter().map(|t| ListItem::new(t.name.to_string())).collect();
    let list = List::new(items)
      .block(tables)
      .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, table_chunks[0], &mut table_list_state);

    let query_chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
      .split(table_chunks[1]);

    let query_border_color = if self.selected_component == ComponentKind::Query { Color::Cyan } else { Color::White };
    let border_style = Style::default().fg(query_border_color);
    let input_block = Block::default().borders(Borders::ALL).border_style(border_style).title("Query");
    let style = ratatui::style::Style::default().bg(query_border_color).add_modifier(Modifier::REVERSED);
    // self.query_input.set_cursor_style(style);
    // self.query_input.set_style(style);
    self.query_input.set_block(input_block);

    f.render_widget(self.query_input.widget(), query_chunks[0]);

    let skip_count = self.horizonal_scroll_offset * VISIBLE_COLUMNS;
    let normal_style = Style::default();
    let header_cells = self
      .selected_headers
      .iter()
      .skip(skip_count)
      .take(VISIBLE_COLUMNS)
      .map(|h| Cell::from(h.to_string()).style(Style::default().fg(Color::Red).bg(Color::Green)));
    let header = ratatui::widgets::Row::new(header_cells).style(normal_style).height(1);

    let rows = self.query_results.iter().map(|r| {
      let cells = r.iter().skip(skip_count).take(VISIBLE_COLUMNS).map(|c| Cell::from(c.to_string()));
      ratatui::widgets::Row::new(cells).height(1).bottom_margin(1)
    });

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

    f.render_stateful_widget(result_table, query_chunks[1], &mut table_state);

    if self.show_row_details {
      if let Some(json_str) = self.json() {
        let popup = Popup::new("Row Details", json_str);
        f.render_widget(popup.to_widget(), f.size());
      }
    }

    Ok(())
  }
}
