use tui_textarea::Input;

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
      Self::Normal => crate::theme::Theme::cursor_normal(),
      Self::Insert => crate::theme::Theme::cursor_insert(),
      Self::Visual => crate::theme::Theme::cursor_visual(),
      Self::Operator(_) => crate::theme::Theme::cursor_normal(),
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
