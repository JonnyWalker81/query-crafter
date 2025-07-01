use async_trait::async_trait;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  backend::Backend,
  layout::Rect,
  style::{Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, Paragraph},
  Frame,
};
use ropey::{Rope, RopeSlice};

use crate::{
  action::Action,
  editor_common::{Mode, Transition},
  editor_component::EditorComponent,
};

pub struct CustomVimEditor {
  mode: Mode,
  content: Rope,
  cursor_row: usize,
  cursor_col: usize,
  selection: Option<(usize, usize)>, // (start_char_idx, end_char_idx)
}

impl Default for CustomVimEditor {
  fn default() -> Self {
    Self { mode: Mode::Normal, content: Rope::new(), cursor_row: 0, cursor_col: 0, selection: None }
  }
}

impl CustomVimEditor {
  fn move_cursor(&mut self, row_offset: isize, col_offset: isize) {
    let num_lines = self.content.len_lines();
    let new_row_signed = self.cursor_row as isize + row_offset;
    let new_col_signed = self.cursor_col as isize + col_offset;

    let new_row = if new_row_signed < 0 {
      0
    } else if new_row_signed as usize >= num_lines {
      num_lines.saturating_sub(1)
    } else {
      new_row_signed as usize
    };

    let line_len = self.content.line(new_row).len_chars();
    let new_col = if new_col_signed < 0 {
      0
    } else if new_col_signed as usize > line_len {
      line_len
    } else {
      new_col_signed as usize
    };

    self.cursor_row = new_row;
    self.cursor_col = new_col;
  }

  fn insert_char(&mut self, c: char) {
    let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
    self.content.insert_char(char_idx, c);
    self.move_cursor(0, 1);
  }

  fn insert_newline(&mut self) {
    let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
    self.content.insert_char(char_idx, '\n');
    self.cursor_row += 1;
    self.cursor_col = 0;
  }

  fn delete_char_backwards(&mut self) {
    if self.cursor_col > 0 {
      let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
      self.content.remove(char_idx - 1..char_idx);
      self.move_cursor(0, -1);
    } else if self.cursor_row > 0 {
      let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
      self.content.remove(char_idx - 1..char_idx);
      self.move_cursor(-1, self.content.line(self.cursor_row - 1).len_chars() as isize);
    }
  }

  fn delete_char_forwards(&mut self) {
    let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
    if char_idx < self.content.len_chars() {
      self.content.remove(char_idx..char_idx + 1);
    }
  }

  fn get_comment_style(line: &str) -> &'static str {
    if line.trim_start().starts_with("//") {
      "//"
    } else if line.trim_start().starts_with("#") {
      "#"
    } else if line.trim_start().starts_with("--") {
      "--"
    } else {
      "//" // Default to C-style comment
    }
  }

  fn toggle_comment(&mut self) {
    let (start_line, end_line) = if let Some((start_idx, end_idx)) = self.selection {
      let start_line = self.content.char_to_line(start_idx);
      let end_line = self.content.char_to_line(end_idx);
      (start_line, end_line)
    } else {
      (self.cursor_row, self.cursor_row)
    };

    for i in start_line..=end_line {
      let line = self.content.line(i);
      let line_str = line.to_string();
      let trimmed_line = line_str.trim_start();
      let leading_whitespace_len = line_str.len() - trimmed_line.len();
      let comment_str = Self::get_comment_style(&line_str);

      if trimmed_line.starts_with(comment_str) {
        // Uncomment
        let comment_start_char_idx = self.content.line_to_char(i) + leading_whitespace_len;
        self.content.remove(comment_start_char_idx..comment_start_char_idx + comment_str.len());
      } else {
        // Comment
        let insert_char_idx = self.content.line_to_char(i) + leading_whitespace_len;
        self.content.insert(insert_char_idx, comment_str);
      }
    }
    self.selection = None;
  }

  fn start_selection(&mut self) {
    let current_char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
    self.selection = Some((current_char_idx, current_char_idx));
  }

  fn update_selection(&mut self) {
    if let Some((start_idx, _)) = self.selection {
      let current_char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
      let (new_start, new_end) =
        if current_char_idx < start_idx { (current_char_idx, start_idx) } else { (start_idx, current_char_idx) };
      self.selection = Some((new_start, new_end));
    }
  }

  fn clear_selection(&mut self) {
    self.selection = None;
  }
}

