use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::*;

use crate::{
    action::Action,
    components::vim::Vim,
    editor_common::Mode,
    editor_component::EditorComponent,
};

#[derive(Debug)]
pub enum EditorBackend {
    TuiTextarea(Vim),
}

impl Default for EditorBackend {
    fn default() -> Self {
        Self::TuiTextarea(Vim::new(Mode::Normal))
    }
}

impl EditorBackend {
    pub fn new_from_config(_backend_name: &str) -> Self {
        // For now, always use TuiTextarea
        // In the future, match on backend_name to support different editors
        Self::TuiTextarea(Vim::new(Mode::Normal))
    }

    pub fn as_editor_component(&mut self) -> &mut dyn EditorComponent {
        match self {
            Self::TuiTextarea(vim) => vim,
        }
    }

    pub fn get_text(&self) -> String {
        match self {
            Self::TuiTextarea(vim) => vim.get_text(),
        }
    }

    pub fn get_selected_text(&self) -> Option<String> {
        match self {
            Self::TuiTextarea(vim) => vim.get_selected_text(),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        match self {
            Self::TuiTextarea(vim) => vim.set_text(text),
        }
    }

    pub fn get_cursor_position(&self) -> (usize, usize) {
        match self {
            Self::TuiTextarea(vim) => vim.get_cursor_position(),
        }
    }

    pub fn get_text_up_to_cursor(&self) -> String {
        match self {
            Self::TuiTextarea(vim) => vim.get_text_up_to_cursor(),
        }
    }

    pub fn insert_text_at_cursor(&mut self, text: &str) {
        match self {
            Self::TuiTextarea(vim) => vim.insert_text_at_cursor(text),
        }
    }

    pub fn mode(&self) -> Mode {
        match self {
            Self::TuiTextarea(vim) => vim.mode(),
        }
    }

    pub fn is_auto_format_enabled(&self) -> bool {
        match self {
            Self::TuiTextarea(vim) => vim.is_auto_format_enabled(),
        }
    }

    pub fn set_auto_format(&mut self, _enabled: bool) {
        // TODO: Implement set_auto_format for Vim
    }

    pub fn format_all(&mut self) -> Result<()> {
        match self {
            Self::TuiTextarea(vim) => vim.format_all().map_err(|e| color_eyre::eyre::eyre!(e)),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        match self {
            Self::TuiTextarea(vim) => {
                vim.draw(f, area);
                Ok(())
            },
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match self {
            Self::TuiTextarea(vim) => vim.on_key_event(key),
        }
    }

    pub fn draw_with_focus(&mut self, f: &mut Frame, area: Rect, is_focused: bool) {
        match self {
            Self::TuiTextarea(vim) => vim.draw_with_focus(f, area, is_focused),
        }
    }

    pub fn init(&mut self, area: Rect) -> Result<()> {
        match self {
            Self::TuiTextarea(vim) => vim.init(area),
        }
    }

    pub fn move_to_start(&mut self) {
        // TODO: Implement move_to_start for Vim
    }

    pub fn move_to_end(&mut self) {
        // TODO: Implement move_to_end for Vim
    }
    
    pub fn delete_word_before_cursor(&mut self) {
        match self {
            Self::TuiTextarea(vim) => vim.delete_word_before_cursor(),
        }
    }
    
    pub fn delete_char_before_cursor(&mut self) {
        // TODO: Implement delete_char_before_cursor for Vim
        // For now, use delete_word_before_cursor as a workaround
        self.delete_word_before_cursor();
    }
    
    pub fn toggle_auto_format(&mut self) {
        match self {
            Self::TuiTextarea(vim) => vim.toggle_auto_format(),
        }
    }
    
    pub fn format_query(&mut self, selection_only: bool) -> Result<String> {
        match self {
            Self::TuiTextarea(vim) => vim.format_query(selection_only)
                .map(|_| String::new())
                .map_err(|e| color_eyre::eyre::eyre!(e)),
        }
    }
}