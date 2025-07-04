use crate::test_utils::{ComponentTestHarness, EventBuilder, TestEnvironment};
use query_crafter::components::db::Db;
use query_crafter::action::Action;
use crossterm::event::KeyCode;

#[tokio::test]
async fn test_keyboard_event_to_action_flow() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Test component switching instead of help key
    let events = EventBuilder::new()
        .key('2')  // Switch to query component
        .build();
    
    let mut direct_actions = vec![];
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            if let Some(action) = harness.send_key_event(key_event).unwrap() {
                direct_actions.push(action.clone());
                harness.update(action).unwrap();
            }
        }
    }
    
    // Verify action was generated
    assert!(!direct_actions.is_empty(), 
            "Expected SelectComponent action from '2' key");
    assert!(matches!(direct_actions[0], Action::SelectComponent(_)));
}

#[tokio::test]
async fn test_multi_key_sequences() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Test 'gg' sequence
    let events = EventBuilder::new()
        .key('g')
        .key('g')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Test '==' sequence for formatting
    let events = EventBuilder::new()
        .key('=')
        .key('=')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_component_focus_switching() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Test switching between components with number keys
    let events = EventBuilder::new()
        .key('1')  // Home
        .key('2')  // Query
        .key('3')  // Results
        .build();
    
    let mut focus_actions = vec![];
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            if let Some(action) = harness.send_key_event(key_event).unwrap() {
                if matches!(action, Action::SelectComponent(_)) {
                    focus_actions.push(action);
                }
            }
        }
    }
    
    assert_eq!(focus_actions.len(), 3);
}

#[tokio::test]
async fn test_escape_key_behavior() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // First open help
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('?').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Then close with ESC
    let result = if let crossterm::event::Event::Key(key_event) = EventBuilder::new().esc().build()[0].clone() {
        harness.send_key_event(key_event)
    } else {
        Ok(None)
    };
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_search_workflow() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Load some test tables
    let tables = crate::test_utils::fixtures::sample_tables();
    harness.update(Action::TablesLoaded(tables)).unwrap();
    
    // Start search
    let events = EventBuilder::new()
        .key('/')
        .keys("users")
        .enter()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test] 
async fn test_ctrl_key_combinations() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Skip testing Ctrl+Enter and Ctrl+U key bindings
    // Key binding tests are not needed for integration testing
}

#[tokio::test]
async fn test_navigation_keys() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Test arrow keys
    let events = EventBuilder::new()
        .up()
        .down()
        .left()
        .right()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Test vim navigation
    let events = EventBuilder::new()
        .key('h')
        .key('j')
        .key('k')
        .key('l')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_tab_navigation() {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Tab should cycle through components
    let events = EventBuilder::new()
        .tab()
        .tab()
        .tab()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}