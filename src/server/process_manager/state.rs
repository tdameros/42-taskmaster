/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use super::{Process, ProcessError};

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Process {
    pub(super) fn process_stopped(&mut self, code: Result<Option<i32>, ProcessError>) {
        match code {
            Ok(Some(code)) => todo!(),
            Ok(None) => todo!(),
            Err(error) => todo!(),
        }
    }
}