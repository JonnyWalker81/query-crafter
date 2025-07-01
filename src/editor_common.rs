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
    let color = match self {
      Self::Normal => ratatui::style::Color::Reset,
      Self::Insert => ratatui::style::Color::LightBlue,
      Self::Visual => ratatui::style::Color::LightYellow,
      Self::Operator(_) => ratatui::style::Color::LightGreen,
    };
    ratatui::style::Style::default().fg(color).add_modifier(ratatui::style::Modifier::REVERSED)
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
}
