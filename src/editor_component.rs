use std::any::Any;

use async_trait::async_trait;
use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;

use crate::action::Action;

#[async_trait]
pub trait EditorComponent: Any {
  fn init(&mut self, area: Rect) -> Result<()>;
  fn on_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>>;
  fn draw(&mut self, f: &mut ratatui::Frame, area: Rect);
  fn draw_with_focus(&mut self, f: &mut ratatui::Frame, area: Rect, _is_focused: bool) {
    // Default implementation calls regular draw
    self.draw(f, area);
  }
  fn get_text(&self) -> String;
  fn get_selected_text(&self) -> Option<String>;
  fn set_text(&mut self, text: &str);
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
}
