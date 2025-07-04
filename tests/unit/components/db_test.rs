use query_crafter::components::db::{Db, DbTable, DbColumn};
use query_crafter::components::{Component, ComponentKind};
use query_crafter::action::Action;
use tokio::sync::mpsc;
use ratatui::layout::Rect;

async fn create_test_db() -> Db {
    Db::new()
}

#[tokio::test]
async fn test_db_component_initialization() {
    let mut db = create_test_db().await;
    
    // Register action handler
    let (tx, _rx) = mpsc::unbounded_channel();
    db.register_action_handler(tx).unwrap();
    
    // Initialize with area
    let area = Rect::new(0, 0, 80, 24);
    db.init(area).unwrap();
}

#[tokio::test]
async fn test_table_navigation() {
    let mut db = create_test_db().await;
    
    // Load some test tables
    let tables = vec![
        DbTable {
            name: "users".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
        DbTable {
            name: "posts".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
        DbTable {
            name: "comments".to_string(),
            schema: "public".to_string(),
            columns: vec![],
        },
    ];
    
    let action = Action::TablesLoaded(tables);
    let result = db.update(action).unwrap();
    assert!(result.is_none());
    
    // Test moving down
    let result = db.update(Action::TableMoveDown).unwrap();
    assert!(result.is_none());
    
    // Test moving up
    let result = db.update(Action::TableMoveUp).unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_query_execution_flow() {
    let mut db = create_test_db().await;
    
    // Set query text
    let result = db.update(Action::HandleQuery("SELECT * FROM users".to_string()));
    assert!(result.is_ok());
    
    // Mark query as started
    let result = db.update(Action::QueryStarted);
    assert!(result.is_ok());
    
    // Provide query results
    let headers = vec!["id".to_string(), "name".to_string(), "email".to_string()];
    let results = vec![
        vec!["1".to_string(), "Alice".to_string(), "alice@example.com".to_string()],
        vec!["2".to_string(), "Bob".to_string(), "bob@example.com".to_string()],
    ];
    
    let result = db.update(Action::QueryResult(headers, results));
    assert!(result.is_ok());
    
    // Mark query as completed
    let result = db.update(Action::QueryCompleted);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_error_handling() {
    let mut db = create_test_db().await;
    
    // Send an error
    let error_msg = "Connection failed".to_string();
    let result = db.update(Action::Error(error_msg.clone()));
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_component_focus() {
    let mut db = create_test_db().await;
    
    // Test focus changes
    let result = db.update(Action::FocusQuery).unwrap();
    assert_eq!(result, Some(Action::SelectComponent(ComponentKind::Query)));
    
    let result = db.update(Action::FocusResults).unwrap();
    assert_eq!(result, Some(Action::SelectComponent(ComponentKind::Results)));
    
    let result = db.update(Action::FocusHome).unwrap();
    assert_eq!(result, Some(Action::SelectComponent(ComponentKind::Home)));
}

#[tokio::test]
async fn test_table_columns_loading() {
    let mut db = create_test_db().await;
    
    let columns = vec![
        DbColumn {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            is_nullable: false,
        },
        DbColumn {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            is_nullable: false,
        },
    ];
    
    let result = db.update(Action::TableColumnsLoaded("users".to_string(), columns));
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_clear_query() {
    let mut db = create_test_db().await;
    
    // Clear query should clear the editor
    let result = db.update(Action::ClearQuery);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_format_actions() {
    let mut db = create_test_db().await;
    
    // Test format query action
    let result = db.update(Action::FormatQuery);
    assert!(result.is_ok());
    
    // Test toggle auto-format
    let result = db.update(Action::ToggleAutoFormat);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_jump_navigation() {
    let mut db = create_test_db().await;
    
    // Load some results first
    let headers = vec!["id".to_string(), "name".to_string()];
    let mut results = vec![];
    for i in 1..=20 {
        results.push(vec![i.to_string(), format!("Item {}", i)]);
    }
    
    db.update(Action::QueryResult(headers, results)).unwrap();
    
    // Test jump to top
    let result = db.update(Action::RowJumpToTop);
    assert!(result.is_ok());
    
    // Test jump to bottom
    let result = db.update(Action::RowJumpToBottom);
    assert!(result.is_ok());
    
    // Test page navigation
    let result = db.update(Action::RowPageDown);
    assert!(result.is_ok());
    
    let result = db.update(Action::RowPageUp);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_csv() {
    let mut db = create_test_db().await;
    
    // Load some results first
    let headers = vec!["id".to_string(), "name".to_string()];
    let results = vec![
        vec!["1".to_string(), "Test Item".to_string()],
    ];
    
    db.update(Action::QueryResult(headers, results)).unwrap();
    
    // Test export action
    let result = db.update(Action::ExportResultsToCsv);
    assert!(result.is_ok());
}

#[cfg(test)]
mod key_event_tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    
    fn create_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }
    
    #[tokio::test]
    async fn test_search_functionality() {
        let mut db = create_test_db().await;
        
        // Start table search with '/'
        let event = create_key_event(KeyCode::Char('/'), KeyModifiers::empty());
        let result = db.handle_key_events(event);
        assert!(result.is_ok());
        
        // Type search query
        for ch in "users".chars() {
            let event = create_key_event(KeyCode::Char(ch), KeyModifiers::empty());
            db.handle_key_events(event).unwrap();
        }
        
        // Complete search with Enter
        let event = create_key_event(KeyCode::Enter, KeyModifiers::empty());
        db.handle_key_events(event).unwrap();
    }
    
    #[tokio::test]
    async fn test_help_toggle() {
        let mut db = create_test_db().await;
        
        // Toggle help with '?'
        let event = create_key_event(KeyCode::Char('?'), KeyModifiers::empty());
        let result = db.handle_key_events(event);
        assert!(result.is_ok());
        
        // Close help with ESC
        let event = create_key_event(KeyCode::Esc, KeyModifiers::empty());
        let result = db.handle_key_events(event);
        assert!(result.is_ok());
    }
}