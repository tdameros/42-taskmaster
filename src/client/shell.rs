use libc::{tcgetattr, tcsetattr, termios, ECHO, ICANON, TCSANOW};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

const ESCAPE_KEY: u8 = 0x1B;
const BACKSPACE: u8 = 0x7F;
const CLEAR_LINE: &str = "\x1B[2K";
const CLEAR_CHAR: &str = "\x1B[1D \x1B[1D";
const RESET_CURSOR: &str = "\x1B[0G";
const ARROW_UP: [u8; 3] = [ESCAPE_KEY, b'[', b'A'];
const ARROW_DOWN: [u8; 3] = [ESCAPE_KEY, b'[', b'B'];
const PROMPT: &str = "> ";

pub struct CliShell {
    history: Vec<String>,
    history_index: usize,
    current_line: String,
}

enum HistoryDirection {
    Backward,
    Forward,
}

impl CliShell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable raw mode to read single keypresses without waiting for Enter
    fn enable_raw_mode() -> termios {
        let fd = io::stdin().as_raw_fd();
        let mut termios = unsafe {
            let mut termios = std::mem::zeroed();
            tcgetattr(fd, &mut termios);
            termios
        };

        let orig_termios = termios;
        // Disable canonical mode and echo
        termios.c_lflag &= !(ICANON | ECHO);
        // Apply changes immediately
        unsafe { tcsetattr(fd, TCSANOW, &termios) };

        orig_termios
    }

    /// Restore the terminal to its original settings
    fn disable_raw_mode(orig_termios: termios) {
        let fd = io::stdin().as_raw_fd();
        unsafe {
            tcsetattr(fd, TCSANOW, &orig_termios);
        }
    }

    /// Function to read a single keypress, including escape sequences
    fn getch() -> Vec<u8> {
        let stdin = io::stdin();
        let mut buffer = vec![0; 3];
        stdin.lock().read_exact(&mut buffer[..1]).unwrap();

        if buffer[0] == ESCAPE_KEY {
            stdin.lock().read_exact(&mut buffer[1..3]).unwrap();
        } else {
            buffer.truncate(1);
        }
        buffer
    }

    fn refresh_prompt(&self) {
        print!("{}", CLEAR_LINE);
        print!("{}", RESET_CURSOR);
        print!("{}", PROMPT);
        print!("{}", self.history[self.history_index]);
        io::stdout().flush().unwrap();
    }

    /// Process history navigation (up or down arrow)
    fn handle_history_navigation(&mut self, direction: HistoryDirection) {
        match direction {
            HistoryDirection::Backward => {
                if self.history_index > 0 {
                    self.history_index -= 1;
                }
            }
            HistoryDirection::Forward => {
                if self.history_index + 1 < self.history.len() {
                    self.history_index += 1;
                }
            }
        }
        self.current_line
            .clone_from(&self.history[self.history_index]);
        self.refresh_prompt();
    }

    fn handle_character_input(&mut self, ch: u8) {
        if ch.is_ascii_graphic() || ch == b' ' {
            print!("{}", ch as char);
            self.current_line.push(ch as char);
            if self.history_index == self.history.len() - 1 {
                self.history[self.history_index].clone_from(&self.current_line);
            }
        } else if ch == BACKSPACE && !self.current_line.is_empty() {
            self.current_line.pop();
            print!("{CLEAR_CHAR}");
        }
        io::stdout().flush().unwrap();
    }

    pub fn read_line(&mut self) -> String {
        let origin_termios = Self::enable_raw_mode();
        print!("{PROMPT}");
        io::stdout().flush().unwrap();
        let mut input = Self::getch();
        while !(input.len() == 1 && input[0] == b'\n') {
            if input == ARROW_UP {
                self.handle_history_navigation(HistoryDirection::Backward);
            } else if input == ARROW_DOWN {
                self.handle_history_navigation(HistoryDirection::Forward);
            } else if input.len() == 1 {
                self.handle_character_input(input[0]);
            }
            input = Self::getch();
        }
        println!();
        let len = self.history.len();
        if !self.current_line.is_empty() {
            self.history[len - 1].clone_from(&self.current_line);
            self.history.push(String::new());
        }
        let return_line = self.current_line.clone();
        self.current_line.clear();
        self.history_index = self.history.len() - 1;
        Self::disable_raw_mode(origin_termios);
        return_line
    }
}

impl Default for CliShell {
    fn default() -> Self {
        Self {
            history: vec![String::new()],
            history_index: 0,
            current_line: String::new(),
        }
    }
}
