use std::error::Error;
use std::fmt;

#[derive(Default)]
pub struct History {
    history: Vec<String>,
    history_index: usize,
}

#[derive(Debug)]
pub enum HistoryError {
    Overflow,
    Underflow,
    Empty,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistoryError::Overflow => write!(f, "forward overflow, not enough history"),
            HistoryError::Underflow => write!(f, "backward underflow, not enough history"),
            HistoryError::Empty => write!(f, "History is empty."),
        }
    }
}

impl Error for HistoryError {}

impl History {
    pub fn get_current_line(&self) -> Option<String> {
        if self.history.is_empty() {
            None
        } else {
            Some(self.history[self.history_index].clone())
        }
    }

    pub fn forward(&mut self) -> Result<(), HistoryError> {
        if !self.history.is_empty() && self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            Ok(())
        } else {
            Err(HistoryError::Overflow)
        }
    }

    pub fn backward(&mut self) -> Result<(), HistoryError> {
        if !self.history.is_empty() && self.history_index > 0 {
            self.history_index -= 1;
            Ok(())
        } else {
            Err(HistoryError::Underflow)
        }
    }

    pub fn push(&mut self, line: String) {
        self.history.push(line);
    }

    pub fn pop(&mut self) -> Result<(), HistoryError> {
        if !self.history.is_empty() {
            self.history.pop();
            self.history_index = if self.history.is_empty() {
                0
            } else {
                self.history.len() - 1
            };
            Ok(())
        } else {
            Err(HistoryError::Empty)
        }
    }

    pub fn is_last_line(&self) -> bool {
        self.history_index + 1 == self.history.len()
    }

    pub fn set_last_line(&mut self, line: String) -> Result<(), HistoryError> {
        if let Some(last) = self.history.last_mut() {
            last.clone_from(&line);
            Ok(())
        } else {
            Err(HistoryError::Empty)
        }
    }

    /// Restore history to the last line added
    pub fn restore(&mut self) -> Result<(), HistoryError> {
        if !self.history.is_empty() {
            self.history_index = self.history.len() - 1;
            Ok(())
        } else {
            Err(HistoryError::Empty)
        }
    }
}
