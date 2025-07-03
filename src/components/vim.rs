use crossterm::event::KeyEvent;
use ratatui::widgets::{Block, BorderType, Borders};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};
use query_crafter_theme as theme;

use crate::{
  editor_common::{Mode, Transition},
  editor_component::EditorComponent,
};

// State of Vim emulation
#[derive(Default, Clone, Debug)]
pub struct Vim {
  mode: Mode,
  pending: Input, // Pending input to handle a sequence with two keys like gg
  textarea: TextArea<'static>,
}

impl Vim {
  pub fn new(mode: Mode) -> Self {
    eprintln!("Initializing Vim editor in mode: {mode:?}");
    Self { mode, pending: Input::default(), textarea: TextArea::default() }
  }

  pub fn mode(&self) -> Mode {
    self.mode
  }

  pub fn with_pending(self, pending: &Input) -> Self {
    Self { mode: self.mode, pending: pending.clone(), textarea: self.textarea }
  }

  fn toggle_line_comment(&mut self) {
    let cursor_row = self.textarea.cursor().0;
    let lines = self.textarea.lines().to_vec();

    if cursor_row >= lines.len() {
      return;
    }

    let current_line = &lines[cursor_row];
    let trimmed = current_line.trim_start();

    // Determine comment style based on content or default to SQL
    let comment_prefix =
      if trimmed.starts_with("//") || current_line.contains("rust") || current_line.contains("javascript") {
        "//"
      } else if trimmed.starts_with("#") || current_line.contains("python") || current_line.contains("bash") {
        "#"
      } else {
        "--" // Default to SQL comment style for database queries
      };

    // Check if line is already commented
    let is_commented = trimmed.starts_with(comment_prefix);
    let leading_whitespace = current_line.len() - trimmed.len();

    // Move to beginning of line
    self.textarea.move_cursor(CursorMove::Head);

    if is_commented {
      // Uncomment: find and remove comment prefix
      for _ in 0..leading_whitespace {
        self.textarea.move_cursor(CursorMove::Forward);
      }
      // Delete the comment prefix
      for _ in 0..comment_prefix.len() {
        self.textarea.delete_next_char();
      }
      // Remove space after comment if present
      let current_lines = self.textarea.lines().to_vec();
      if current_lines.get(cursor_row).and_then(|line| line.chars().nth(self.textarea.cursor().1)) == Some(' ') {
        self.textarea.delete_next_char();
      }
    } else {
      // Comment: add comment prefix
      for _ in 0..leading_whitespace {
        self.textarea.move_cursor(CursorMove::Forward);
      }
      // Insert comment prefix with space
      for c in format!("{comment_prefix} ").chars() {
        self.textarea.insert_char(c);
      }
    }
  }

