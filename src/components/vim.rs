use std::{env, fmt, fs, io, io::BufRead};

use crossterm::{
  event::{DisableMouseCapture, EnableMouseCapture},
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  backend::CrosstermBackend,
  style::{Color, Modifier, Style},
  widgets::{Block, Borders},
  Terminal,
};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  #[default]
  Normal,
  Insert,
  Visual,
  Operator(char),
}

impl Mode {
  // fn block<'a>(&self) -> Block<'a> {
  //   let help = match self {
  //     Self::Normal => "type q to quit, type i to enter insert mode",
  //     Self::Insert => "type Esc to back to normal mode",
  //     Self::Visual => "type y to yank, type d to delete, type Esc to back to normal mode",
  //     Self::Operator(_) => "move cursor to apply operator",
  //   };
  //   let title = format!("{} MODE ({})", self, help);
  //   Block::default().borders(Borders::ALL).title(title)
  // }

  pub fn cursor_style(&self) -> Style {
    let color = match self {
      Self::Normal => Color::Reset,
      Self::Insert => Color::LightBlue,
      Self::Visual => Color::LightYellow,
      Self::Operator(_) => Color::LightGreen,
    };
    Style::default().fg(color).add_modifier(Modifier::REVERSED)
  }
}

impl fmt::Display for Mode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
    match self {
      Self::Normal => write!(f, "NORMAL"),
      Self::Insert => write!(f, "INSERT"),
      Self::Visual => write!(f, "VISUAL"),
      Self::Operator(c) => write!(f, "OPERATOR({})", c),
    }
  }
}

// How the Vim emulation state transitions
#[derive(Debug)]
pub enum Transition {
  Nop,
  Mode(Mode),
  Pending(Input),
  Quit,
}

// State of Vim emulation
#[derive(Default, Clone)]
pub struct Vim {
  mode: Mode,
  pending: Input, // Pending input to handle a sequence with two keys like gg
}

impl Vim {
  pub fn new(mode: Mode) -> Self {
    Self { mode, pending: Input::default() }
  }

  pub fn mode(&self) -> Mode {
    self.mode
  }

  pub fn with_pending(self, pending: &Input) -> Self {
    Self { mode: self.mode, pending: pending.clone() }
  }

