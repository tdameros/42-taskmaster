/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use tcl::{error::TaskmasterError, message::{Request, send}};
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
    pub fn from_client_input(user_input: String) -> Result<CliCommand, TaskmasterError> {
        user_input.split_ascii_whitespace();
        todo!()
    }

    pub async fn execute(&self, stream: &mut TcpStream) {
        match self {
            CliCommand::Exit => CliCommand::exit(),
            CliCommand::Help => CliCommand::help(),
            CliCommand::Request(request) => { CliCommand::forward_to_server(request, stream).await; },
        }
    }

    pub fn exit() {
        std::process::exit(0);
    }

    pub fn help() {
        println!("
        Taskmaster Client Commands:

            status [PROGRAM]    Get the status of all the programs
            start [PROGRAM]     Start a program
            stop [PROGRAM]      Stop a program
            restart [PROGRAM]   Restart a program
            reload              Reload configuration file
            exit                Exit client shell
            help                Show this help message

        ")
    }
    
    async fn forward_to_server(request: &Request, stream: &mut TcpStream) -> Result<(), TaskmasterError> {
       send(stream, request).await?;
       todo!()
    }

}