  pub fn transition(&mut self, input: Input) -> Transition {
    if input.key == Key::Null {
      return Transition::Nop;
    }

    match self.mode {
      Mode::Normal | Mode::Visual | Mode::Operator(_) => {
        match input {
          Input { key: Key::Char('h'), .. } => self.textarea.move_cursor(CursorMove::Back),
          Input { key: Key::Char('j'), .. } => self.textarea.move_cursor(CursorMove::Down),
          Input { key: Key::Char('k'), .. } => self.textarea.move_cursor(CursorMove::Up),
          Input { key: Key::Char('l'), .. } => self.textarea.move_cursor(CursorMove::Forward),
          Input { key: Key::Char('w'), .. } => self.textarea.move_cursor(CursorMove::WordForward),
          Input { key: Key::Char('b'), ctrl: false, .. } => self.textarea.move_cursor(CursorMove::WordBack),
          Input { key: Key::Char('^'), .. } => self.textarea.move_cursor(CursorMove::Head),
          Input { key: Key::Char('0'), .. } => self.textarea.move_cursor(CursorMove::Head),
          Input { key: Key::Char('$'), .. } => self.textarea.move_cursor(CursorMove::End),
          Input { key: Key::Char('D'), .. } => {
            self.textarea.delete_line_by_end();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('C'), .. } => {
            self.textarea.delete_line_by_end();
            self.textarea.cancel_selection();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('p'), .. } => {
            self.textarea.paste();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('u'), ctrl: false, .. } => {
            self.textarea.undo();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('r'), ctrl: true, .. } => {
            self.textarea.redo();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('x'), .. } => {
            self.textarea.delete_next_char();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('i'), .. } => {
            self.textarea.cancel_selection();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('a'), .. } => {
            self.textarea.cancel_selection();
            self.textarea.move_cursor(CursorMove::Forward);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('A'), .. } => {
            self.textarea.cancel_selection();
            self.textarea.move_cursor(CursorMove::End);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('o'), .. } => {
            self.textarea.move_cursor(CursorMove::End);
            self.textarea.insert_newline();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('O'), .. } => {
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.insert_newline();
            self.textarea.move_cursor(CursorMove::Up);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('I'), .. } => {
            self.textarea.cancel_selection();
            self.textarea.move_cursor(CursorMove::Head);
            return Transition::Mode(Mode::Insert);
          },
          // Input { key: Key::Char('q'), .. } => return Transition::Quit,
          Input { key: Key::Char('e'), ctrl: true, .. } => {
            return Transition::Action(crate::action::Action::ExecuteQuery);
          },
          Input { key: Key::Char('y'), ctrl: true, .. } => {
            return Transition::Action(crate::action::Action::ExecuteQuery);
          },
          Input { key: Key::Char('d'), ctrl: true, .. } => self.textarea.scroll(Scrolling::HalfPageDown),
          Input { key: Key::Char('u'), ctrl: true, .. } => self.textarea.scroll(Scrolling::HalfPageUp),
          Input { key: Key::Char('f'), ctrl: true, .. } => self.textarea.scroll(Scrolling::PageDown),
          Input { key: Key::Char('b'), ctrl: true, .. } => self.textarea.scroll(Scrolling::PageUp),
          Input { key: Key::Char('v'), ctrl: false, .. } if self.mode == Mode::Normal => {
            self.textarea.start_selection();
            return Transition::Mode(Mode::Visual);
          },
          Input { key: Key::Char('V'), ctrl: false, .. } if self.mode == Mode::Normal => {
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.start_selection();
            self.textarea.move_cursor(CursorMove::End);
            return Transition::Mode(Mode::Visual);
          },
          Input { key: Key::Esc, .. } | Input { key: Key::Char('v'), ctrl: false, .. } if self.mode == Mode::Visual => {
            self.textarea.cancel_selection();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('g'), ctrl: false, .. }
            if matches!(self.pending, Input { key: Key::Char('g'), ctrl: false, .. }) =>
          {
            self.textarea.move_cursor(CursorMove::Top)
          },
          Input { key: Key::Char('G'), ctrl: false, .. } => self.textarea.move_cursor(CursorMove::Bottom),
          Input { key: Key::Char(c), ctrl: false, .. } if self.mode == Mode::Operator(c) => {
            // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
            self.textarea.move_cursor(CursorMove::Head);
            self.textarea.start_selection();
            let cursor = self.textarea.cursor();
            self.textarea.move_cursor(CursorMove::Down);
            if cursor == self.textarea.cursor() {
              self.textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
            }
          },
          Input { key: Key::Char(op @ ('y' | 'd' | 'c')), ctrl: false, .. } if self.mode == Mode::Normal => {
            self.textarea.start_selection();
            return Transition::Mode(Mode::Operator(op));
          },
          Input { key: Key::Char('g'), ctrl: false, .. } if self.mode == Mode::Normal => {
            return Transition::Mode(Mode::Operator('g'));
          },
          Input { key: Key::Char('y'), ctrl: false, .. } if self.mode == Mode::Visual => {
            self.textarea.copy();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('d'), ctrl: false, .. } if self.mode == Mode::Visual => {
            self.textarea.cut();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('c'), ctrl: false, .. } if self.mode == Mode::Visual => {
            self.textarea.cut();
            return Transition::Mode(Mode::Insert);
          },
          // Handle gcc command for commenting
          Input { key: Key::Char('c'), ctrl: false, .. }
            if matches!(self.pending, Input { key: Key::Char('g'), ctrl: false, .. }) =>
          {
            self.toggle_line_comment();
            return Transition::Mode(Mode::Normal);
          },
          // Handle gc operator for commenting with motions
          Input { key: Key::Char('c'), ctrl: false, .. } if self.mode == Mode::Operator('g') => {
            // For now, just comment current line - can be extended with motions
            self.toggle_line_comment();
            return Transition::Mode(Mode::Normal);
          },
          input => return Transition::Pending(input),
        }

        // Handle the pending operator
        match self.mode {
          Mode::Operator('y') => {
            self.textarea.copy();
            Transition::Mode(Mode::Normal)
          },
          Mode::Operator('d') => {
            self.textarea.cut();
            Transition::Mode(Mode::Normal)
          },
          Mode::Operator('c') => {
            self.textarea.cut();
            Transition::Mode(Mode::Insert)
          },
          Mode::Operator('g') => {
            // Handle 'g' operator - for now just return to normal,
            // proper handling is done in the main match above
            Transition::Mode(Mode::Normal)
          },
          _ => Transition::Nop,
        }
      },
      Mode::Insert => {
        match input {
          Input { key: Key::Esc, .. } | Input { key: Key::Char('c'), ctrl: true, .. } => Transition::Mode(Mode::Normal),
          Input { key: Key::Char('e'), ctrl: true, .. } => Transition::Action(crate::action::Action::ExecuteQuery),
          Input { key: Key::Char('y'), ctrl: true, .. } => Transition::Action(crate::action::Action::ExecuteQuery),
          input => {
            self.textarea.input(input); // Use default key mappings in insert mode
            Transition::Mode(Mode::Insert)
          },
        }
      },
    }
  }

