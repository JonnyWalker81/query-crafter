use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Table,   // Normal table navigation
    Row,     // Row is selected for detail view
    Cell,    // Individual cell selection
    Preview, // Popup preview mode
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryEntry {
    pub query: String,
    pub timestamp: u64,
    pub row_count: Option<usize>,
    pub execution_time_ms: Option<u64>,
    pub error: Option<String>,
}

impl QueryHistoryEntry {
    pub fn new(query: String) -> Self {
        Self {
            query,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            row_count: None,
            execution_time_ms: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DbColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DbTable {
    pub schema: String,
    pub name: String,
    pub columns: Vec<DbColumn>,
}