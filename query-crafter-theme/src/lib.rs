use ratatui::style::{Color, Modifier, Style};

/// Const theme implementation that can be evaluated at compile time
/// This reduces recompilation cascades when theme values change

// Color constants
pub const ACCENT_BLUE: Color = Color::Rgb(97, 175, 239);
pub const ACCENT_CYAN: Color = Color::Rgb(86, 182, 194);
pub const ACCENT_GREEN: Color = Color::Rgb(152, 195, 121);
pub const ACCENT_ORANGE: Color = Color::Rgb(209, 154, 102);
pub const ACCENT_PURPLE: Color = Color::Rgb(198, 120, 221);
pub const ACCENT_RED: Color = Color::Rgb(224, 108, 117);
pub const BG_CURSOR: Color = Color::Rgb(82, 89, 102);
pub const BG_PRIMARY: Color = Color::Rgb(40, 44, 52);
pub const BG_SECONDARY: Color = Color::Rgb(33, 37, 43);
pub const BG_SELECTION: Color = Color::Rgb(62, 68, 81);
pub const BG_TERTIARY: Color = Color::Rgb(44, 49, 58);
pub const BORDER_FOCUSED: Color = Color::Rgb(97, 175, 239);
pub const BORDER_MUTED: Color = Color::Rgb(62, 68, 81);
pub const BORDER_NORMAL: Color = Color::Rgb(92, 99, 112);
pub const ERROR: Color = Color::Rgb(224, 108, 117);
pub const FG_PRIMARY: Color = Color::Rgb(171, 178, 191);
pub const FG_SECONDARY: Color = Color::Rgb(92, 99, 112);
pub const FG_TERTIARY: Color = Color::Rgb(139, 148, 158);
pub const INFO: Color = Color::Rgb(97, 175, 239);
pub const SUCCESS: Color = Color::Rgb(152, 195, 121);
pub const WARNING: Color = Color::Rgb(229, 192, 123);

// Pre-computed styles as consts where possible
pub const STYLE_BG_PRIMARY: Style = Style::new().bg(BG_PRIMARY).fg(FG_PRIMARY);
pub const STYLE_BG_SECONDARY: Style = Style::new().bg(BG_SECONDARY).fg(FG_PRIMARY);
pub const STYLE_INPUT: Style = Style::new().bg(BG_TERTIARY).fg(FG_PRIMARY);
pub const STYLE_BORDER_NORMAL: Style = Style::new().fg(BORDER_NORMAL);
pub const STYLE_BORDER_FOCUSED: Style = Style::new().fg(BORDER_FOCUSED);
pub const STYLE_MUTED: Style = Style::new().fg(FG_SECONDARY);
pub const STYLE_LINE_NUMBERS: Style = Style::new().fg(FG_TERTIARY);
pub const STYLE_TAB_NORMAL: Style = Style::new().fg(FG_SECONDARY);
pub const STYLE_STATUS_BAR: Style = Style::new().bg(BG_SECONDARY).fg(FG_PRIMARY);
pub const STYLE_INFO: Style = Style::new().fg(INFO);

// Styles that need modifiers must use functions, but we'll use lazy_static for caching
use once_cell::sync::Lazy;

pub static STYLE_SELECTION: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(BG_SELECTION).fg(FG_PRIMARY).add_modifier(Modifier::BOLD)
});

pub static STYLE_SELECTION_ACTIVE: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(ACCENT_BLUE).fg(BG_PRIMARY).add_modifier(Modifier::BOLD)
});

pub static STYLE_HEADER: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(BG_SELECTION).fg(ACCENT_CYAN).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
});

pub static STYLE_TITLE: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD)
});

pub static STYLE_SUCCESS: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(SUCCESS).add_modifier(Modifier::BOLD)
});

pub static STYLE_WARNING: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(WARNING).add_modifier(Modifier::BOLD)
});

pub static STYLE_ERROR: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ERROR).add_modifier(Modifier::BOLD)
});