  /// Returns the selected text from a `tui-textarea` by using its selection range.
  ///
  /// # Arguments
  ///
  /// * `textarea` - An immutable reference to the `TextArea` instance.
  ///
  /// # Returns
  ///
  /// * An `Option<String>` containing the selected text if there is a
  ///   selection, or `None` otherwise.
  fn get_selected_text(textarea: &TextArea) -> Option<String> {
    // selection_range() returns Option<((start_row, start_col), (end_row, end_col))>
    let selection = textarea.selection_range()?;

    let (start_row, start_col) = selection.0;
    let (end_row, end_col) = selection.1;

    // The TextArea can be dereferenced to a slice of strings (`&[String]`)
    // which gives us access to the lines of text.
    let lines = textarea.lines();

    if start_row == end_row {
      // Single-line selection
      lines.get(start_row).map(|line| {
        let end = std::cmp::min(end_col, line.len());
        line.get(start_col..end).unwrap_or("").to_string()
      })
    } else {
      // Multi-line selection
      let mut result = Vec::new();

      // First line: from start_col to the end
      if let Some(line) = lines.get(start_row) {
        result.push(line.get(start_col..).unwrap_or(""));
      }

      // Middle lines: entire lines
      if end_row > start_row + 1 {
        for i in (start_row + 1)..end_row {
          if let Some(line) = lines.get(i) {
            result.push(line);
          }
        }
      }

      // Last line: from the beginning to end_col
      if let Some(line) = lines.get(end_row) {
        let end = std::cmp::min(end_col, line.len());
        result.push(line.get(..end).unwrap_or(""));
      }

      Some(result.join("\n"))
    }
  }
}

impl EditorComponent for Vim {
  fn init(&mut self, _area: ratatui::layout::Rect) -> color_eyre::eyre::Result<()> {
    // Set initial block and styling
    let block = Block::default()
      .borders(Borders::ALL)
      .title("Query Editor")
      .title_style(theme::title())
      .border_style(theme::border_normal())
      .border_type(BorderType::Rounded)
      .style(theme::bg_primary());
    self.textarea.set_block(block);
    self.textarea.set_cursor_style(self.mode.cursor_style());
    Ok(())
  }

