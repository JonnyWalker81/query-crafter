use ratatui::{
  buffer::Buffer,
  layout::Rect,
  style::{Color, Style},
  text::{Line, Span},
  widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Widget, Wrap},
};

use crate::{
  autocomplete::{AutocompleteState, SuggestionItem, SuggestionKind},
  theme::Theme,
};

/// Widget for rendering autocomplete suggestions in a popup
pub struct AutocompletePopup<'a> {
  state: &'a AutocompleteState,
  max_height: u16,
  max_width: u16,
}

impl<'a> AutocompletePopup<'a> {
  pub fn new(state: &'a AutocompleteState) -> Self {
    Self {
      state,
      max_height: 10, // Show up to 10 suggestions
      max_width: 50,  // Maximum popup width
    }
  }

  pub fn max_height(mut self, height: u16) -> Self {
    self.max_height = height;
    self
  }

  pub fn max_width(mut self, width: u16) -> Self {
    self.max_width = width;
    self
  }

  /// Calculate the popup area based on cursor position and available space
  pub fn calculate_popup_area(&self, parent_area: Rect, cursor_pos: (u16, u16)) -> Rect {
    let (cursor_x, cursor_y) = cursor_pos;

    // Calculate popup dimensions
    let popup_height = std::cmp::min(
      self.max_height.saturating_add(2), // +2 for borders
      self.state.suggestions.len() as u16 + 2,
    );

    let popup_width = std::cmp::min(
      self.max_width,
      self.calculate_max_suggestion_width().saturating_add(4), // +4 for borders and padding
    );

    // Determine popup position (prefer below cursor, but above if not enough space)
    let popup_y = if cursor_y + popup_height + 1 <= parent_area.bottom() {
      // Show below cursor
      cursor_y + 1
    } else if cursor_y >= popup_height {
      // Show above cursor
      cursor_y.saturating_sub(popup_height)
    } else {
      // Show at the bottom of the screen if neither fits well
      parent_area.bottom().saturating_sub(popup_height)
    };

    // Determine popup x position (try to align with cursor, but keep within bounds)
    let popup_x = if cursor_x + popup_width <= parent_area.right() {
      cursor_x
    } else {
      parent_area.right().saturating_sub(popup_width)
    };

    Rect {
      x: popup_x.max(parent_area.x),
      y: popup_y.max(parent_area.y),
      width: popup_width.min(parent_area.width),
      height: popup_height.min(parent_area.height),
    }
  }

  fn calculate_max_suggestion_width(&self) -> u16 {
    self
      .state
      .suggestions
      .iter()
      .map(|s| self.format_suggestion_text(s).len() as u16)
      .max()
      .unwrap_or(20)
      .min(self.max_width.saturating_sub(4))
  }

  fn format_suggestion_text(&self, suggestion: &SuggestionItem) -> String {
    match suggestion.kind {
      SuggestionKind::Table => format!("📋 {}", suggestion.text),
      SuggestionKind::Column => {
        if let Some(table) = &suggestion.table_context {
          format!("📊 {}.{}", table, suggestion.text)
        } else {
          format!("📊 {}", suggestion.text)
        }
      },
      SuggestionKind::Keyword => format!("🔧 {}", suggestion.text),
    }
  }

  fn get_suggestion_style(&self, _index: usize, is_selected: bool) -> Style {
    if is_selected {
      Theme::selection_active()
    } else {
      Style::default().fg(Theme::FG_PRIMARY)
    }
  }

  fn get_kind_color(&self, kind: &SuggestionKind) -> Color {
    match kind {
      SuggestionKind::Table => Theme::ACCENT_BLUE,
      SuggestionKind::Column => Theme::ACCENT_GREEN,
      SuggestionKind::Keyword => Theme::ACCENT_PURPLE,
    }
  }
}

