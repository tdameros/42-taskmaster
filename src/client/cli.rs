/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use crate::history::History;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use tcl::error::TaskmasterError;
use tcl::mylibc::{ECHO, ICANON, TCSANOW};

/* -------------------------------------------------------------------------- */
/*                                  Constants                                 */
/* -------------------------------------------------------------------------- */
const ESCAPE_KEY: u8 = 0x1B;
const BACKSPACE: u8 = 0x7F;
const CLEAR_LINE: &str = "\x1B[2K";
const CLEAR_CHAR: &str = "\x1B[1D \x1B[1D";
const RESET_CURSOR: &str = "\x1B[0G";
const ARROW_UP: [u8; 3] = [ESCAPE_KEY, b'[', b'A'];
const ARROW_DOWN: [u8; 3] = [ESCAPE_KEY, b'[', b'B'];
const PROMPT: &str = "> ";

/* -------------------------------------------------------------------------- */
/*                             Struct Declaration                             */
/* -------------------------------------------------------------------------- */
#[derive(Default)]
pub struct Cli {
    line: String,
    history: History,
}

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Cli {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_line(&mut self) -> Result<String, TaskmasterError> {
        let origin_termios = Self::enable_raw_mode()?;
        Self::display_prompt()?;
        self.history.push(String::new());
        let _ = self.history.restore();
        let mut input = Self::getch()?;
        while !(input.len() == 1 && input[0] == b'\n') {
            self.handle_input(input)?;
            input = Self::getch()?;
        }
        println!();
        if !self.line.is_empty() {
            let _ = self.history.set_last_line(self.line.clone());
        } else {
            let _ = self.history.pop();
        }
        let return_line = self.line.clone();
        self.line.clear();
        tcl::mylibc::disable_raw_mode(origin_termios)?;
        Ok(return_line)
    }

    /// Enable raw mode to read single keypresses without waiting for Enter
    fn enable_raw_mode() -> Result<tcl::mylibc::Termios, std::io::Error> {
        let fd = io::stdin().as_raw_fd();
        let mut termios = tcl::mylibc::get_terminal_attributes(fd)?;

        let orig_termios = termios;
        // Disable canonical mode and echo
        termios.c_lflag &= !(ICANON | ECHO);
        // Apply changes immediately
        tcl::mylibc::set_terminal_attributes(fd, TCSANOW, &termios)?;

        Ok(orig_termios)
    }

    /// Function to read a single keypress, including escape sequences
    fn getch() -> Result<Vec<u8>, TaskmasterError> {
        let stdin = io::stdin();
        let mut buffer = vec![0; 3];
        stdin.lock().read_exact(&mut buffer[..1])?;

        if buffer[0] == ESCAPE_KEY {
            stdin.lock().read_exact(&mut buffer[1..3])?;
        } else {
            buffer.truncate(1);
        }
        Ok(buffer)
    }

    fn handle_input(&mut self, input: Vec<u8>) -> Result<(), TaskmasterError> {
        if input.len() == 1 {
            self.handle_character_input(input[0])?;
        } else {
            self.handle_sequence_key(input)?;
        }
        Ok(())
    }

    fn handle_character_input(&mut self, ch: u8) -> Result<(), TaskmasterError> {
        if ch.is_ascii_graphic() || ch == b' ' {
            print!("{}", ch as char);
            self.line.push(ch as char);
        } else if ch == BACKSPACE && !self.line.is_empty() {
            self.line.pop();
            print!("{CLEAR_CHAR}");
        }
        if self.history.is_last_line() {
            let _ = self.history.set_last_line(self.line.clone());
        }
        io::stdout().flush()?;
        Ok(())
    }

    fn handle_sequence_key(&mut self, input: Vec<u8>) -> Result<(), TaskmasterError> {
        if let Ok(sequence) = input.try_into() as Result<[u8; 3], _> {
            match sequence {
                ARROW_UP => {
                    let _ = self.history.backward();
                }
                ARROW_DOWN => {
                    let _ = self.history.forward();
                }
                _ => {}
            }
            if let Some(line) = self.history.get_current_line() {
                self.line = line;
                self.refresh_prompt()?;
            }
        }
        Ok(())
    }

    fn refresh_prompt(&self) -> Result<(), TaskmasterError> {
        print!("{}", CLEAR_LINE);
        print!("{}", RESET_CURSOR);
        print!("{}", PROMPT);
        print!("{}", self.line);
        io::stdout().flush()?;
        Ok(())
    }

    fn display_prompt() -> Result<(), TaskmasterError> {
        print!("{}", PROMPT);
        io::stdout().flush()?;
        Ok(())
    }
}
