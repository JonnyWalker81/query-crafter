use std::collections::HashMap;

use nucleo::{Config, Matcher, Utf32Str};

use crate::components::db::{DbColumn, DbTable};

/// Represents different types of autocomplete suggestions
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionKind {
  Table,
  Column,
  Keyword,
}

/// A single autocomplete suggestion with metadata
#[derive(Debug, Clone)]
pub struct SuggestionItem {
  pub text: String,
  pub kind: SuggestionKind,
  pub score: u32,
  pub table_context: Option<String>, // For columns, which table they belong to
}

impl SuggestionItem {
  pub fn new_table(name: String) -> Self {
    Self { text: name, kind: SuggestionKind::Table, score: 0, table_context: None }
  }

  pub fn new_column(name: String, table: String) -> Self {
    Self { text: name, kind: SuggestionKind::Column, score: 0, table_context: Some(table) }
  }

  pub fn new_keyword(keyword: String) -> Self {
    Self { text: keyword, kind: SuggestionKind::Keyword, score: 0, table_context: None }
  }

  pub fn with_score(mut self, score: u32) -> Self {
    self.score = score;
    self
  }
}

/// Manages the current state of autocomplete
#[derive(Debug, Default)]
pub struct AutocompleteState {
  pub suggestions: Vec<SuggestionItem>,
  pub selected_index: usize,
  pub is_active: bool,
  pub trigger_position: usize, // Cursor position where autocomplete was triggered
  pub current_word: String,    // The word being completed
}

impl AutocompleteState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn activate(&mut self, position: usize, word: String) {
    self.is_active = true;
    self.trigger_position = position;
    self.current_word = word;
    self.selected_index = 0;
  }

  pub fn deactivate(&mut self) {
    self.is_active = false;
    self.suggestions.clear();
    self.selected_index = 0;
    self.current_word.clear();
  }

  pub fn select_next(&mut self) {
    if !self.suggestions.is_empty() {
      self.selected_index = (self.selected_index + 1) % self.suggestions.len();
    }
  }

  pub fn select_previous(&mut self) {
    if !self.suggestions.is_empty() {
      if self.selected_index == 0 {
        self.selected_index = self.suggestions.len() - 1;
      } else {
        self.selected_index -= 1;
      }
    }
  }

  pub fn get_selected_suggestion(&self) -> Option<&SuggestionItem> {
    self.suggestions.get(self.selected_index)
  }

  pub fn update_suggestions(&mut self, suggestions: Vec<SuggestionItem>) {
    self.suggestions = suggestions;
    self.selected_index = 0; // Reset selection when suggestions change
  }
}

/// SQL context information for determining what kind of suggestions to show
#[derive(Debug, PartialEq)]
pub enum SqlContext {
  None,
  TableName,
  ColumnName { table: Option<String> },
  AfterSelect,
  AfterFrom,
  AfterWhere,
}

/// Simple SQL token parser to determine context for autocomplete
pub struct SqlParser;

impl SqlParser {
  /// Analyzes the SQL text up to the cursor position to determine context
  pub fn analyze_context(sql: &str, cursor_pos: usize) -> (SqlContext, String) {
    let text_before_cursor = if cursor_pos <= sql.len() { &sql[..cursor_pos] } else { sql };

    // Get the current word being typed
    let current_word = Self::get_current_word(text_before_cursor);

    // Convert to uppercase for keyword matching
    let upper_text = text_before_cursor.to_uppercase();
    let tokens: Vec<&str> = upper_text.split_whitespace().collect();

    if tokens.is_empty() {
      return (SqlContext::None, current_word);
    }

    // Check context based on recent tokens
    match tokens.last() {
      Some(&"SELECT") => (SqlContext::AfterSelect, current_word),
      Some(&"FROM") => (SqlContext::AfterFrom, current_word),
      Some(&"WHERE") => (SqlContext::AfterWhere, current_word),
      _ => {
        // Look for more complex patterns
        if let Some(context) = Self::analyze_complex_context(&tokens) {
          (context, current_word)
        } else {
          (SqlContext::None, current_word)
        }
      },
    }
  }

