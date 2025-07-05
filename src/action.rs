use serde::{Deserialize, Serialize};
use strum::Display;

use crate::components::{db::{DbColumn, DbTable}, ComponentKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Display, Deserialize)]
pub enum Action {
  Tick,
  Render,
  Resize(u16, u16),
  Suspend,
  Resume,
  Quit,
  Refresh,
  Error(String),
  Help,
  TablesLoaded(Vec<DbTable>),
  TableMoveUp,
  TableMoveDown,
  RowMoveUp,
  RowMoveDown,
  ScrollTableLeft,
  ScrollTableRight,
  LoadSelectedTable,
  LoadTables(String),
  LoadTable(String),
  ViewTableColumns,
  ViewTableSchema,
  TableColumnsLoaded(String, Vec<DbColumn>), // table_name, columns
  QueryResult(Vec<String>, Vec<Vec<String>>),
  QueryExecutionTime(u64), // milliseconds
  FocusQuery,
  FocusResults,
  FocusHome,
  SelectComponent(ComponentKind),
  ExecuteQuery,
  HandleQuery(String),
  QueryStarted,
  QueryCompleted,
  RowDetails,
  SwitchEditor,
  ClearQuery,
  TriggerAutocomplete,
  UpdateAutocompleteDocument(String),
  RequestAutocomplete {
    text: String,
    cursor_line: usize,
    cursor_col: usize,
    context: String, // Serialized SqlContext
  },
  AutocompleteResults(Vec<(String, String)>), // Vec of (text, kind)
  SetTunnelMode(bool), // Notify components about tunnel mode
  ExportResultsToCsv,
  RowJumpToTop,
  RowJumpToBottom,
  TableJumpToTop,
  TableJumpToBottom,
  RowPageUp,
  RowPageDown,
  TablePageUp,
  TablePageDown,
  FormatQuery,
  FormatSelection,
  ToggleAutoFormat,
  ExplainQuery,
  ExplainAnalyzeQuery,
  ToggleExplainView,
  ToggleExplainAnalyze,
  CopyExplainResults,
}