impl<'a> Widget for AutocompletePopup<'a> {
  fn render(self, area: Rect, buf: &mut Buffer) {
    if !self.state.is_active || self.state.suggestions.is_empty() {
      return;
    }

    // Clear the area first
    Clear.render(area, buf);

    // Create the popup block
    let block = Block::default()
      .borders(Borders::ALL)
      .border_type(BorderType::Rounded)
      .border_style(Theme::border_focused())
      .style(Theme::bg_secondary())
      .title(" Autocomplete ");

    let inner_area = block.inner(area);
    block.render(area, buf);

    // Create list items from suggestions
    let items: Vec<ListItem> = self
      .state
      .suggestions
      .iter()
      .enumerate()
      .map(|(index, suggestion)| {
        let is_selected = index == self.state.selected_index;
        let style = self.get_suggestion_style(index, is_selected);

        // Create spans with appropriate colors
        let icon_style = Style::default().fg(self.get_kind_color(&suggestion.kind));

        let spans = match suggestion.kind {
          SuggestionKind::Table => vec![Span::styled("📋 ", icon_style), Span::styled(&suggestion.text, style)],
          SuggestionKind::Column => {
            if let Some(table) = &suggestion.table_context {
              vec![
                Span::styled("📊 ", icon_style),
                Span::styled(table, Style::default().fg(Theme::FG_SECONDARY)),
                Span::styled(".", Style::default().fg(Theme::FG_SECONDARY)),
                Span::styled(&suggestion.text, style),
              ]
            } else {
              vec![Span::styled("📊 ", icon_style), Span::styled(&suggestion.text, style)]
            }
          },
          SuggestionKind::Keyword => vec![Span::styled("🔧 ", icon_style), Span::styled(&suggestion.text, style)],
        };

        ListItem::new(Line::from(spans))
      })
      .collect();

    // Create the list widget
    let list =
      List::new(items).style(Theme::bg_secondary()).highlight_style(Theme::selection_active()).highlight_symbol("▶ ");

    // Create list state for proper rendering
    let mut list_state = ListState::default();
    list_state.select(Some(self.state.selected_index));

    // Render the list
    ratatui::widgets::StatefulWidget::render(list, inner_area, buf, &mut list_state);

    // Add a footer with help text if there's space
    if inner_area.height > self.state.suggestions.len() as u16 + 1 {
      let help_area = Rect {
        x: inner_area.x,
        y: inner_area.y + self.state.suggestions.len() as u16,
        width: inner_area.width,
        height: 1,
      };

      let help_text = "↑↓: Navigate • Enter/Tab: Accept • Esc: Cancel";
      let help_paragraph =
        Paragraph::new(help_text).style(Style::default().fg(Theme::FG_SECONDARY)).wrap(Wrap { trim: true });

      help_paragraph.render(help_area, buf);
    }
  }
}

/// Utility struct for rendering autocomplete popup at a specific position
pub struct AutocompleteRenderer<'a> {
  popup: AutocompletePopup<'a>,
  cursor_position: (u16, u16),
}

impl<'a> AutocompleteRenderer<'a> {
  pub fn new(state: &'a AutocompleteState, cursor_position: (u16, u16)) -> Self {
    Self { popup: AutocompletePopup::new(state), cursor_position }
  }

  pub fn max_height(mut self, height: u16) -> Self {
    self.popup = self.popup.max_height(height);
    self
  }

  pub fn max_width(mut self, width: u16) -> Self {
    self.popup = self.popup.max_width(width);
    self
  }

  /// Render the autocomplete popup on the given frame
  pub fn render(self, frame: &mut ratatui::Frame, area: Rect) {
    if !self.popup.state.is_active || self.popup.state.suggestions.is_empty() {
      return;
    }

    let popup_area = self.popup.calculate_popup_area(area, self.cursor_position);
    frame.render_widget(self.popup, popup_area);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::autocomplete::{AutocompleteState, SuggestionItem};

  #[test]
  fn test_popup_area_calculation() {
    let mut state = AutocompleteState::new();
    state.suggestions =
      vec![SuggestionItem::new_table("users".to_string()), SuggestionItem::new_table("posts".to_string())];
    state.is_active = true;

    let popup = AutocompletePopup::new(&state);
    let parent_area = Rect::new(0, 0, 80, 24);
    let cursor_pos = (10, 5);

    let popup_area = popup.calculate_popup_area(parent_area, cursor_pos);

    // Should be positioned below cursor
    assert_eq!(popup_area.y, 6); // cursor_y + 1
    assert_eq!(popup_area.x, 10); // cursor_x
  }

  #[test]
  fn test_popup_area_above_cursor() {
    let mut state = AutocompleteState::new();
    state.suggestions =
      vec![SuggestionItem::new_table("users".to_string()), SuggestionItem::new_table("posts".to_string())];
    state.is_active = true;

    let popup = AutocompletePopup::new(&state);
    let parent_area = Rect::new(0, 0, 80, 24);
    let cursor_pos = (10, 22); // Near bottom

    let popup_area = popup.calculate_popup_area(parent_area, cursor_pos);

    // Should be positioned above cursor since there's no space below
    assert!(popup_area.y < 22);
  }
}
