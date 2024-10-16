/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io::stdin;
use tcl::{
    message::{send, Response},
    SOCKET_ADDRESS,
};
use tokio::net::TcpStream;
use command::CliCommand;

/* -------------------------------------------------------------------------- */
/*                                   Module                                   */
/* -------------------------------------------------------------------------- */
mod command;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    println!("Trying to connect to the server");
    let mut stream = TcpStream::connect(SOCKET_ADDRESS)
        .await
        .expect("Can't Connect to the server");

    loop {
        let mut user_input = String::new();
        if let Err(input_error) = stdin().read_line(&mut user_input) {
            eprintln!("Error Occurred while reading user input: {input_error}, please close the terminal and restart the client");
        }
        let trimmed_user_input = user_input.trim().to_owned();

        if trimmed_user_input.eq_ignore_ascii_case("quit") {
            // here we want to replace this with a match to se what command the user is tell
            break;
        }
        // let command = match CliCommand::from_client_input(trimmed_user_input) {
            // Ok(command) => command.execute(),
        //     Err(e) => {
        //         eprintln!("error while parsing command");
        //         CliCommand::help();
        //     }
        // };

        if let Err(e) = send(&mut stream, &Response::Test(trimmed_user_input.to_owned())).await {
            eprintln!("Error while sending message to server: {e}");
        }
    }
}
