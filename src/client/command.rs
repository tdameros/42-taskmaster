/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::ops::Deref;

use tcl::{
    error::TaskmasterError,
    message::{send, Request},
};
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
pub enum CliCommand {
    Request(Request),
    Exit,
    Help,
}
impl CliCommand {
    pub fn from_client_input(user_input: &str) -> Result<CliCommand, TaskmasterError> {
        let arguments: Vec<&str> = user_input.split_ascii_whitespace().collect();
        if arguments.len() > 2 {
            return Err(TaskmasterError::Custom(format!(
                "`{}` contain to many arguments",
                user_input
            )));
        } else if arguments.is_empty() {
            return Err(TaskmasterError::Custom(
                "your command contain nothing".to_owned(),
            ));
        }
        let command = arguments
            .first()
            .expect("unreachable")
            .to_ascii_lowercase()
            .to_owned();
        let cli_command = if arguments.len() == 1 {
            match command.deref() {
                "exit" => CliCommand::Exit,
                "help" => CliCommand::Help,
                "status" => CliCommand::Request(Request::Status),
                "reload" => CliCommand::Request(Request::Reload),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        } else {
            let argument = arguments.get(1).expect("unreachable").to_ascii_lowercase();
            match command.deref() {
                "start" => CliCommand::Request(Request::Start(argument.to_owned())),
                "stop" => CliCommand::Request(Request::Stop(argument.to_owned())),
                "restart" => CliCommand::Request(Request::Restart(argument.to_owned())),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        };
        Ok(cli_command)
    }

    pub async fn execute(&self, stream: &mut TcpStream) -> Result<(), TaskmasterError> {
        match self {
            CliCommand::Exit => {
                CliCommand::exit();
                Ok(())
            }
            CliCommand::Help => {
                CliCommand::help();
                Ok(())
            }
            CliCommand::Request(request) => {
                Ok(CliCommand::forward_to_server(request, stream).await?)
            }
        }
    }

    pub fn exit() {
        std::process::exit(0);
    }

    pub fn help() {
        println!(
            "Taskmaster Client Commands:

    status [PROGRAM]    Get the status of all the programs
    start [PROGRAM]     Start a program
    stop [PROGRAM]      Stop a program
    restart [PROGRAM]   Restart a program
    reload              Reload configuration file
    exit                Exit client shell
    help                Show this help message"
        )
    }

    async fn forward_to_server(
        request: &Request,
        stream: &mut TcpStream,
    ) -> Result<(), TaskmasterError> {
        send(stream, request).await?;
        Ok(())
    }
}
