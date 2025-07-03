use serde::{Deserialize, Serialize};
use strum::Display;

use crate::components::{db::DbTable, ComponentKind};

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
  QueryResult(Vec<String>, Vec<Vec<String>>),
  FocusQuery,
  FocusResults,
  FocusHome,
  SelectComponent(ComponentKind),
  ExecuteQuery,
  HandleQuery(String),
  RowDetails,
  SwitchEditor,
  ClearQuery,
}
