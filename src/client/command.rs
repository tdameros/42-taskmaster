/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use std::ops::Deref;
use tcl::message::{receive, Response};
use tcl::{
    error::TaskmasterError,
    message::{send, Request},
};
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                             Struct Declaration                             */
/* -------------------------------------------------------------------------- */
/// this enum represent the set of all possible command that the client can receive
pub enum Command {
    Request(Request),
    Exit,
    Help,
}

/* -------------------------------------------------------------------------- */
/*                            Struct Implementation                           */
/* -------------------------------------------------------------------------- */
impl Command {
    /// This Function will match the command and execute it properly
    pub async fn execute(&self, stream: &mut TcpStream) -> Result<Response, TaskmasterError> {
        match self {
            Command::Exit => {
                Command::exit();
                Ok(Response::Success(String::from("Success exit")))
            }
            Command::Help => {
                Command::help();
                Ok(Response::Success(String::from("Success help")))
            }
            Command::Request(request) => {
                Command::forward_to_server(request, stream).await?;
                let response: Result<Response, TaskmasterError> = receive(stream).await;
                match response {
                    Ok(result) => {
                        print!("{result}");
                        Ok(result)
                    }
                    Err(error) => {
                        println!("{error}");
                        Err(error)
                    }
                }
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

/* -------------------------------------------------------------------------- */
/*                            Trait Implementation                            */
/* -------------------------------------------------------------------------- */
impl TryFrom<&str> for Command {
    type Error = TaskmasterError;

    fn try_from(user_input: &str) -> Result<Self, Self::Error> {
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
                "exit" => Command::Exit,
                "help" => Command::Help,
                "status" => Command::Request(Request::Status),
                "reload" => Command::Request(Request::Reload),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        } else {
            // get the argument
            let argument = arguments.get(1).expect("unreachable").to_ascii_lowercase();
            // try to match against command that require one argument
            match command.deref() {
                "start" => Command::Request(Request::Start(argument.to_owned())),
                "stop" => Command::Request(Request::Stop(argument.to_owned())),
                "restart" => Command::Request(Request::Restart(argument.to_owned())),
                "attach" => Command::Request(Request::Attach(argument.to_owned())),
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        };

        Ok(cli_command)
    }
}
