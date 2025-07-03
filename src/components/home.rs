use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, Frame};
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Home {
  command_tx: Option<UnboundedSender<Action>>,
  config: Config,
}

impl Home {
  pub fn new() -> Self {
    Self::default()
  }
}

impl Component for Home {
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    self.command_tx = Some(tx);
    Ok(())
  }

  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config;
    Ok(())
  }

  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    let _ = action == Action::Tick;
    Ok(None)
  }

  fn init(&mut self, _area: ratatui::layout::Rect) -> Result<()> {
    Ok(())
  }

  fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
    if let Some(crate::tui::Event::Key(key)) = event {
      self.handle_key_events(key)
    } else {
      Ok(None)
    }
  }

  fn handle_key_events(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    f.render_widget(Paragraph::new("hello world"), area);
    Ok(())
  }
}
