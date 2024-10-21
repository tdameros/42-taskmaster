/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use command::CliCommand;
use std::io::{self, stdin, Write};
use tcl::SOCKET_ADDRESS;
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod command;

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
    CliCommand::help(); // display the cli manual

    // the actual cli
    loop {
        // terminal prompt
        print!("> ");
        io::stdout().flush().expect("Error while flushing stdout");

        // acquiring user input
        let mut user_input = String::new();
        if let Err(input_error) = stdin().read_line(&mut user_input) {
            eprintln!("Error Occurred while reading user input: {input_error}, please close the terminal and restart the client");
            return; // we need to close the program if this happen
        }
        let trimmed_user_input = user_input.trim().to_owned();

        // executing the client order
        match CliCommand::from_client_input(trimmed_user_input.as_str()) {
            Ok(command) => {
                if let Err(error) = command.execute(&mut stream).await {
                    eprintln!("error while executing command: {error}");
                }
            }
            Err(e) => {
                eprintln!("error while parsing command: {e}, tap 'help' for more info about available command or exit to 'close'");
            }
        };
    }
}