#[async_trait]
impl EditorComponent for CustomVimEditor {
  fn init(&mut self, _area: Rect) -> Result<()> {
    Ok(())
  }

  fn on_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    match self.mode {
      Mode::Normal => {
        match key.code {
          KeyCode::Char('h') => self.move_cursor(0, -1),
          KeyCode::Char('j') => self.move_cursor(1, 0),
          KeyCode::Char('k') => self.move_cursor(-1, 0),
          KeyCode::Char('l') => self.move_cursor(0, 1),
          KeyCode::Char('i') => self.mode = Mode::Insert,
          KeyCode::Char('v') => {
            self.start_selection();
            self.mode = Mode::Visual;
          },
          KeyCode::Char('x') => self.delete_char_forwards(),
          KeyCode::Char('q') => return Ok(Some(Action::Quit)),
          KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(Some(Action::Quit)),
          KeyCode::Char('g') => self.mode = Mode::Operator('g'), // For gcc/gc
          _ => {},
        }
      },
      Mode::Insert => {
        match key.code {
          KeyCode::Esc => self.mode = Mode::Normal,
          KeyCode::Backspace => self.delete_char_backwards(),
          KeyCode::Enter => self.insert_newline(),
          KeyCode::Char(c) => self.insert_char(c),
          _ => {},
        }
      },
      Mode::Visual => {
        self.update_selection();
        match key.code {
          KeyCode::Esc => {
            self.clear_selection();
            self.mode = Mode::Normal;
          },
          KeyCode::Char('h') => self.move_cursor(0, -1),
          KeyCode::Char('j') => self.move_cursor(1, 0),
          KeyCode::Char('k') => self.move_cursor(-1, 0),
          KeyCode::Char('l') => self.move_cursor(0, 1),
          KeyCode::Char('c') => {
            self.toggle_comment();
            self.mode = Mode::Normal;
          },
          _ => {},
        }
      },
      Mode::Operator('g') => {
        match key.code {
          KeyCode::Char('c') => {
            self.toggle_comment();
            self.mode = Mode::Normal;
          },
          _ => self.mode = Mode::Normal, // Fallback if not 'gc'
        }
      },
      _ => {},
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut ratatui::Frame, area: Rect) {
    let mut lines = Vec::new();
    let (selection_start, selection_end) =
      if let Some((start, end)) = self.selection { (Some(start), Some(end)) } else { (None, None) };

    for (i, line) in self.content.lines().enumerate() {
      let mut spans = Vec::new();
      let line_str = line.to_string();
      let line_start_char_idx = self.content.line_to_char(i);

      for (j, c) in line_str.chars().enumerate() {
        let mut style = Style::default();
        let char_absolute_idx = line_start_char_idx + j;

        if let (Some(s_start), Some(s_end)) = (selection_start, selection_end) {
          if char_absolute_idx >= s_start && char_absolute_idx < s_end {
            style = style.add_modifier(Modifier::REVERSED);
          }
        }

        if self.cursor_row == i && self.cursor_col == j {
          style = style.add_modifier(Modifier::REVERSED);
        }
        spans.push(Span::styled(c.to_string(), style));
      }
      lines.push(Line::from(spans));
    }

    let block = Block::default().borders(Borders::ALL).title(format!("Custom Vim Editor - {}", self.mode));
    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
  }

  fn get_text(&self) -> String {
    self.content.to_string()
  }

  fn set_text(&mut self, text: &str) {
    self.content = Rope::from(text);
    self.cursor_row = 0;
    self.cursor_col = 0;
    self.selection = None;
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}
