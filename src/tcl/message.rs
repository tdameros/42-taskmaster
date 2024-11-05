/*!
 * This Module is responsible for the transport of message (Serialization and deserialization)
 * and provide a unify interface for all binary needing to use it with two generic function
 * send and receive, it use it's own protocol to control the length of a given message,
 * those should not exceed 1 MB. This module also provide a unify place for the common used struct
 * during message exchange. it was decided that the protocol expect a response after a request no matter what
 * so a client should expect to receive a response after a request
 */
/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use crate::{error::TaskmasterError, MAX_MESSAGE_SIZE};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, time::SystemTime};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/* -------------------------------------------------------------------------- */
/*                               Message Struct                               */
/* -------------------------------------------------------------------------- */
/// Represent what can be send to the client as a response
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Success(String),
    Error(String),
    Status(Vec<ProgramStatus>),
}

/// Represent what can be send to the server as request
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Status,
    Start(String),
    Stop(String),
    Restart(String),
    Reload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgramStatus {
    pub name: String,
    pub status: Vec<ProcessStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessStatus {
    pub status: ProcessState,
    pub pid: Option<u32>,
    pub start_time: Option<SystemTime>,
    pub shutdown_time: Option<SystemTime>,
    pub number_of_restart: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ProcessState {
    /// the default state, has never been started.
    NeverStartedYet,

    /// The process has been stopped due to a stop request
    Stopped,

    /// The process is starting due to a start request.
    Starting,

    /// The process is running.
    Running,

    /// The process entered the Starting state but subsequently exited too quickly
    /// (before the time defined in time_to_start) to move to the Running state.
    Backoff,

    /// The process is stopping due to a stop request.
    Stopping,

    /// The process exited from the RUNNING state expectedly.
    ExitedExpectedly,

    /// The process exited from the RUNNING state unexpectedly.
    ExitedUnExpectedly,

    /// The process could not be started successfully.
    Fatal,

    /// The process is in an unknown state (error while getting the exit status).
    Unknown,
}

/* -------------------------------------------------------------------------- */
/*                                  Function                                  */
/* -------------------------------------------------------------------------- */
/// write the message to the socket returning an error if it fails
pub async fn send<'a, T: Serialize>(
    stream: &mut TcpStream,
    message: &T,
) -> Result<(), TaskmasterError> {
    // serialize the message
    let serialized_message = serde_yaml::to_string(message)?;

    // check the message length and transform the length to send it with the message
    let length = serialized_message.len();
    if length as u32 > MAX_MESSAGE_SIZE {
        return Err(TaskmasterError::MessageTooLong);
    }
    let length_in_byte = (length as u32).to_be_bytes();

    // write the message to the socket preceded by it's length
    stream.write_all(&length_in_byte).await?;
    stream.write_all(serialized_message.as_bytes()).await?;

    Ok(())
}

/// receive a message and try to deserialize it into the type T
pub async fn receive<T: for<'a> Deserialize<'a>>(
    stream: &mut TcpStream,
) -> Result<T, TaskmasterError> {
    // get the length of the incoming message and check if the message can be received
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let message_length = u32::from_be_bytes(length_bytes) as usize;
    if message_length as u32 > MAX_MESSAGE_SIZE {
        return Err(TaskmasterError::MessageTooLong);
    }

    // read the rest of the message
    let mut buffer = vec![0u8; message_length];
    stream.read_exact(&mut buffer).await?;

    // deserialize the message into the demanded struct
    let yaml_string = String::from_utf8(buffer)?;
    let received_message: T = serde_yaml::from_str(&yaml_string)?;

    // return the message if everything went right
    Ok(received_message)
}

/* -------------------------------------------------------------------------- */
/*                           Display Implementation                           */
/* -------------------------------------------------------------------------- */
impl Display for ProgramStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
