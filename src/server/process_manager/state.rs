/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Process, ProcessError, ProcessState};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Process {
    pub(super) fn update_starting(&mut self, code: Result<Option<i32>, ProcessError>) {
        match code {
            // the program is no longer running
            Ok(Some(code)) => match self.is_no_longer_starting() {
                Ok(true) => match self.config.expected_exit_code.contains(&code) {
                    true => self.state = ProcessState::Exited,
                    false => self.state = ProcessState::Stopped,
                },
                Ok(false) => self.state = ProcessState::Backoff,
                Err(_) => unreachable!(),
            },
            // the program is still running
            Ok(None) => match self.is_no_longer_starting() {
                Ok(true) => self.state = ProcessState::Running,
                Ok(false) => {}
                Err(_) => unreachable!(),
            },
            // we don't know the state of the child anymore
            Err(_) => self.state = ProcessState::Unknown,
        };
    }

    pub(super) fn update_running(&mut self, code: Result<Option<i32>, ProcessError>) {
        match code {
            // the program is not running anymore
            Ok(Some(code)) => match self.config.expected_exit_code.contains(&code) {
                true => self.state = ProcessState::Exited,
                false => self.state = ProcessState::Stopped,
            },
            // the program is still running
            Ok(None) => {}
            // we don't know the state of the child anymore
            Err(_) => self.state = ProcessState::Unknown,
        };
    }

    pub(super) fn update_stopping(&mut self, code: Result<Option<i32>, ProcessError>) {
        match code {
            Ok(Some(_)) => {
                // the program is not running anymore
                self.state = ProcessState::Stopped;
            }
            Ok(None) => {
                // the program is still running
            }
            // we don't know the state of the child anymore
            Err(_) => self.state = ProcessState::Unknown,
        };
    }

    pub(super) fn react_never_started_yet(&mut self) {
        if self.config.start_at_launch {
            self.start();
        }
    }

    pub(super) fn react_stopped(&mut self) {
        self.clean_child();
    }

    pub(super) fn react_backoff(&mut self) {
        use std::cmp::Ordering as O;
        match self.number_of_restart.cmp(&self.config.max_number_of_restart) {
            O::Less => {
                self.clean_child();
                match self.start() {
                    Ok(_) => self.number_of_restart += 1,
                    Err(_) => self.state = ProcessState::Fatal,
                };
            },
            O::Equal |
            O::Greater => {
                self.state = ProcessState::Fatal;
                self.clean_child();
            },
        };
    }
}
