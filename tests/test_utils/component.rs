use color_eyre::Result;
use ratatui::{
    backend::TestBackend,
    Terminal,
    layout::Rect,
};
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};

use query_crafter::{
    action::Action,
    components::Component,
    config::Config,
    tui::Event,
};

use super::{TEST_TERMINAL_WIDTH, TEST_TERMINAL_HEIGHT};

pub struct ComponentTestHarness<C: Component> {
    pub component: C,
    pub backend: TestBackend,
    pub terminal: Terminal<TestBackend>,
    pub action_tx: UnboundedSender<Action>,
    pub action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
}

impl<C: Component> ComponentTestHarness<C> {
    pub fn new(mut component: C) -> Result<Self> {
        let backend = TestBackend::new(TEST_TERMINAL_WIDTH, TEST_TERMINAL_HEIGHT);
        let terminal = Terminal::new(backend)?;
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        
        // Register action handler
        component.register_action_handler(action_tx.clone())?;
        
        // Initialize component with terminal area
        let area = Rect::new(0, 0, TEST_TERMINAL_WIDTH, TEST_TERMINAL_HEIGHT);
        component.init(area)?;
        
        Ok(Self {
            component,
            backend: TestBackend::new(TEST_TERMINAL_WIDTH, TEST_TERMINAL_HEIGHT),
            terminal,
            action_tx,
            action_rx,
        })
    }
    
    pub fn with_config(mut self, config: Config) -> Result<Self> {
        self.component.register_config_handler(config)?;
        Ok(self)
    }
    
    pub fn render(&mut self) -> Result<String> {
        self.terminal.draw(|f| {
            self.component.draw(f, f.area()).unwrap();
        })?;
        
        let buffer = self.terminal.backend().buffer();
        let mut output = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                output.push_str(cell.symbol());
            }
            output.push('\n');
        }
        Ok(output.trim_end().to_string())
    }
    
    pub fn send_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        self.component.handle_key_events(key)
    }
    
    pub fn send_event(&mut self, event: Event) -> Result<Option<Action>> {
        self.component.handle_events(Some(event))
    }
    
    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        self.component.update(action)
    }
    
    pub async fn collect_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Ok(action) = self.action_rx.try_recv() {
            actions.push(action);
        }
        actions
    }
    
    pub fn get_buffer_content(&self) -> Vec<String> {
        let buffer = self.terminal.backend().buffer();
        let mut lines = Vec::new();
        
        for y in 0..TEST_TERMINAL_HEIGHT {
            let mut line = String::new();
            for x in 0..TEST_TERMINAL_WIDTH {
                let cell = &buffer[(x, y)];
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        
        lines
    }
}

pub struct TestEnvironment {
    pub config: Config,
    pub db: Arc<dyn query_crafter::sql::Queryer>,
}

impl TestEnvironment {
    pub async fn new() -> Result<Self> {
        use tempfile::NamedTempFile;
        use query_crafter::sql::Sqlite;
        
        // Create a temporary SQLite database
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap().to_string();
        let db = Arc::new(Sqlite::new(&db_path).await?);
        
        let config = Config {
            config: Default::default(),
            editor: query_crafter::config::EditorConfig {
                backend: "tui-textarea".to_string(),
            },
            keybindings: Default::default(),
            styles: Default::default(),
            autocomplete: Default::default(),
            lsp: Default::default(),
        };
        
        Ok(Self { config, db })
    }
    
    pub async fn setup_test_tables(&self) -> Result<()> {
        // Since Queryer trait doesn't have execute method,
        // tests will need to use the query method with appropriate SQL
        // or mock the responses
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        assert_eq!(env.config.editor.backend, "tui-textarea");
    }
}