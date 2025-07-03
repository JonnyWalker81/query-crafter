use ratatui::style::{Color, Modifier, Style};

/// Modern One Dark Pro theme inspired by popular Rust TUI applications
/// like Helix, gitui, kubetui, and bottom
pub struct Theme;

impl Theme {
  // #8b949e - Line numbers

  // Accent colors
  pub const ACCENT_BLUE: Color = Color::Rgb(97, 175, 239);
  // #d19a66 - Numbers, warnings
  pub const ACCENT_CYAN: Color = Color::Rgb(86, 182, 194);
  // #c678dd - Keywords, types
  pub const ACCENT_GREEN: Color = Color::Rgb(152, 195, 121);
  // #e06c75 - Errors, deletion
  pub const ACCENT_ORANGE: Color = Color::Rgb(209, 154, 102);
  // #61afef - Functions, keywords
  pub const ACCENT_PURPLE: Color = Color::Rgb(198, 120, 221);
  // #98c379 - Strings, success
  pub const ACCENT_RED: Color = Color::Rgb(224, 108, 117);
  // #3e4451 - Selection background
  pub const BG_CURSOR: Color = Color::Rgb(82, 89, 102);
  // Background colors
  pub const BG_PRIMARY: Color = Color::Rgb(40, 44, 52);
  // #282c34 - Main background
  pub const BG_SECONDARY: Color = Color::Rgb(33, 37, 43);
  // #2c313a - Input backgrounds
  pub const BG_SELECTION: Color = Color::Rgb(62, 68, 81);
  // #21252b - Secondary panels
  pub const BG_TERTIARY: Color = Color::Rgb(44, 49, 58);
  // #5c6370 - Normal borders
  pub const BORDER_FOCUSED: Color = Color::Rgb(97, 175, 239);
  // #61afef - Focused borders
  pub const BORDER_MUTED: Color = Color::Rgb(62, 68, 81);
  // #61afef - Info states

  // Border colors
  pub const BORDER_NORMAL: Color = Color::Rgb(92, 99, 112);
  // #e5c07b - Warning states
  pub const ERROR: Color = Color::Rgb(224, 108, 117);
  // #528bff - Cursor background

  // Foreground colors
  pub const FG_PRIMARY: Color = Color::Rgb(171, 178, 191);
  // #abb2bf - Main text
  pub const FG_SECONDARY: Color = Color::Rgb(92, 99, 112);
  // #5c6370 - Muted text/comments
  pub const FG_TERTIARY: Color = Color::Rgb(139, 148, 158);
  // #e06c75 - Error states
  pub const INFO: Color = Color::Rgb(97, 175, 239);
  // #56b6c2 - Operators, info

  // Status colors
  pub const SUCCESS: Color = Color::Rgb(152, 195, 121);
  // #98c379 - Success states
  pub const WARNING: Color = Color::Rgb(229, 192, 123);

  // #3e4451 - Subtle borders

  // Component-specific styles

  /// Main application background
  pub fn bg_primary() -> Style {
    Style::default().bg(Self::BG_PRIMARY).fg(Self::FG_PRIMARY)
  }

  /// Secondary panel background
  pub fn bg_secondary() -> Style {
    Style::default().bg(Self::BG_SECONDARY).fg(Self::FG_PRIMARY)
  }

  /// Input field styling
  pub fn input() -> Style {
    Style::default().bg(Self::BG_TERTIARY).fg(Self::FG_PRIMARY)
  }

  /// Selection/highlight styling
  pub fn selection() -> Style {
    Style::default().bg(Self::BG_SELECTION).fg(Self::FG_PRIMARY).add_modifier(Modifier::BOLD)
  }

  /// Active selection with accent color
  pub fn selection_active() -> Style {
    Style::default().bg(Self::ACCENT_BLUE).fg(Self::BG_PRIMARY).add_modifier(Modifier::BOLD)
  }

  /// Normal border styling
  pub fn border_normal() -> Style {
    Style::default().fg(Self::BORDER_NORMAL)
  }

  /// Focused border styling
  pub fn border_focused() -> Style {
    Style::default().fg(Self::BORDER_FOCUSED)
  }

  /// Header styling for tables
  pub fn header() -> Style {
    Style::default()
      .bg(Self::BG_SELECTION)
      .fg(Self::ACCENT_CYAN)
      .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
  }

  /// Title styling
  pub fn title() -> Style {
    Style::default().fg(Self::ACCENT_BLUE).add_modifier(Modifier::BOLD)
  }

  /// Success message styling
  pub fn success() -> Style {
    Style::default().fg(Self::SUCCESS).add_modifier(Modifier::BOLD)
  }

  /// Warning message styling  
  pub fn warning() -> Style {
    Style::default().fg(Self::WARNING).add_modifier(Modifier::BOLD)
  }

