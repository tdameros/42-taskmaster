/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io;
use command::CliCommand;
use std::io::{stdin, Write};
use tcl::{
    message::{send, Response},
    SOCKET_ADDRESS,
};
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
    println!("Trying to connect to the server");
    let mut stream = TcpStream::connect(SOCKET_ADDRESS)
        .await
        .expect("Can't Connect to the server");

    CliCommand::help();
    loop {
        print!("> ");
        io::stdout().flush().expect("Error while flushing stdout");
        let mut user_input = String::new();
        if let Err(input_error) = stdin().read_line(&mut user_input) {
            eprintln!("Error Occurred while reading user input: {input_error}, please close the terminal and restart the client");
        }
        let trimmed_user_input = user_input.trim().to_owned();

        match CliCommand::from_client_input(trimmed_user_input.as_str()) {
            Ok(command) => {
                if let Err(error) = command.execute(&mut stream).await {
                    eprintln!("error while parsing command: {error}");
                    todo!()
                }
            }
            Err(e) => {
                eprintln!("error while parsing command: {e}");
                CliCommand::help();
            }
        };

        if let Err(e) = send(&mut stream, &Response::Test(trimmed_user_input.to_owned())).await {
            eprintln!("Error while sending message to server: {e}");
        }
    }
}
