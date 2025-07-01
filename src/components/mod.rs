pub mod custom_vim_editor;
pub mod db;
pub mod fps;
pub mod home;
pub mod vim;

use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, config::Config, tui};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentKind {
  #[default]
  Home,
  Query,
  Results,
}

pub trait Component {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()>;
  fn register_config_handler(&mut self, config: Config) -> Result<()>;
  fn update(&mut self, action: Action) -> Result<Option<Action>>;
  fn handle_events(&mut self, event: Option<tui::Event>) -> Result<Option<Action>>;
  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>>;
  fn init(&mut self, area: Rect) -> Result<()>;
  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()>;
}