  fn get_current_word(text: &str) -> String {
    // Find the last word that might be partially typed
    let chars: Vec<char> = text.chars().collect();
    let mut end = chars.len();

    // Skip trailing whitespace
    while end > 0 && chars[end - 1].is_whitespace() {
      end -= 1;
    }

    let mut start = end;
    // Go back to find the start of the current word
    while start > 0 && !chars[start - 1].is_whitespace() {
      start -= 1;
    }

    chars[start..end].iter().collect()
  }

  fn analyze_complex_context(tokens: &[&str]) -> Option<SqlContext> {
    // Look for patterns like "SELECT ... FROM" or "table_name."
    for (i, &token) in tokens.iter().enumerate() {
      match token {
        "FROM" => {
          // After FROM, we expect table names
          return Some(SqlContext::TableName);
        },
        "SELECT" => {
          // After SELECT, we expect column names or table names
          if i == tokens.len() - 1 {
            return Some(SqlContext::AfterSelect);
          }
          // Check if we're still in the SELECT clause
          let has_from = tokens[i..].contains(&"FROM");
          if !has_from {
            return Some(SqlContext::ColumnName { table: None });
          }
        },
        _ => {},
      }
    }

    None
  }
}

/// Provides autocomplete suggestions based on available schema and context
pub struct AutocompleteProvider {
  tables: Vec<DbTable>,
  table_columns_cache: HashMap<String, Vec<DbColumn>>,
  sql_keywords: Vec<String>,
  matcher: Matcher,
}

impl Default for AutocompleteProvider {
  fn default() -> Self {
    Self::new()
  }
}

impl AutocompleteProvider {
  pub fn new() -> Self {
    let sql_keywords = vec![
      "SELECT".to_string(),
      "FROM".to_string(),
      "WHERE".to_string(),
      "INSERT".to_string(),
      "UPDATE".to_string(),
      "DELETE".to_string(),
      "CREATE".to_string(),
      "DROP".to_string(),
      "ALTER".to_string(),
      "TABLE".to_string(),
      "INDEX".to_string(),
      "VIEW".to_string(),
      "JOIN".to_string(),
      "INNER".to_string(),
      "LEFT".to_string(),
      "RIGHT".to_string(),
      "OUTER".to_string(),
      "ON".to_string(),
      "GROUP".to_string(),
      "BY".to_string(),
      "ORDER".to_string(),
      "HAVING".to_string(),
      "LIMIT".to_string(),
      "OFFSET".to_string(),
      "UNION".to_string(),
      "DISTINCT".to_string(),
      "COUNT".to_string(),
      "SUM".to_string(),
      "AVG".to_string(),
      "MAX".to_string(),
      "MIN".to_string(),
    ];

    Self {
      tables: Vec::new(),
      table_columns_cache: HashMap::new(),
      sql_keywords,
      matcher: Matcher::new(Config::DEFAULT),
    }
  }

  pub fn update_tables(&mut self, tables: Vec<DbTable>) {
    self.tables = tables;
  }

  pub fn update_table_columns(&mut self, table_name: String, columns: Vec<DbColumn>) {
    self.table_columns_cache.insert(table_name, columns);
  }

  /// Generate suggestions based on context and current input
  pub fn get_suggestions(&mut self, context: SqlContext, input: &str) -> Vec<SuggestionItem> {
    let mut suggestions = Vec::new();

    match context {
      SqlContext::TableName | SqlContext::AfterFrom => {
        suggestions.extend(self.get_table_suggestions(input));
      },
      SqlContext::ColumnName { table } => {
        suggestions.extend(self.get_column_suggestions(input, table.as_deref()));
      },
      SqlContext::AfterSelect => {
        // Both columns and keywords are valid after SELECT
        suggestions.extend(self.get_column_suggestions(input, None));
        suggestions.extend(self.get_keyword_suggestions(input));
      },
      SqlContext::AfterWhere => {
        // Columns and some keywords are valid after WHERE
        suggestions.extend(self.get_column_suggestions(input, None));
        suggestions.extend(self.get_keyword_suggestions(input));
      },
      SqlContext::None => {
        // Show keywords by default
        suggestions.extend(self.get_keyword_suggestions(input));
      },
    }

    // Sort by score (descending) and limit results
    suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    suggestions.truncate(20); // Limit to 20 suggestions for performance

    suggestions
  }

