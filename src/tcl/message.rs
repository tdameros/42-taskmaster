/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use crate::{error::TaskmasterError, MAX_MESSAGE_SIZE};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                               Message Struct                               */
/* -------------------------------------------------------------------------- */
#[derive(Serialize, serde::Deserialize)]
pub enum Message {
    Test(String),
}

pub struct Program {
    pub pid: u32,
}

/* -------------------------------------------------------------------------- */
/*                                  Function                                  */
/* -------------------------------------------------------------------------- */
pub async fn send_message<'a, T: Serialize>(
    stream: &mut TcpStream,
    message: &T,
) -> Result<(), TaskmasterError> {
    let serialized_message =
        serde_yaml::to_string(message).map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;

    let length = serialized_message.len();
    if length as u32 > MAX_MESSAGE_SIZE {
        return Err(TaskmasterError::MessageTooLong);
    }
    let length_in_byte = (length as u32).to_be_bytes();

    stream.write_all(&length_in_byte).await?;
    stream.write_all(serialized_message.as_bytes()).await?;

    Ok(())
}

pub async fn receive_message<T: for<'a> Deserialize<'a>>(
    stream: &mut TcpStream,
) -> Result<T, TaskmasterError> {
    let mut length_bytes = [0u8; 4];
    stream.read_exact(&mut length_bytes).await?;
    let message_length = u32::from_be_bytes(length_bytes) as usize;

    if message_length as u32 > MAX_MESSAGE_SIZE {
        return Err(TaskmasterError::MessageTooLong);
    }

    let mut buffer = vec![0u8; message_length];
    stream.read_exact(&mut buffer).await?;

    let yaml_string =
        String::from_utf8(buffer).map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;
    let received_message: T = serde_yaml::from_str(&yaml_string)
        .map_err(|e| TaskmasterError::SerdeError(e.to_string()))?;
    Ok(received_message)
}
