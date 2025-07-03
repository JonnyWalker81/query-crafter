use ratatui::style::{Color, Modifier, Style};

/// Modern One Dark Pro theme inspired by popular Rust TUI applications
/// This now delegates to an external crate for zero rebuild impact
pub struct Theme;

impl Theme {
  // Re-export color constants from external theme crate
  pub const ACCENT_BLUE: Color = query_crafter_theme::ACCENT_BLUE;
  pub const ACCENT_CYAN: Color = query_crafter_theme::ACCENT_CYAN;
  pub const ACCENT_GREEN: Color = query_crafter_theme::ACCENT_GREEN;
  pub const ACCENT_ORANGE: Color = query_crafter_theme::ACCENT_ORANGE;
  pub const ACCENT_PURPLE: Color = query_crafter_theme::ACCENT_PURPLE;
  pub const ACCENT_RED: Color = query_crafter_theme::ACCENT_RED;
  pub const BG_CURSOR: Color = query_crafter_theme::BG_CURSOR;
  pub const BG_PRIMARY: Color = query_crafter_theme::BG_PRIMARY;
  pub const BG_SECONDARY: Color = query_crafter_theme::BG_SECONDARY;
  pub const BG_SELECTION: Color = query_crafter_theme::BG_SELECTION;
  pub const BG_TERTIARY: Color = query_crafter_theme::BG_TERTIARY;
  pub const BORDER_FOCUSED: Color = query_crafter_theme::BORDER_FOCUSED;
  pub const BORDER_MUTED: Color = query_crafter_theme::BORDER_MUTED;
  pub const BORDER_NORMAL: Color = query_crafter_theme::BORDER_NORMAL;
  pub const ERROR: Color = query_crafter_theme::ERROR;
  pub const FG_PRIMARY: Color = query_crafter_theme::FG_PRIMARY;
  pub const FG_SECONDARY: Color = query_crafter_theme::FG_SECONDARY;
  pub const FG_TERTIARY: Color = query_crafter_theme::FG_TERTIARY;
  pub const INFO: Color = query_crafter_theme::INFO;
  pub const SUCCESS: Color = query_crafter_theme::SUCCESS;
  pub const WARNING: Color = query_crafter_theme::WARNING;

  // Delegate all style methods to theme_const
  #[inline(always)]
  pub fn bg_primary() -> Style { query_crafter_theme::bg_primary() }

  #[inline(always)]
  pub fn bg_secondary() -> Style { query_crafter_theme::bg_secondary() }

  #[inline(always)]
  pub fn input() -> Style { query_crafter_theme::input() }

  #[inline(always)]
  pub fn selection() -> Style { query_crafter_theme::selection() }

  #[inline(always)]
  pub fn selection_active() -> Style { query_crafter_theme::selection_active() }

  #[inline(always)]
  pub fn border_normal() -> Style { query_crafter_theme::border_normal() }

  #[inline(always)]
  pub fn border_focused() -> Style { query_crafter_theme::border_focused() }

  #[inline(always)]
  pub fn header() -> Style { query_crafter_theme::header() }

  #[inline(always)]
  pub fn title() -> Style { query_crafter_theme::title() }

  #[inline(always)]
  pub fn success() -> Style { query_crafter_theme::success() }

  #[inline(always)]
  pub fn warning() -> Style { query_crafter_theme::warning() }

  #[inline(always)]
  pub fn error() -> Style { query_crafter_theme::error() }

  #[inline(always)]
  pub fn info() -> Style { query_crafter_theme::info() }

  #[inline(always)]
  pub fn muted() -> Style { query_crafter_theme::muted() }

  #[inline(always)]
  pub fn line_numbers() -> Style { query_crafter_theme::line_numbers() }

  #[inline(always)]
  pub fn tab_normal() -> Style { query_crafter_theme::tab_normal() }

  #[inline(always)]
  pub fn tab_selected() -> Style { query_crafter_theme::tab_selected() }

  #[inline(always)]
  pub fn status_bar() -> Style { query_crafter_theme::status_bar() }

  #[inline(always)]
  pub fn cursor_normal() -> Style { query_crafter_theme::cursor_normal() }

  #[inline(always)]
  pub fn cursor_insert() -> Style { query_crafter_theme::cursor_insert() }

  #[inline(always)]
  pub fn cursor_visual() -> Style { query_crafter_theme::cursor_visual() }

  #[inline(always)]
  pub fn sql_keyword() -> Style { query_crafter_theme::sql_keyword() }

  #[inline(always)]
  pub fn sql_function() -> Style { query_crafter_theme::sql_function() }

  #[inline(always)]
  pub fn sql_string() -> Style { query_crafter_theme::sql_string() }

  #[inline(always)]
  pub fn sql_number() -> Style { query_crafter_theme::sql_number() }

  #[inline(always)]
  pub fn sql_comment() -> Style { query_crafter_theme::sql_comment() }

  #[inline(always)]
  pub fn sql_operator() -> Style { query_crafter_theme::sql_operator() }
}

/// Modern color palette - delegates to theme_const
pub struct Colors;

impl Colors {
  pub const DANGER: Color = query_crafter_theme::ERROR;
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
      query_crafter_theme::selection_active()
    } else {
      Style::default().bg(Theme::BG_SECONDARY).fg(Theme::FG_PRIMARY).add_modifier(Modifier::BOLD)
    }
  }

  #[inline(always)]
  pub fn list_item(selected: bool) -> Style {
    if selected {
      query_crafter_theme::selection_active()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  #[inline(always)]
  pub fn table_row(selected: bool) -> Style {
    if selected {
      query_crafter_theme::selection()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  #[inline(always)]
  pub fn border(focused: bool) -> Style {
    if focused {
      query_crafter_theme::border_focused()
    } else {
      query_crafter_theme::border_normal()
    }
  }
}