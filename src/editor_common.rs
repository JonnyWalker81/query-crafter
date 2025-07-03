use tui_textarea::Input;
use query_crafter_theme as theme;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  #[default]
  Normal,
  Insert,
  Visual,
  Operator(char),
}

impl Mode {
  pub fn cursor_style(&self) -> ratatui::style::Style {
    match self {
      Self::Normal => theme::cursor_normal(),
      Self::Insert => theme::cursor_insert(),
      Self::Visual => theme::cursor_visual(),
      Self::Operator(_) => theme::cursor_normal(),
    }
  }
}

impl std::fmt::Display for Mode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    match self {
      Self::Normal => write!(f, "NORMAL"),
      Self::Insert => write!(f, "INSERT"),
      Self::Visual => write!(f, "VISUAL"),
      Self::Operator(c) => write!(f, "OPERATOR({c})"),
    }
  }
}

#[derive(Debug)]
pub enum Transition {
  Nop,
  Mode(Mode),
  Pending(Input),
  Quit,
  Action(crate::action::Action),
}
