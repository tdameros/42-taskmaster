/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use cli::Cli;
use command::Command;
use tcl::SOCKET_ADDRESS;
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */

mod cli;
mod command;
mod history;
/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */

#[tokio::main]
async fn main() {
    // connect to the server
    println!("Trying to connect to the server");
    let mut stream = TcpStream::connect(SOCKET_ADDRESS)
        .await
        .expect("Can't Connect to the server");
    Command::help(); // display the cli manual
    let mut shell = Cli::new();
    loop {
        match shell.read_line() {
            Ok(user_input) => {
                process_user_input(user_input, &mut stream).await;
            }
            Err(error) => {
                eprintln!("Error reading line: {}", error);
                return;
            }
        }
    }
}

async fn process_user_input(user_input: String, stream: &mut TcpStream) {
    let trimmed_user_input = user_input.trim().to_owned();

    if trimmed_user_input.is_empty() {
        return;
    }

    match Command::from_client_input(trimmed_user_input.as_str()) {
        Ok(command) => {
            if let Err(error) = command.execute(stream).await {
                eprintln!("Error while executing command: {error}");
            }
        }
        Err(error) => {
            eprintln!("Error while parsing command: {error}. Type 'help' for more info or 'exit' to close.");
        }
    }
}