  fn on_key_event(&mut self, key: KeyEvent) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
    let input = Input::from(key);
    let transition = self.transition(input);

    match transition {
      Transition::Mode(mode) if self.mode != mode => {
        self.textarea.set_cursor_style(mode.cursor_style());
        self.mode = mode;
      },
      Transition::Nop | Transition::Mode(_) => {},
      Transition::Pending(ref input) => {
        self.pending = input.clone();
      },
      Transition::Quit => return Ok(Some(crate::action::Action::Quit)),
      Transition::Action(action) => return Ok(Some(action)),
    }

    Ok(None)
  }

  fn draw(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    self.draw_with_focus(f, area, false); // Default to not focused
  }

  fn draw_with_focus(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool) {
    // Update the block title to show current mode with focus state
    let border_style =
      if is_focused { theme::border_focused() } else { theme::border_normal() };

    let block = Block::default()
      .borders(Borders::ALL)
      .title(format!("Query Editor - {}", self.mode))
      .title_style(theme::title())
      .border_style(border_style)
      .border_type(BorderType::Rounded)
      .style(theme::bg_primary());
    self.textarea.set_block(block);

    // Ensure cursor style is always up to date
    self.textarea.set_cursor_style(self.mode.cursor_style());

    f.render_widget(&self.textarea, area);
  }

  fn get_text(&self) -> String {
    self.textarea.lines().join("\n")
  }

  fn get_selected_text(&self) -> Option<String> {
    if self.textarea.is_selecting() {
      Vim::get_selected_text(&self.textarea)
    } else {
      None
    }
  }

  fn set_text(&mut self, text: &str) {
    self.textarea = TextArea::from(text.lines().map(String::from).collect::<Vec<_>>());
    self.textarea.set_cursor_style(self.mode.cursor_style());
    let block = Block::default()
      .borders(Borders::ALL)
      .title(format!("Query Editor - {}", self.mode))
      .title_style(theme::title())
      .border_style(theme::border_normal())
      .border_type(BorderType::Rounded)
      .style(theme::bg_primary());
    self.textarea.set_block(block);
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}

impl Vim {
  /// Get the current cursor position (row, col)
  pub fn get_cursor_position(&self) -> (usize, usize) {
    self.textarea.cursor()
  }

  /// Get text from start up to current cursor position
  pub fn get_text_up_to_cursor(&self) -> String {
    let lines = self.textarea.lines();
    let (cursor_row, cursor_col) = self.textarea.cursor();

    if cursor_row >= lines.len() {
      return lines.join("\n");
    }

    let mut result = String::new();

    // Add complete lines before cursor row
    for (i, line) in lines.iter().enumerate() {
      if i < cursor_row {
        if i > 0 {
          result.push('\n');
        }
        result.push_str(line);
      } else if i == cursor_row {
        if cursor_row > 0 {
          result.push('\n');
        }
        // Add partial line up to cursor column
        let chars: Vec<char> = line.chars().collect();
        let end_pos = std::cmp::min(cursor_col, chars.len());
        result.push_str(&chars[..end_pos].iter().collect::<String>());
        break;
      }
    }

    result
  }

  /// Insert text at current cursor position
  pub fn insert_text_at_cursor(&mut self, text: &str) {
    self.textarea.insert_str(text);
  }

  /// Delete the word before the cursor
  pub fn delete_word_before_cursor(&mut self) {
    use tui_textarea::CursorMove;

    // Save current position
    let original_pos = self.textarea.cursor();

    // Move to start of current word
    self.textarea.move_cursor(CursorMove::WordBack);
    let word_start = self.textarea.cursor();

    // If we moved, select from word start to original position
    if word_start != original_pos {
      self.textarea.start_selection();

      // Move back to original position to select the word
      while self.textarea.cursor() != original_pos {
        self.textarea.move_cursor(CursorMove::Forward);
      }

      // Delete the selected text
      self.textarea.cut();
    }
  }
}
