use query_crafter::action::Action;
use query_crafter::components::db::DbColumn;

#[test]
fn test_view_table_columns_action() {
    // Test that ViewTableColumns action exists and can be created
    let action = Action::ViewTableColumns;
    match action {
        Action::ViewTableColumns => {
            // Success - action exists
        }
        _ => panic!("ViewTableColumns action not matched correctly"),
    }
}

#[test]
fn test_view_table_schema_action() {
    // Test that ViewTableSchema action exists and can be created
    let action = Action::ViewTableSchema;
    match action {
        Action::ViewTableSchema => {
            // Success - action exists
        }
        _ => panic!("ViewTableSchema action not matched correctly"),
    }
}

#[test]
fn test_table_columns_loaded_action() {
    // Test that TableColumnsLoaded action exists and can store data
    let columns = vec![
        DbColumn {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            is_nullable: false,
        },
        DbColumn {
            name: "name".to_string(),
            data_type: "TEXT".to_string(),
            is_nullable: true,
        },
    ];
    
    let action = Action::TableColumnsLoaded("test_table".to_string(), columns.clone());
    match action {
        Action::TableColumnsLoaded(table_name, loaded_columns) => {
            assert_eq!(table_name, "test_table");
            assert_eq!(loaded_columns.len(), 2);
            assert_eq!(loaded_columns[0].name, "id");
            assert_eq!(loaded_columns[1].name, "name");
        }
        _ => panic!("TableColumnsLoaded action not matched correctly"),
    }
}