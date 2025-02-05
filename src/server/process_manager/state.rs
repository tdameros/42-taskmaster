/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::better_logs::send_http_message;
#[cfg(feature = "reqwest")]
use crate::better_logs::send_notification;

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
                    Some(true) => {
                        match self.config.expected_exit_code.contains(&code) {
                            true => self.state = ProcessState::ExitedExpectedly,
                            false => self.state = ProcessState::ExitedUnExpectedly,
                        };
                    }
                    Some(false) => self.state = ProcessState::Backoff,
                    None => unreachable!(),
                };
                self.clean_child();
            }
            // the program is still running
            None => match self.is_no_longer_starting() {
                Some(true) => self.state = ProcessState::Running,
                Some(false) => {}
                None => unreachable!(),
            },
        };
    }

    pub(super) fn update_running(&mut self, code: Option<i32>) {
        if let Some(code) = code {
            match self.config.expected_exit_code.contains(&code) {
                true => self.state = ProcessState::ExitedExpectedly,
                false => self.state = ProcessState::ExitedUnExpectedly,
            };
            self.clean_child();
        }
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

    pub(super) fn update_unknown(&mut self, code: Option<i32>) {
        match code {
            Some(code) => {
                match self.config.expected_exit_code.contains(&code) {
                    true => self.state = ProcessState::ExitedExpectedly,
                    false => self.state = ProcessState::ExitedUnExpectedly,
                };
                self.clean_child();
            }
            None => match self.is_no_longer_starting() {
                Some(true) => self.state = ProcessState::Running,
                Some(false) => self.state = ProcessState::Starting,
                None => unreachable!(),
            },
        }
    }

    pub(super) async fn react_never_started_yet(&mut self) -> Result<(), ProcessError> {
        if self.config.start_at_launch {
            self.start().await?;
        }

        Ok(())
    }

    pub(super) async fn react_backoff(&mut self, program_name: &str) -> Result<(), ProcessError> {
        use std::cmp::Ordering as O;
        match self
            .number_of_restart
            .cmp(&self.config.max_number_of_restart)
        {
            O::Less => match self.start().await {
                Ok(_) => self.number_of_restart += 1,
                Err(e) => {
                    self.number_of_restart += 1;
                    return Err(e);
                }
            },
            O::Equal | O::Greater => {
                if !self.config.fatal_state_report_address.is_empty() {
                    send_http_message(
                        self.config.fatal_state_report_address.to_owned(),
                        format!("one process of {program_name} could not be launch successfully"),
                    );
                }
                #[cfg(feature = "reqwest")]
                let token = std::env::var("API_KEY").unwrap_or_default();
                #[cfg(feature = "reqwest")]
                if !token.is_empty() {
                    send_notification(
                        token,
                        program_name.to_owned(),
                        "didn't start successfully".to_owned(),
                    )
                    .await;
                }
                self.state = ProcessState::Fatal;
            }
        };

        Ok(())
    }

    pub(super) async fn react_stopping(&mut self) -> Result<(), ProcessError> {
        if self.its_time_to_kill_the_child() {
            self.kill().await?;
        };

        Ok(())
    }

    pub(super) async fn react_expected_exit(&mut self) -> Result<(), ProcessError> {
        use crate::config::AutoRestart as AR;
        match self.config.auto_restart {
            AR::Always => self.start().await,
            AR::Unexpected | AR::Never => Ok(()),
        }
    }

    pub(super) async fn react_unexpected_exit(&mut self) -> Result<(), ProcessError> {
        use crate::config::AutoRestart as AR;
        match self.config.auto_restart {
            AR::Always | AR::Unexpected => self.start().await,
            AR::Never => Ok(()),
        }
    }
}
