use crate::test_utils::{ComponentTestHarness, EventBuilder, TestEnvironment};
use query_crafter::components::db::Db;
use query_crafter::components::vim::Vim;
use query_crafter::editor_common::Mode;
use query_crafter::action::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

async fn setup_db_with_query(query: &str) -> ComponentTestHarness<Db> {
    let env = TestEnvironment::new().await.unwrap();
    let db = Db::new();
    let mut harness = ComponentTestHarness::new(db).unwrap()
        .with_config(env.config).unwrap();
    
    // Focus on query editor
    harness.update(Action::FocusQuery).unwrap();
    
    // Enter insert mode and type query
    let events = EventBuilder::new()
        .key('i')
        .keys(query)
        .esc()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    harness
}

#[tokio::test]
async fn test_vim_mode_transitions() {
    let mut harness = setup_db_with_query("SELECT * FROM users").await;
    
    // Test i -> Insert mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('i').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test ESC -> Normal mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().esc().build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test v -> Visual mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('v').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test ESC -> Normal mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().esc().build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
}

#[tokio::test]
async fn test_vim_dd_delete_line() {
    let mut harness = setup_db_with_query("line 1\nline 2\nline 3").await;
    
    // Move to second line and delete it
    let events = EventBuilder::new()
        .key('j')  // Move down
        .key('d')
        .key('d')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Verify line was deleted by checking buffer content
    let buffer = harness.get_buffer_content();
    let content = buffer.join("\n");
    assert!(!content.contains("line 2"));
}

#[tokio::test]
async fn test_vim_yy_yank_and_paste() {
    let mut harness = setup_db_with_query("line 1\nline 2").await;
    
    // Yank first line and paste at end
    let events = EventBuilder::new()
        .key('y')
        .key('y')
        .key('G')  // Go to end
        .key('p')  // Paste
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_vim_visual_selection() {
    let mut harness = setup_db_with_query("SELECT * FROM users").await;
    
    // Enter visual mode and select some text
    let events = EventBuilder::new()
        .key('v')
        .key('l')
        .key('l')
        .key('l')
        .key('l')
        .key('l')
        .key('l')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_vim_format_operations() {
    let mut harness = setup_db_with_query("select * from users where id=1").await;
    
    // Ensure we're in normal mode and focused on query editor
    harness.update(Action::FocusQuery).unwrap();
    
    // Test == to format entire query
    let events = EventBuilder::new()
        .key('=')
        .key('=')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Render to update buffer
    harness.render().unwrap();
    
    // Verify formatting happened
    let buffer = harness.get_buffer_content();
    let content = buffer.join("\n");
    assert!(content.contains("SELECT") || content.contains("FROM"));
}

#[tokio::test]
async fn test_vim_gg_and_G_navigation() {
    let mut harness = setup_db_with_query("line 1\nline 2\nline 3\nline 4").await;
    
    // Go to bottom with G
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().shift('g').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Go to top with gg
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
async fn test_vim_word_navigation() {
    let mut harness = setup_db_with_query("SELECT * FROM users WHERE id = 1").await;
    
    // Test word forward with w
    let events = EventBuilder::new()
        .key('w')
        .key('w')
        .key('w')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Test word backward with b
    let events = EventBuilder::new()
        .key('b')
        .key('b')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_vim_change_operations() {
    let mut harness = setup_db_with_query("SELECT * FROM users").await;
    
    // Test cc to change line
    let events = EventBuilder::new()
        .key('c')
        .key('c')
        .keys("SELECT id, name FROM users")
        .esc()
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_vim_auto_format_toggle() {
    let mut harness = setup_db_with_query("select * from users").await;
    
    // Toggle auto-format with =a
    let events = EventBuilder::new()
        .key('=')
        .key('a')
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}

#[tokio::test]
async fn test_vim_visual_mode_format() {
    let mut harness = setup_db_with_query("select * from users\nselect * from posts").await;
    
    // Ensure we're in normal mode and focused on query editor
    harness.update(Action::FocusQuery).unwrap();
    
    // Select first line in visual mode and format
    let events = EventBuilder::new()
        .key('V')  // Visual line mode
        .key('=')  // Format selection
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
    
    // Render to update buffer
    harness.render().unwrap();
    
    // Verify formatting happened on first line
    let buffer = harness.get_buffer_content();
    let content = buffer.join("\n");
    assert!(content.contains("SELECT"));
}

#[tokio::test]
async fn test_vim_insert_mode_shortcuts() {
    let mut harness = setup_db_with_query("").await;
    
    // Enter insert mode
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().key('i').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Test Ctrl+U to clear
    if let crossterm::event::Event::Key(key_event) = EventBuilder::new().ctrl('u').build()[0].clone() {
        harness.send_key_event(key_event).unwrap();
    }
    
    // Type some text
    let events = EventBuilder::new()
        .keys("SELECT * FROM users")
        .build();
    
    for event in events {
        if let crossterm::event::Event::Key(key_event) = event {
            harness.send_key_event(key_event).unwrap();
        }
    }
}