use std::{env, fs, io, io::BufRead};

use crossterm::{
  event::{DisableMouseCapture, EnableMouseCapture, KeyEvent},
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  backend::Backend,
  layout::Rect,
  widgets::{Block, Borders},
};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

use crate::{
  action::Action,
  editor_common::{Mode, Transition},
  editor_component::EditorComponent,
};

// State of Vim emulation
#[derive(Default, Clone)]
pub struct Vim {
  mode: Mode,
  pending: Input, // Pending input to handle a sequence with two keys like gg
  textarea: TextArea<'static>,
}

impl Vim {
  pub fn new(mode: Mode) -> Self {
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
          Input { key: Key::Char('q'), .. } => return Transition::Quit,
          Input { key: Key::Char('e'), ctrl: true, .. } => self.textarea.scroll((1, 0)),
          Input { key: Key::Char('y'), ctrl: true, .. } => self.textarea.scroll((-1, 0)),
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
          input => {
            self.textarea.input(input); // Use default key mappings in insert mode
            Transition::Mode(Mode::Insert)
          },
        }
      },
    }
  }
}

impl EditorComponent for Vim {
  fn init(&mut self, _area: ratatui::layout::Rect) -> color_eyre::eyre::Result<()> {
    // Set initial block and styling
    let block = ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title("Query Editor");
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
    }

    Ok(None)
  }

  fn draw(&mut self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    // Update the block title to show current mode
    let block = ratatui::widgets::Block::default()
      .borders(ratatui::widgets::Borders::ALL)
      .title(format!("Query Editor - {}", self.mode));
    self.textarea.set_block(block);

    f.render_widget(&self.textarea, area);
  }

  fn get_text(&self) -> String {
    self.textarea.lines().join("\n")
  }

  fn set_text(&mut self, text: &str) {
    self.textarea = TextArea::from(text.lines().map(String::from).collect::<Vec<_>>());
    self.textarea.set_cursor_style(self.mode.cursor_style());
    let block = ratatui::widgets::Block::default()
      .borders(ratatui::widgets::Borders::ALL)
      .title(format!("Query Editor - {}", self.mode));
    self.textarea.set_block(block);
  }

  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
    self
  }
}
