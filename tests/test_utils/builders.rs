use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use query_crafter::action::Action;

pub struct EventBuilder {
    events: Vec<Event>,
}

impl EventBuilder {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
    
    pub fn key(mut self, key: char) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Char(key),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn ctrl(mut self, key: char) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Char(key),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn shift(mut self, key: char) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Char(key.to_ascii_uppercase()),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn keys(mut self, keys: &str) -> Self {
        for ch in keys.chars() {
            self = self.key(ch);
        }
        self
    }
    
    pub fn enter(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn esc(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn up(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn down(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn left(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn right(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn tab(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn backspace(mut self) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
        self
    }
    
    pub fn build(self) -> Vec<Event> {
        self.events
    }
}

pub struct ActionBuilder;

impl ActionBuilder {
    pub fn table_move_up() -> Action {
        Action::TableMoveUp
    }
    
    pub fn table_move_down() -> Action {
        Action::TableMoveDown
    }
    
    pub fn row_move_up() -> Action {
        Action::RowMoveUp
    }
    
    pub fn row_move_down() -> Action {
        Action::RowMoveDown
    }
    
    pub fn execute_query() -> Action {
        Action::ExecuteQuery
    }
    
    pub fn format_query() -> Action {
        Action::FormatQuery
    }
    
    pub fn handle_query(query: String) -> Action {
        Action::HandleQuery(query)
    }
    
    pub fn query_result(headers: Vec<String>, results: Vec<Vec<String>>) -> Action {
        Action::QueryResult(headers, results)
    }
    
    pub fn error(msg: String) -> Action {
        Action::Error(msg)
    }
    
    pub fn tables_loaded(tables: Vec<query_crafter::components::db::DbTable>) -> Action {
        Action::TablesLoaded(tables)
    }
    
    pub fn clear_query() -> Action {
        Action::ClearQuery
    }
    
    pub fn focus_query() -> Action {
        Action::FocusQuery
    }
    
    pub fn focus_results() -> Action {
        Action::FocusResults
    }
    
    pub fn focus_home() -> Action {
        Action::FocusHome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_builder() {
        let events = EventBuilder::new()
            .keys("hello")
            .ctrl('a')
            .enter()
            .build();
        
        assert_eq!(events.len(), 7); // 5 chars + ctrl+a + enter
        
        if let Event::Key(key) = &events[0] {
            assert_eq!(key.code, KeyCode::Char('h'));
        }
    }
    
    #[test]
    fn test_multi_key_sequences() {
        let events = EventBuilder::new()
            .key('g')
            .key('g')
            .build();
        
        assert_eq!(events.len(), 2);
        for event in &events {
            if let Event::Key(key) = event {
                assert_eq!(key.code, KeyCode::Char('g'));
            }
        }
    }
}