  pub fn transition(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
    if input.key == Key::Null {
      return Transition::Nop;
    }

    match self.mode {
      Mode::Normal | Mode::Visual | Mode::Operator(_) => {
        match input {
          Input { key: Key::Char('h'), .. } => textarea.move_cursor(CursorMove::Back),
          Input { key: Key::Char('j'), .. } => textarea.move_cursor(CursorMove::Down),
          Input { key: Key::Char('k'), .. } => textarea.move_cursor(CursorMove::Up),
          Input { key: Key::Char('l'), .. } => textarea.move_cursor(CursorMove::Forward),
          Input { key: Key::Char('w'), .. } => textarea.move_cursor(CursorMove::WordForward),
          Input { key: Key::Char('b'), ctrl: false, .. } => textarea.move_cursor(CursorMove::WordBack),
          Input { key: Key::Char('^'), .. } => textarea.move_cursor(CursorMove::Head),
          Input { key: Key::Char('$'), .. } => textarea.move_cursor(CursorMove::End),
          Input { key: Key::Char('D'), .. } => {
            textarea.delete_line_by_end();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('C'), .. } => {
            textarea.delete_line_by_end();
            textarea.cancel_selection();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('p'), .. } => {
            textarea.paste();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('u'), ctrl: false, .. } => {
            textarea.undo();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('r'), ctrl: true, .. } => {
            textarea.redo();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('x'), .. } => {
            textarea.delete_next_char();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('i'), .. } => {
            textarea.cancel_selection();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('a'), .. } => {
            textarea.cancel_selection();
            textarea.move_cursor(CursorMove::Forward);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('A'), .. } => {
            textarea.cancel_selection();
            textarea.move_cursor(CursorMove::End);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('o'), .. } => {
            textarea.move_cursor(CursorMove::End);
            textarea.insert_newline();
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('O'), .. } => {
            textarea.move_cursor(CursorMove::Head);
            textarea.insert_newline();
            textarea.move_cursor(CursorMove::Up);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('I'), .. } => {
            textarea.cancel_selection();
            textarea.move_cursor(CursorMove::Head);
            return Transition::Mode(Mode::Insert);
          },
          Input { key: Key::Char('q'), .. } => return Transition::Quit,
          Input { key: Key::Char('e'), ctrl: true, .. } => textarea.scroll((1, 0)),
          Input { key: Key::Char('y'), ctrl: true, .. } => textarea.scroll((-1, 0)),
          Input { key: Key::Char('d'), ctrl: true, .. } => textarea.scroll(Scrolling::HalfPageDown),
          Input { key: Key::Char('u'), ctrl: true, .. } => textarea.scroll(Scrolling::HalfPageUp),
          Input { key: Key::Char('f'), ctrl: true, .. } => textarea.scroll(Scrolling::PageDown),
          Input { key: Key::Char('b'), ctrl: true, .. } => textarea.scroll(Scrolling::PageUp),
          Input { key: Key::Char('v'), ctrl: false, .. } if self.mode == Mode::Normal => {
            textarea.start_selection();
            return Transition::Mode(Mode::Visual);
          },
          Input { key: Key::Char('V'), ctrl: false, .. } if self.mode == Mode::Normal => {
            textarea.move_cursor(CursorMove::Head);
            textarea.start_selection();
            textarea.move_cursor(CursorMove::End);
            return Transition::Mode(Mode::Visual);
          },
          Input { key: Key::Esc, .. } | Input { key: Key::Char('v'), ctrl: false, .. } if self.mode == Mode::Visual => {
            textarea.cancel_selection();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('g'), ctrl: false, .. }
            if matches!(self.pending, Input { key: Key::Char('g'), ctrl: false, .. }) =>
          {
            textarea.move_cursor(CursorMove::Top)
          },
          Input { key: Key::Char('G'), ctrl: false, .. } => textarea.move_cursor(CursorMove::Bottom),
          Input { key: Key::Char(c), ctrl: false, .. } if self.mode == Mode::Operator(c) => {
            // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
            textarea.move_cursor(CursorMove::Head);
            textarea.start_selection();
            let cursor = textarea.cursor();
            textarea.move_cursor(CursorMove::Down);
            if cursor == textarea.cursor() {
              textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
            }
          },
          Input { key: Key::Char(op @ ('y' | 'd' | 'c')), ctrl: false, .. } if self.mode == Mode::Normal => {
            textarea.start_selection();
            return Transition::Mode(Mode::Operator(op));
          },
          Input { key: Key::Char('y'), ctrl: false, .. } if self.mode == Mode::Visual => {
            textarea.copy();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('d'), ctrl: false, .. } if self.mode == Mode::Visual => {
            textarea.cut();
            return Transition::Mode(Mode::Normal);
          },
          Input { key: Key::Char('c'), ctrl: false, .. } if self.mode == Mode::Visual => {
            textarea.cut();
            return Transition::Mode(Mode::Insert);
          },
          input => return Transition::Pending(input),
        }

        // Handle the pending operator
        match self.mode {
          Mode::Operator('y') => {
            textarea.copy();
            Transition::Mode(Mode::Normal)
          },
          Mode::Operator('d') => {
            textarea.cut();
            Transition::Mode(Mode::Normal)
          },
          Mode::Operator('c') => {
            textarea.cut();
            Transition::Mode(Mode::Insert)
          },
          _ => Transition::Nop,
        }
      },
      Mode::Insert => {
        match input {
          Input { key: Key::Esc, .. } | Input { key: Key::Char('c'), ctrl: true, .. } => Transition::Mode(Mode::Normal),
          input => {
            textarea.input(input); // Use default key mappings in insert mode
            Transition::Mode(Mode::Insert)
          },
        }
      },
    }
  }
}
