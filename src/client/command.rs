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
/// this enum represent the set of all possible command that the client can receive
pub enum CliCommand {
    Request(Request),
    Exit,
    Help,
}

impl CliCommand {
    /// Try to produce a CliCommand enum based on the user input,
    /// returning the appropriate error if enable
    pub fn from_client_input(user_input: &str) -> Result<CliCommand, TaskmasterError> {
        // collect the user input into a vector for ease of processing
        let arguments: Vec<&str> = user_input.split_ascii_whitespace().collect();

        // check if too many or too little argument are present
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

        // extract the first command from the user input
        let command = arguments
            .first()
            .expect("unreachable")
            .to_ascii_lowercase()
            .to_owned();

        // construct the CliCommand struct base on whenever there are only 1 or two word in the user input
        let cli_command = if arguments.len() == 1 {
            // try to match against command that need no argument
            match command.deref() {
                "exit" => CliCommand::Exit,
                "help" => CliCommand::Help,
                "status" => CliCommand::Request(Request::Status),
                "reload" => CliCommand::Request(Request::Reload),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        } else {
            // get the argument
            let argument = arguments.get(1).expect("unreachable").to_ascii_lowercase();
            // try to match against command that require one argument
            match command.deref() {
                "start" => CliCommand::Request(Request::Start(argument.to_owned())),
                "stop" => CliCommand::Request(Request::Stop(argument.to_owned())),
                "restart" => CliCommand::Request(Request::Restart(argument.to_owned())),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        };

        Ok(cli_command)
    }

    /// This Function will match the command and execute it properly
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

    /// process the Exit command
    pub fn exit() {
        std::process::exit(0);
    }

    /// Process the Help Command and Display the Cli command and argument
    pub fn help() {
        println!(
            "Taskmaster Client/server architecture Commands:

            status              Get the status of all the programs
            start [PROGRAM]     Start a program
            stop [PROGRAM]      Stop a program
            restart [PROGRAM]   Restart a program
            reload              Reload configuration file
            exit                Exit client shell
            help                Show this help message

        "
        )
    }

    /// process the request command
    async fn forward_to_server(
        request: &Request,
        stream: &mut TcpStream,
    ) -> Result<(), TaskmasterError> {
        send(stream, request).await?;
        Ok(())
    }
}