  /// Error message styling
  pub fn error() -> Style {
    Style::default().fg(Self::ERROR).add_modifier(Modifier::BOLD)
  }

  /// Info message styling
  pub fn info() -> Style {
    Style::default().fg(Self::INFO)
  }

  /// Muted text styling
  pub fn muted() -> Style {
    Style::default().fg(Self::FG_SECONDARY)
  }

  /// Line numbers styling
  pub fn line_numbers() -> Style {
    Style::default().fg(Self::FG_TERTIARY)
  }

  /// Tab styling - normal
  pub fn tab_normal() -> Style {
    Style::default().fg(Self::FG_SECONDARY)
  }

  /// Tab styling - selected
  pub fn tab_selected() -> Style {
    Style::default().fg(Self::ACCENT_BLUE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
  }

  /// Status bar styling
  pub fn status_bar() -> Style {
    Style::default().bg(Self::BG_SECONDARY).fg(Self::FG_PRIMARY)
  }

  // Mode-specific cursor styles for vim editor

  pub fn cursor_normal() -> Style {
    Style::default().bg(Self::ACCENT_ORANGE).fg(Self::BG_PRIMARY).add_modifier(Modifier::BOLD)
  }

  pub fn cursor_insert() -> Style {
    Style::default().bg(Self::ACCENT_GREEN).fg(Self::BG_PRIMARY).add_modifier(Modifier::BOLD)
  }

  pub fn cursor_visual() -> Style {
    Style::default().bg(Self::ACCENT_PURPLE).fg(Self::BG_PRIMARY).add_modifier(Modifier::BOLD)
  }

  // Syntax highlighting for SQL

  pub fn sql_keyword() -> Style {
    Style::default().fg(Self::ACCENT_PURPLE).add_modifier(Modifier::BOLD)
  }

  pub fn sql_function() -> Style {
    Style::default().fg(Self::ACCENT_BLUE)
  }

  pub fn sql_string() -> Style {
    Style::default().fg(Self::ACCENT_GREEN)
  }

  pub fn sql_number() -> Style {
    Style::default().fg(Self::ACCENT_ORANGE)
  }

  pub fn sql_comment() -> Style {
    Style::default().fg(Self::FG_SECONDARY)
  }

  pub fn sql_operator() -> Style {
    Style::default().fg(Self::ACCENT_CYAN)
  }
}

/// Modern color palette inspired by popular Rust TUI applications
pub struct Colors;

impl Colors {
  // Yellow
  pub const DANGER: Color = Color::Rgb(224, 108, 117);
  // Cyan

  // Grays for modern interfaces
  pub const GRAY_100: Color = Color::Rgb(248, 249, 250);
  // Lightest
  pub const GRAY_200: Color = Color::Rgb(233, 236, 239);
  pub const GRAY_300: Color = Color::Rgb(206, 212, 218);
  pub const GRAY_400: Color = Color::Rgb(173, 181, 189);
  pub const GRAY_500: Color = Color::Rgb(108, 117, 125);
  // Medium
  pub const GRAY_600: Color = Color::Rgb(92, 99, 112);
  pub const GRAY_700: Color = Color::Rgb(73, 80, 87);
  pub const GRAY_800: Color = Color::Rgb(52, 58, 64);
  pub const GRAY_900: Color = Color::Rgb(33, 37, 41);
  // Red
  pub const INFO: Color = Color::Rgb(86, 182, 194);
  // One Dark Pro palette
  pub const ONE_DARK_BG: Color = Color::Rgb(40, 44, 52);
  pub const ONE_DARK_FG: Color = Color::Rgb(171, 178, 191);
  // Semantic colors for modern TUI applications
  pub const PRIMARY: Color = Color::Rgb(97, 175, 239);
  // Blue
  pub const SECONDARY: Color = Color::Rgb(198, 120, 221);
  // Purple
  pub const SUCCESS: Color = Color::Rgb(152, 195, 121);
  // Green
  pub const WARNING: Color = Color::Rgb(229, 192, 123); // Darkest
}

// Convenience functions for common styling patterns
pub mod style {
  use super::*;

  /// Creates a modern button style
  pub fn button(active: bool) -> Style {
    if active {
      Theme::selection_active()
    } else {
      Style::default().bg(Theme::BG_SECONDARY).fg(Theme::FG_PRIMARY).add_modifier(Modifier::BOLD)
    }
  }

  /// Creates a modern list item style
  pub fn list_item(selected: bool) -> Style {
    if selected {
      Theme::selection_active()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  /// Creates a modern table row style
  pub fn table_row(selected: bool) -> Style {
    if selected {
      Theme::selection()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  /// Creates a modern border style based on focus state
  pub fn border(focused: bool) -> Style {
    if focused {
      Theme::border_focused()
    } else {
      Theme::border_normal()
    }
  }
}
