use std::sync::{Arc, Mutex};
use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tokio::sync::mpsc::{self, Receiver, Sender};
use color_eyre::Result;

pub trait TerminalEvent: Send + Sync {
    fn poll_event(&self, duration: Duration) -> Result<bool>;
    fn read_event(&self) -> Result<Event>;
}

pub struct MockTerminal {
    events: Arc<Mutex<Receiver<Event>>>,
}

impl MockTerminal {
    pub fn new(events: Receiver<Event>) -> Self {
        Self {
            events: Arc::new(Mutex::new(events)),
        }
    }
}

impl TerminalEvent for MockTerminal {
    fn poll_event(&self, _duration: Duration) -> Result<bool> {
        let events = self.events.lock().unwrap();
        Ok(!events.is_empty())
    }
    
    fn read_event(&self) -> Result<Event> {
        let mut events = self.events.lock().unwrap();
        match events.try_recv() {
            Ok(event) => Ok(event),
            Err(_) => Ok(Event::Key(KeyEvent {
                code: KeyCode::Null,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            })),
        }
    }
}

pub struct TestDriver {
    tx: Sender<Event>,
    // For capturing application state
    pub app_state: Arc<Mutex<TestAppState>>,
}

#[derive(Default, Debug, Clone)]
pub struct TestAppState {
    pub current_component: String,
    pub query_text: String,
    pub selected_table: Option<String>,
    pub results_count: usize,
    pub error_message: Option<String>,
    pub mode: String,
}

impl TestDriver {
    pub fn new() -> (Self, Receiver<Event>) {
        let (tx, rx) = mpsc::channel(100);
        let app_state = Arc::new(Mutex::new(TestAppState::default()));
        
        (Self { tx, app_state }, rx)
    }
    
    pub async fn send_key(&self, key: char) -> Result<()> {
        self.tx.send(Event::Key(KeyEvent {
            code: KeyCode::Char(key),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })).await?;
        Ok(())
    }
    
    pub async fn send_ctrl(&self, key: char) -> Result<()> {
        self.tx.send(Event::Key(KeyEvent {
            code: KeyCode::Char(key),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })).await?;
        Ok(())
    }
    
    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        for ch in keys.chars() {
            self.send_key(ch).await?;
            // Small delay to simulate typing
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }
    
    pub async fn send_special_key(&self, code: KeyCode) -> Result<()> {
        self.tx.send(Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        })).await?;
        Ok(())
    }
    
    pub fn get_state(&self) -> TestAppState {
        self.app_state.lock().unwrap().clone()
    }
    
    pub fn update_state<F>(&self, updater: F) 
    where 
        F: FnOnce(&mut TestAppState)
    {
        let mut state = self.app_state.lock().unwrap();
        updater(&mut state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_terminal_key_events() {
        let (driver, rx) = TestDriver::new();
        let terminal = MockTerminal::new(rx);
        
        // Send a key
        driver.send_key('a').await.unwrap();
        
        // Poll should return true
        assert!(terminal.poll_event(Duration::from_millis(10)).unwrap());
        
        // Read should return the key event
        if let Event::Key(key_event) = terminal.read_event().unwrap() {
            assert_eq!(key_event.code, KeyCode::Char('a'));
        } else {
            panic!("Expected key event");
        }
    }
    
    #[tokio::test]
    async fn test_driver_send_keys() {
        let (driver, mut rx) = TestDriver::new();
        
        driver.send_keys("hello").await.unwrap();
        
        let mut received = String::new();
        for _ in 0..5 {
            if let Some(Event::Key(key_event)) = rx.recv().await {
                if let KeyCode::Char(ch) = key_event.code {
                    received.push(ch);
                }
            }
        }
        
        assert_eq!(received, "hello");
    }
}