/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use super::{Process, ProcessError, ProcessState};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Process {
    pub(super) fn update_starting(&mut self, code: Option<i32>) {
        match code {
            // the program is no longer running
            Some(code) => {
                match self.is_no_longer_starting() {
                    Ok(true) => {
                        match self.config.expected_exit_code.contains(&code) {
                            true => self.state = ProcessState::ExitedExpectedly,
                            false => self.state = ProcessState::ExitedUnExpectedly,
                        };
                    }
                    Ok(false) => self.state = ProcessState::Backoff,
                    Err(_) => unreachable!(),
                };
                self.clean_child();
            }
            // the program is still running
            None => match self.is_no_longer_starting() {
                Ok(true) => self.state = ProcessState::Running,
                Ok(false) => {}
                Err(_) => unreachable!(),
            },
        };
    }

    pub(super) fn update_running(&mut self, code: Option<i32>) {
        match code {
            // the program is not running anymore
            Some(code) => {
                match self.config.expected_exit_code.contains(&code) {
                    true => self.state = ProcessState::Exited,
                    false => self.state = ProcessState::Stopped,
                };
                self.clean_child();
            }
            // the program is still running
            None => {}
        };
    }

    pub(super) fn update_stopping(&mut self, code: Option<i32>) {
        match code {
            Some(_) => {
                // the program is not running anymore
                self.state = ProcessState::Stopped;
                self.clean_child();
            }
            None => {
                // the program is still running
            }
        };
    }

    pub(super) fn react_never_started_yet(&mut self) -> Result<(), ProcessError> {
        if self.config.start_at_launch {
            self.start()?;
        }

        Ok(())
    }

    pub(super) fn react_stopped(&mut self) -> Result<(), ProcessError> {
        self.clean_child();

        Ok(())
    }

    pub(super) fn react_backoff(&mut self) -> Result<(), ProcessError> {
        use std::cmp::Ordering as O;
        match self
            .number_of_restart
            .cmp(&self.config.max_number_of_restart)
        {
            O::Less => {
                self.clean_child();
                self.start()
                    .map(|_| self.number_of_restart += 1)
                    .inspect_err(|_| self.state = ProcessState::Fatal)?;
            }
            O::Equal | O::Greater => {
                self.state = ProcessState::Fatal;
                self.clean_child();
            }
        };

        Ok(())
    }

    pub(super) fn react_stopping(&mut self) -> Result<(), ProcessError> {
        if self.its_time_to_kill_the_child() {
            self.kill()?;
        };

        Ok(())
    }

    pub(super) fn react_exited(&mut self) -> Result<(), ProcessError> {
        use crate::config::AutoRestart as aR;
        match self.config.auto_restart {
            aR::Always => {
                self.start()?;
            }
            aR::Unexpected | aR::Never => {}
        }

        Ok(())
    }
}
