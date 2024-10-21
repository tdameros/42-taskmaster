/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use command::CliCommand;
use shell::CliShell;
use tcl::SOCKET_ADDRESS;
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */

mod command;
mod shell;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

#[tokio::main]
async fn main() {
    println!("Trying to connect to the server");
    let mut stream = TcpStream::connect(SOCKET_ADDRESS)
        .await
        .expect("Can't Connect to the server");

    CliCommand::help();
    let mut shell = CliShell::new();
    loop {
        let user_input = shell.read_line();
        let trimmed_user_input = user_input.trim().to_owned();

        match CliCommand::from_client_input(trimmed_user_input.as_str()) {
            Ok(command) => {
                if let Err(error) = command.execute(&mut stream).await {
                    eprintln!("error while executing command: {error}");
                }
            }
            Err(e) => {
                eprintln!("error while parsing command: {e}");
            }
        };
    }
}
