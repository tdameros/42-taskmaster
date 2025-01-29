/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use cli::Cli;
use command::Command;
use std::time::Duration;
use tcl::message::{receive, send, Request, Response};
use tcl::SOCKET_ADDRESS;
use tokio::net::TcpStream;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::sleep;
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
    let mut stream = loop {
        match TcpStream::connect(SOCKET_ADDRESS).await {
            Ok(stream) => {
                break stream;
            }
            Err(e) => {
                eprintln!("can't connect: {e}");
                sleep(Duration::from_secs(2)).await;
            }
        }
    };

    // disable CTRL+C (SIGINT)
    let _ = signal(SignalKind::interrupt()).expect("Failed to create signal");

    Command::help(); // display the cli manual
    let mut shell = Cli::new();
    loop {
        match shell.read_line() {
            Ok(user_input) => {
                let result = process_user_input(user_input, &mut stream).await;
                if let Some((Command::Request(Request::Attach(_)), Response::Success(_))) = result {
                    receive_attach(&mut stream).await;
                }
            }
            Err(error) => {
                if error.is_unexpected_end_of_file() {
                    println!();
                } else {
                    eprintln!("Error reading line: {}", error);
                }
                return;
            }
        }
    }
}

async fn process_user_input(
    user_input: String,
    stream: &mut TcpStream,
) -> Option<(Command, Response)> {
    let trimmed_user_input = user_input.trim().to_owned();

    if trimmed_user_input.is_empty() {
        return None;
    }

    match Command::try_from(trimmed_user_input.as_str()) {
        Ok(command) => match command.execute(stream).await {
            Ok(response) => Some((command, response)),
            Err(error) => {
                eprintln!("Error while executing command: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("Error while parsing command: {error}. Type 'help' for more info or 'exit' to close.");
            None
        }
    }
}

async fn receive_attach(stream: &mut TcpStream) {
    let mut signal = signal(SignalKind::interrupt()).expect("Failed to create signal");
    loop {
        select! {
            response = receive::<Response>(stream) => {

                match response {
                    Ok(result) => match result {
                        Response::Error(result) => {
                            print!("{result}");
                            return;
                        },
                        result => {
                            print!("{result}");
                        }
                    },
                    Err(error) => {
                        println!("{error}");
                        break;
                    }
                }
            },

            _ = signal.recv() => {
                let detach = Request::Detach;
                match send::<Request>(stream, &detach).await {
                    Ok(_) => {
                        match receive::<Response>(stream).await {
                            Ok(response) => {
                                print!("{response}");
                            }
                            Err(error) => {
                                eprintln!("{error}");
                            }
                        }
                        break;
                    }
                    Err(error) => {
                        eprintln!("Failed to detach: {error}");
                    }
                }
            }
        }
    }
}