  fn get_table_suggestions(&mut self, input: &str) -> Vec<SuggestionItem> {
    let mut suggestions = Vec::new();

    // Clone the tables to avoid borrowing conflicts
    let tables = self.tables.clone();
    for table in &tables {
      if let Some(score) = self.fuzzy_match(input, &table.name) {
        suggestions.push(SuggestionItem::new_table(table.name.clone()).with_score(score));
      }
    }

    suggestions
  }

  fn get_column_suggestions(&mut self, input: &str, table_context: Option<&str>) -> Vec<SuggestionItem> {
    let mut suggestions = Vec::new();

    if let Some(table_name) = table_context {
      // Show columns for specific table
      if let Some(columns) = self.table_columns_cache.get(table_name).cloned() {
        for column in &columns {
          if let Some(score) = self.fuzzy_match(input, &column.name) {
            suggestions.push(SuggestionItem::new_column(column.name.clone(), table_name.to_string()).with_score(score));
          }
        }
      }
    } else {
      // Show columns from all tables
      let cache_clone = self.table_columns_cache.clone();
      for (table_name, columns) in &cache_clone {
        for column in columns {
          if let Some(score) = self.fuzzy_match(input, &column.name) {
            suggestions.push(SuggestionItem::new_column(column.name.clone(), table_name.clone()).with_score(score));
          }
        }
      }
    }

    suggestions
  }

  fn get_keyword_suggestions(&mut self, input: &str) -> Vec<SuggestionItem> {
    let mut suggestions = Vec::new();

    // Clone keywords to avoid borrowing conflicts
    let keywords = self.sql_keywords.clone();
    for keyword in &keywords {
      if let Some(score) = self.fuzzy_match(input, keyword) {
        suggestions.push(SuggestionItem::new_keyword(keyword.clone()).with_score(score));
      }
    }

    suggestions
  }

  fn fuzzy_match(&mut self, pattern: &str, text: &str) -> Option<u32> {
    if pattern.is_empty() {
      return Some(100); // Empty pattern matches everything with high score
    }

    let mut pattern_buf = Vec::new();
    let mut text_buf = Vec::new();
    let pattern_utf32 = Utf32Str::new(pattern, &mut pattern_buf);
    let text_utf32 = Utf32Str::new(text, &mut text_buf);

    self.matcher.fuzzy_match(text_utf32, pattern_utf32).map(|score| score as u32)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sql_context_analysis() {
    let (context, word) = SqlParser::analyze_context("SELECT ", 7);
    assert_eq!(context, SqlContext::AfterSelect);
    assert_eq!(word, "");

    let (context, word) = SqlParser::analyze_context("SELECT name FROM ", 17);
    assert_eq!(context, SqlContext::AfterFrom);
    assert_eq!(word, "");

    let (context, word) = SqlParser::analyze_context("SELECT name FROM tab", 20);
    assert_eq!(context, SqlContext::TableName);
    assert_eq!(word, "tab");
  }

  #[test]
  fn test_autocomplete_state() {
    let mut state = AutocompleteState::new();
    assert!(!state.is_active);

    state.activate(10, "tab".to_string());
    assert!(state.is_active);
    assert_eq!(state.trigger_position, 10);
    assert_eq!(state.current_word, "tab");

    state.deactivate();
    assert!(!state.is_active);
    assert!(state.suggestions.is_empty());
  }
}
