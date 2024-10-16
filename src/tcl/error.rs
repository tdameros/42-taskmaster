/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::error::Error;

/* -------------------------------------------------------------------------- */
/*                              Struct Definition                             */
/* -------------------------------------------------------------------------- */
#[derive(Debug)]
pub enum TaskmasterError {
    IoError(std::io::Error),
    SerdeError(String), // to be define
    Custom(String),
    MessageTooLong,
}

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Error for TaskmasterError {}

impl std::fmt::Display for TaskmasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskmasterError::IoError(e) => write!(f, "IO error: {}", e),
            TaskmasterError::SerdeError(e) => write!(f, "Serialization error: {e}"),
            TaskmasterError::MessageTooLong => write!(f, "Message exceeds maximum length"),
            TaskmasterError::Custom(e) => write!(f, "Error: {e}"),
        }
    }
}

impl TaskmasterError {
    pub fn kind(&self) -> std::io::ErrorKind {
        match self {
            TaskmasterError::IoError(err) => err.kind(),
            _ => std::io::ErrorKind::Other,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                             From Implementation                            */
/* -------------------------------------------------------------------------- */
impl From<std::io::Error> for TaskmasterError {
    fn from(value: std::io::Error) -> Self {
        TaskmasterError::IoError(value)
    }
}

/* -------------------------------------------------------------------------- */
/*                               Common Function                              */
/* -------------------------------------------------------------------------- */