pub static STYLE_TAB_SELECTED: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_BLUE).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
});

// Cursor styles
pub static STYLE_CURSOR_NORMAL: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(ACCENT_ORANGE).fg(BG_PRIMARY).add_modifier(Modifier::BOLD)
});

pub static STYLE_CURSOR_INSERT: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(ACCENT_GREEN).fg(BG_PRIMARY).add_modifier(Modifier::BOLD)
});

pub static STYLE_CURSOR_VISUAL: Lazy<Style> = Lazy::new(|| {
    Style::new().bg(ACCENT_PURPLE).fg(BG_PRIMARY).add_modifier(Modifier::BOLD)
});

// SQL syntax highlighting
pub static STYLE_SQL_KEYWORD: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_PURPLE).add_modifier(Modifier::BOLD)
});

pub static STYLE_SQL_FUNCTION: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_BLUE)
});

pub static STYLE_SQL_STRING: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_GREEN)
});

pub static STYLE_SQL_NUMBER: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_ORANGE)
});

pub static STYLE_SQL_COMMENT: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(FG_SECONDARY)
});

pub static STYLE_SQL_OPERATOR: Lazy<Style> = Lazy::new(|| {
    Style::new().fg(ACCENT_CYAN)
});

// Re-export the old Theme struct methods as free functions for easier migration
#[inline(always)]
pub fn bg_primary() -> Style { STYLE_BG_PRIMARY }

#[inline(always)]
pub fn bg_secondary() -> Style { STYLE_BG_SECONDARY }

#[inline(always)]
pub fn input() -> Style { STYLE_INPUT }

#[inline(always)]
pub fn selection() -> Style { *STYLE_SELECTION }

#[inline(always)]
pub fn selection_active() -> Style { *STYLE_SELECTION_ACTIVE }

#[inline(always)]
pub fn border_normal() -> Style { STYLE_BORDER_NORMAL }

#[inline(always)]
pub fn border_focused() -> Style { STYLE_BORDER_FOCUSED }

#[inline(always)]
pub fn header() -> Style { *STYLE_HEADER }

#[inline(always)]
pub fn title() -> Style { *STYLE_TITLE }

#[inline(always)]
pub fn success() -> Style { *STYLE_SUCCESS }

#[inline(always)]
pub fn warning() -> Style { *STYLE_WARNING }

#[inline(always)]
pub fn error() -> Style { *STYLE_ERROR }

#[inline(always)]
pub fn info() -> Style { STYLE_INFO }

#[inline(always)]
pub fn muted() -> Style { STYLE_MUTED }

#[inline(always)]
pub fn line_numbers() -> Style { STYLE_LINE_NUMBERS }

#[inline(always)]
pub fn tab_normal() -> Style { STYLE_TAB_NORMAL }

#[inline(always)]
pub fn tab_selected() -> Style { *STYLE_TAB_SELECTED }

#[inline(always)]
pub fn status_bar() -> Style { STYLE_STATUS_BAR }

#[inline(always)]
pub fn cursor_normal() -> Style { *STYLE_CURSOR_NORMAL }

#[inline(always)]
pub fn cursor_insert() -> Style { *STYLE_CURSOR_INSERT }

#[inline(always)]
pub fn cursor_visual() -> Style { *STYLE_CURSOR_VISUAL }

#[inline(always)]
pub fn sql_keyword() -> Style { *STYLE_SQL_KEYWORD }

#[inline(always)]
pub fn sql_function() -> Style { *STYLE_SQL_FUNCTION }

#[inline(always)]
pub fn sql_string() -> Style { *STYLE_SQL_STRING }

#[inline(always)]
pub fn sql_number() -> Style { *STYLE_SQL_NUMBER }

#[inline(always)]
pub fn sql_comment() -> Style { *STYLE_SQL_COMMENT }

#[inline(always)]
pub fn sql_operator() -> Style { *STYLE_SQL_OPERATOR }