/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

/* -------------------------------------------------------------------------- */
/*                                  Constant                                  */
/* -------------------------------------------------------------------------- */
const LOG_PATH: &str = "./log.txt";

/* -------------------------------------------------------------------------- */
/*                             Struct Declaration                             */
/* -------------------------------------------------------------------------- */
pub(super) struct Logger {
    file: RwLock<File>,
}

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
pub(super) type SharedLogger = Arc<Logger>;

impl Logger {
    /// open a log file specified by the LOG_PATH constant, creating it if it doesn't exist
    /// appending to it if it does.
    pub(super) fn new() -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_PATH)?;
        Ok(Logger {
            file: RwLock::new(file),
        })
    }

    /// write the message to the logging file
    pub(super) fn log(&self, level: &str, message: &str) -> Result<(), std::io::Error> {
        // get the time since unix epoch to be reworked for better formatting
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the time returned by SystemTime::now() is earlier than UNIX_EPOCH")
            .as_secs();

        // format the log
        let log_entry = format!("[{}] {} - {}\n", timestamp, level, message);

        // write the log to the file
        let mut file = self.file.write().unwrap();
        file.write_all(log_entry.as_bytes())?;
        file.flush()?;

        Ok(())
    }
}

pub(crate) fn new_shared_logger() -> Result<SharedLogger, std::io::Error> {
    Ok(Arc::new(Logger::new()?))
}

/* -------------------------------------------------------------------------- */
/*                                    Macro                                   */
/* -------------------------------------------------------------------------- */
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log("DEBUG", &format!($($arg)*)).unwrap_or_else(|e| eprintln!("Logging error: {}", e));
    }
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log("INFO", &format!($($arg)*)).unwrap_or_else(|e| eprintln!("Logging error: {}", e));
    }
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.log("ERROR", &format!($($arg)*)).unwrap_or_else(|e| eprintln!("Logging error: {}", e));
    }
}
