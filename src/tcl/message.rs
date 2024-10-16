/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{error::TaskmasterError, MAX_MESSAGE_SIZE};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/* -------------------------------------------------------------------------- */
/*                               Message Struct                               */
/* -------------------------------------------------------------------------- */
/// used to send info from the server to a client
#[derive(Serialize, serde::Deserialize)]
pub enum Response {
    Test(String)
}

#[derive(Serialize, Deserialize)]
struct Start {}
/// used to send order from a client to the server
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    Status,
    Start(String),
    Stop(String),
    Restart(String),
    Reload,
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
    let serialized_message =
        serde_yaml::to_string(message).map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;

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
    let yaml_string =
        String::from_utf8(buffer).map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;
    let received_message: T = serde_yaml::from_str(&yaml_string)
        .map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;

    // return the message if everything went right
    Ok(received_message)
}
