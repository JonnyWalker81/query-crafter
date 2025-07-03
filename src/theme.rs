use ratatui::style::{Color, Modifier, Style};

/// Modern One Dark Pro theme inspired by popular Rust TUI applications
/// This now delegates to theme_const for better compilation performance
pub struct Theme;

impl Theme {
  // Re-export color constants from theme_const
  pub const ACCENT_BLUE: Color = crate::theme_const::ACCENT_BLUE;
  pub const ACCENT_CYAN: Color = crate::theme_const::ACCENT_CYAN;
  pub const ACCENT_GREEN: Color = crate::theme_const::ACCENT_GREEN;
  pub const ACCENT_ORANGE: Color = crate::theme_const::ACCENT_ORANGE;
  pub const ACCENT_PURPLE: Color = crate::theme_const::ACCENT_PURPLE;
  pub const ACCENT_RED: Color = crate::theme_const::ACCENT_RED;
  pub const BG_CURSOR: Color = crate::theme_const::BG_CURSOR;
  pub const BG_PRIMARY: Color = crate::theme_const::BG_PRIMARY;
  pub const BG_SECONDARY: Color = crate::theme_const::BG_SECONDARY;
  pub const BG_SELECTION: Color = crate::theme_const::BG_SELECTION;
  pub const BG_TERTIARY: Color = crate::theme_const::BG_TERTIARY;
  pub const BORDER_FOCUSED: Color = crate::theme_const::BORDER_FOCUSED;
  pub const BORDER_MUTED: Color = crate::theme_const::BORDER_MUTED;
  pub const BORDER_NORMAL: Color = crate::theme_const::BORDER_NORMAL;
  pub const ERROR: Color = crate::theme_const::ERROR;
  pub const FG_PRIMARY: Color = crate::theme_const::FG_PRIMARY;
  pub const FG_SECONDARY: Color = crate::theme_const::FG_SECONDARY;
  pub const FG_TERTIARY: Color = crate::theme_const::FG_TERTIARY;
  pub const INFO: Color = crate::theme_const::INFO;
  pub const SUCCESS: Color = crate::theme_const::SUCCESS;
  pub const WARNING: Color = crate::theme_const::WARNING;

  // Delegate all style methods to theme_const
  #[inline(always)]
  pub fn bg_primary() -> Style { crate::theme_const::bg_primary() }

  #[inline(always)]
  pub fn bg_secondary() -> Style { crate::theme_const::bg_secondary() }

  #[inline(always)]
  pub fn input() -> Style { crate::theme_const::input() }

  #[inline(always)]
  pub fn selection() -> Style { crate::theme_const::selection() }

  #[inline(always)]
  pub fn selection_active() -> Style { crate::theme_const::selection_active() }

  #[inline(always)]
  pub fn border_normal() -> Style { crate::theme_const::border_normal() }

  #[inline(always)]
  pub fn border_focused() -> Style { crate::theme_const::border_focused() }

  #[inline(always)]
  pub fn header() -> Style { crate::theme_const::header() }

  #[inline(always)]
  pub fn title() -> Style { crate::theme_const::title() }

  #[inline(always)]
  pub fn success() -> Style { crate::theme_const::success() }

  #[inline(always)]
  pub fn warning() -> Style { crate::theme_const::warning() }

  #[inline(always)]
  pub fn error() -> Style { crate::theme_const::error() }

  #[inline(always)]
  pub fn info() -> Style { crate::theme_const::info() }

  #[inline(always)]
  pub fn muted() -> Style { crate::theme_const::muted() }

  #[inline(always)]
  pub fn line_numbers() -> Style { crate::theme_const::line_numbers() }

  #[inline(always)]
  pub fn tab_normal() -> Style { crate::theme_const::tab_normal() }

  #[inline(always)]
  pub fn tab_selected() -> Style { crate::theme_const::tab_selected() }

  #[inline(always)]
  pub fn status_bar() -> Style { crate::theme_const::status_bar() }

  #[inline(always)]
  pub fn cursor_normal() -> Style { crate::theme_const::cursor_normal() }

  #[inline(always)]
  pub fn cursor_insert() -> Style { crate::theme_const::cursor_insert() }

  #[inline(always)]
  pub fn cursor_visual() -> Style { crate::theme_const::cursor_visual() }

  #[inline(always)]
  pub fn sql_keyword() -> Style { crate::theme_const::sql_keyword() }

  #[inline(always)]
  pub fn sql_function() -> Style { crate::theme_const::sql_function() }

  #[inline(always)]
  pub fn sql_string() -> Style { crate::theme_const::sql_string() }

  #[inline(always)]
  pub fn sql_number() -> Style { crate::theme_const::sql_number() }

  #[inline(always)]
  pub fn sql_comment() -> Style { crate::theme_const::sql_comment() }

  #[inline(always)]
  pub fn sql_operator() -> Style { crate::theme_const::sql_operator() }
}

/// Modern color palette - delegates to theme_const
pub struct Colors;

impl Colors {
  pub const DANGER: Color = crate::theme_const::ERROR;
  pub const GRAY_100: Color = Color::Rgb(248, 249, 250);
  pub const GRAY_200: Color = Color::Rgb(233, 236, 239);
  pub const GRAY_300: Color = Color::Rgb(206, 212, 218);
  pub const GRAY_400: Color = Color::Rgb(173, 181, 189);
  pub const GRAY_500: Color = Color::Rgb(108, 117, 125);
  pub const GRAY_600: Color = Color::Rgb(92, 99, 112);
  pub const GRAY_700: Color = Color::Rgb(73, 80, 87);
  pub const GRAY_800: Color = Color::Rgb(52, 58, 64);
  pub const GRAY_900: Color = Color::Rgb(33, 37, 41);
  pub const INFO: Color = Color::Rgb(86, 182, 194);
  pub const ONE_DARK_BG: Color = Color::Rgb(40, 44, 52);
  pub const ONE_DARK_FG: Color = Color::Rgb(171, 178, 191);
  pub const PRIMARY: Color = Color::Rgb(97, 175, 239);
  pub const SECONDARY: Color = Color::Rgb(198, 120, 221);
  pub const SUCCESS: Color = Color::Rgb(152, 195, 121);
  pub const WARNING: Color = Color::Rgb(229, 192, 123);
}

// Convenience functions delegate to theme_const
pub mod style {
  use super::*;

  #[inline(always)]
  pub fn button(active: bool) -> Style {
    if active {
      crate::theme_const::selection_active()
    } else {
      Style::default().bg(Theme::BG_SECONDARY).fg(Theme::FG_PRIMARY).add_modifier(Modifier::BOLD)
    }
  }

  #[inline(always)]
  pub fn list_item(selected: bool) -> Style {
    if selected {
      crate::theme_const::selection_active()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  #[inline(always)]
  pub fn table_row(selected: bool) -> Style {
    if selected {
      crate::theme_const::selection()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  #[inline(always)]
  pub fn border(focused: bool) -> Style {
    if focused {
      crate::theme_const::border_focused()
    } else {
      crate::theme_const::border_normal()
    }
  }
}