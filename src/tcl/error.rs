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
    SerdeError(serde_yaml::Error),
    StringConversionError(std::string::FromUtf8Error),
    Custom(String), // this will disappear over time
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
            TaskmasterError::StringConversionError(e) => write!(f, "String Conversion Error: {e}"),
        }
    }
}

impl TaskmasterError {
    /// Return whenever an error is due to a client disconnecting
    pub fn client_disconnected(&self) -> bool {
        match self {
            TaskmasterError::IoError(error) => match error.kind() {
                std::io::ErrorKind::UnexpectedEof => true,
                _ => false,
            },
            _ => false,
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                             From Implementation                            */
/* -------------------------------------------------------------------------- */
impl From<std::io::Error> for TaskmasterError {
    fn from(error: std::io::Error) -> Self {
        TaskmasterError::IoError(error)
    }
}

impl From<serde_yaml::Error> for TaskmasterError {
    fn from(error: serde_yaml::Error) -> Self {
        TaskmasterError::SerdeError(error)
    }
}

impl From<std::string::FromUtf8Error> for TaskmasterError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        TaskmasterError::StringConversionError(error)
    }
}
