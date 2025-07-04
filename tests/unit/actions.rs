use query_crafter::action::Action;
use query_crafter::components::db::{DbTable, DbColumn};

#[test]
fn test_action_creation() {
    // Test simple actions
    let _ = Action::Quit;
    let _ = Action::TableMoveUp;
    let _ = Action::TableMoveDown;
    let _ = Action::RowMoveUp;
    let _ = Action::RowMoveDown;
    let _ = Action::ExecuteQuery;
    let _ = Action::ClearQuery;
}

#[test]
fn test_action_with_data() {
    // Test actions that carry data
    let query = Action::HandleQuery("SELECT * FROM users".to_string());
    match query {
        Action::HandleQuery(q) => assert_eq!(q, "SELECT * FROM users"),
        _ => panic!("Wrong action type"),
    }
    
    let error = Action::Error("Connection failed".to_string());
    match error {
        Action::Error(e) => assert_eq!(e, "Connection failed"),
        _ => panic!("Wrong action type"),
    }
}

#[test]
fn test_query_result_action() {
    let headers = vec!["id".to_string(), "name".to_string()];
    let results = vec![
        vec!["1".to_string(), "Alice".to_string()],
        vec!["2".to_string(), "Bob".to_string()],
    ];
    
    let action = Action::QueryResult(headers.clone(), results.clone());
    
    match action {
        Action::QueryResult(h, r) => {
            assert_eq!(h, headers);
            assert_eq!(r, results);
        }
        _ => panic!("Wrong action type"),
    }
}

#[test]
fn test_table_actions() {
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
    ];
    
    let action = Action::TablesLoaded(tables.clone());
    
    match action {
        Action::TablesLoaded(t) => {
            assert_eq!(t.len(), 2);
            assert_eq!(t[0].name, "users");
            assert_eq!(t[1].name, "posts");
        }
        _ => panic!("Wrong action type"),
    }
}

#[test]
fn test_table_columns_loaded_action() {
    let columns = vec![
        DbColumn {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            is_nullable: false,
        },
        DbColumn {
            name: "email".to_string(),
            data_type: "TEXT".to_string(),
            is_nullable: true,
        },
    ];
    
    let action = Action::TableColumnsLoaded("users".to_string(), columns.clone());
    
    match action {
        Action::TableColumnsLoaded(table, cols) => {
            assert_eq!(table, "users");
            assert_eq!(cols.len(), 2);
            assert_eq!(cols[0].name, "id");
            assert!(!cols[0].is_nullable);
            assert_eq!(cols[1].name, "email");
            assert!(cols[1].is_nullable);
        }
        _ => panic!("Wrong action type"),
    }
}

#[test]
fn test_navigation_actions() {
    // Test all navigation actions exist
    let _ = Action::FocusHome;
    let _ = Action::FocusQuery;
    let _ = Action::FocusResults;
    let _ = Action::LoadSelectedTable;
    let _ = Action::ViewTableColumns;
    let _ = Action::ViewTableSchema;
}

#[test]
fn test_formatting_actions() {
    let _ = Action::FormatQuery;
    let _ = Action::FormatSelection;
    let _ = Action::ToggleAutoFormat;
}

#[test]
fn test_jump_actions() {
    let _ = Action::RowJumpToTop;
    let _ = Action::RowJumpToBottom;
    let _ = Action::TableJumpToTop;
    let _ = Action::TableJumpToBottom;
    let _ = Action::RowPageUp;
    let _ = Action::RowPageDown;
    let _ = Action::TablePageUp;
    let _ = Action::TablePageDown;
}

#[test]
fn test_export_action() {
    let _ = Action::ExportResultsToCsv;
}

#[test]
fn test_autocomplete_actions() {
    let _ = Action::TriggerAutocomplete;
    
    let results = vec![
        ("users".to_string(), "table".to_string()),
        ("user_id".to_string(), "column".to_string()),
    ];
    
    let action = Action::AutocompleteResults(results.clone());
    
    match action {
        Action::AutocompleteResults(r) => {
            assert_eq!(r.len(), 2);
            assert_eq!(r[0].0, "users");
            assert_eq!(r[0].1, "table");
        }
        _ => panic!("Wrong action type"),
    }
}

#[test]
fn test_action_equality() {
    // Test that actions can be compared
    assert_eq!(Action::Quit, Action::Quit);
    assert_ne!(Action::Quit, Action::TableMoveUp);
    
    assert_eq!(
        Action::Error("test".to_string()),
        Action::Error("test".to_string())
    );
    
    assert_ne!(
        Action::Error("test1".to_string()),
        Action::Error("test2".to_string())
    );
}