use crate::test_utils::{ComponentTestHarness, EventBuilder, TestEnvironment, fixtures};
use query_crafter::components::db::Db;
use query_crafter::action::Action;

#[tokio::test]
async fn test_table_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test tables
    let tables = fixtures::sample_tables();
    harness.update(Action::TablesLoaded(tables)).unwrap();
    
    // Navigate down through tables
    let events = EventBuilder::new()
        .key('j')
        .key('j')
        .key('k')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_table_jump_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test tables
    let tables = fixtures::sample_tables();
    harness.update(Action::TablesLoaded(tables)).unwrap();
    
    // Jump to bottom with Shift+G
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().shift('g').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Jump to top with gg
    let events = EventBuilder::new()
        .key('g')
        .key('g')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_results_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test results
    let (headers, results) = fixtures::sample_query_results();
    harness.update(Action::QueryResult(headers, results)).unwrap();
    
    // Focus on results
    harness.update(Action::FocusResults).unwrap();
    
    // Navigate through results
    let events = EventBuilder::new()
        .key('j')  // Down
        .key('j')
        .key('k')  // Up
        .key('l')  // Right (scroll)
        .key('h')  // Left (scroll)
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_results_jump_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load long results
    let (headers, results) = fixtures::long_query_results();
    harness.update(Action::QueryResult(headers, results)).unwrap();
    
    // Focus on results
    harness.update(Action::FocusResults).unwrap();
    
    // Test jump to bottom
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().shift('g').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test jump to top
    let events = EventBuilder::new()
        .key('g')
        .key('g')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_page_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load long results
    let (headers, results) = fixtures::long_query_results();
    harness.update(Action::QueryResult(headers, results)).unwrap();
    
    // Focus on results
    harness.update(Action::FocusResults).unwrap();
    
    // Test page down
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().ctrl('f').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test page up
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().ctrl('b').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
}

#[tokio::test]
async fn test_cell_selection_mode() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test results
    let (headers, results) = fixtures::sample_query_results();
    harness.update(Action::QueryResult(headers, results)).unwrap();
    
    // Focus on results
    harness.update(Action::FocusResults).unwrap();
    
    // Enter cell selection mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('v').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Navigate cells
    let events = EventBuilder::new()
        .key('l')  // Right
        .key('l')
        .key('j')  // Down
        .key('h')  // Left
        .key('k')  // Up
        .esc()     // Exit cell mode
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_preview_mode() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test results
    let (headers, results) = fixtures::sample_query_results();
    harness.update(Action::QueryResult(headers, results)).unwrap();
    
    // Focus on results
    harness.update(Action::FocusResults).unwrap();
    
    // Toggle preview with 'p'
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('p').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Navigate in preview
    let events = EventBuilder::new()
        .key('j')
        .key('k')
        .key('p')  // Toggle off
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_table_column_view() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test tables
    let tables = fixtures::sample_tables();
    harness.update(Action::TablesLoaded(tables)).unwrap();
    
    // View columns with 'c'
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('c').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Close with ESC
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().esc().build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
}

#[tokio::test]
async fn test_table_schema_view() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load test tables
    let tables = fixtures::sample_tables();
    harness.update(Action::TablesLoaded(tables)).unwrap();
    
    // View schema with 's'
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('s').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Navigate in schema view
    let events = EventBuilder::new()
        .key('j')
        .key('k')
        .esc()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}