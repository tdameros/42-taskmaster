/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use std::collections::HashMap;
use std::ops::Deref;
use std::time::{Duration, SystemTime};
use tcl::message::{receive, ProcessState, ProcessStatus, Response};
use tcl::{
    error::TaskmasterError,
    message::{send, Request},
};
use tokio::net::TcpStream;
/* -------------------------------------------------------------------------- */
/*                                   Struct                                   */
/* -------------------------------------------------------------------------- */
/// this enum represent the set of all possible command that the client can receive
pub enum Command {
    Request(Request),
    Exit,
    Help,
}

impl Command {
    /// Try to produce a CliCommand enum based on the user input,
    /// returning the appropriate error if enable
    pub fn from_client_input(user_input: &str) -> Result<Command, TaskmasterError> {
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
                _ => return Err(TaskmasterError::Custom(format!("'{command}' Not found"))),
            }
        };

        Ok(cli_command)
    }

    /// This Function will match the command and execute it properly
    pub async fn execute(&self, stream: &mut TcpStream) -> Result<(), TaskmasterError> {
        match self {
            Command::Exit => {
                Command::exit();
                Ok(())
            }
            Command::Help => {
                Command::help();
                Ok(())
            }
            Command::Request(request) => {
                Command::forward_to_server(request, stream).await?;
                let response: Result<Response, TaskmasterError> = receive(stream).await;
                match response {
                    Ok(result) => match result {
                        Response::Success(msg) => println!("{msg}"),
                        Response::Error(msg) => println!("ERROR: {msg}"),
                        Response::Status(processes) => Self::display_status(&processes),
                    },
                    Err(error) => {
                        println!("{error}");
                    }
                }
                Ok(())
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

    fn display_status(programs: &HashMap<String, Vec<ProcessState>>) {
        println!("{programs:?}");
        let mut keys: Vec<String> = programs.keys().cloned().collect();
        keys.sort();
        for name in keys.iter() {
            if let Some(processes) = programs.get(name) {
                for process in processes {
                    match process.status.clone() {
                        ProcessStatus::Stopped => Self::display_stopped_process(name, process),
                        ProcessStatus::Stopping => Self::display_alive_process(name, process),
                        ProcessStatus::Running => Self::display_alive_process(name, process),
                        ProcessStatus::Starting => Self::display_alive_process(name, process),
                        ProcessStatus::Fatal(error) => {
                            Self::display_fatal_process(name, process, &error)
                        }
                    }
                }
                if processes.is_empty() {
                    Self::display_never_started_process(name);
                }
            }
        }
    }

    fn display_stopped_process(name: &String, process: &ProcessState) {
        if let Some(shutdown_time) = process.shutdown_time {
            if let Ok(downtime) = SystemTime::now().duration_since(shutdown_time) {
                println!(
                    "{:<15} {:<10} since {}",
                    name,
                    format!("{:?}", process.status),
                    Self::format_duration(downtime)
                );
            } else {
                println!("{:<15} {:<10}", name, format!("{:?}", process.status),);
            }
        } else {
            println!("{:<15} {:<10}", name, format!("{:?}", process.status),);
        }
    }

    fn display_alive_process(name: &String, process: &ProcessState) {
        if let Ok(uptime) = SystemTime::now().duration_since(process.start_time) {
            println!(
                "{:<15} {:<10} pid {:<5}, uptime {}",
                name,
                format!("{:?}", process.status),
                process.pid,
                Self::format_duration(uptime)
            );
        } else {
            println!(
                "{:<15} {:<10} pid {:<5}",
                name,
                format!("{:?}", process.status),
                process.pid,
            );
        }
    }

    fn display_never_started_process(name: &String) {
        println!(
            "{:<15} {:<10}",
            name,
            format!("{:?}", ProcessStatus::Stopped),
        );
    }

    fn display_fatal_process(name: &String, process: &ProcessState, error: &String) {
        println!(
            "{:<15} {:<10} ({})",
            name,
            format!("{:?}", process.status),
            error
        );
    }

    fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    }
}
