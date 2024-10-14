/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */

use std::io::stdin;
use tcl::{
    get_server_address,
    message::{send_message, Message},
};
use tokio::net::TcpStream;

/* -------------------------------------------------------------------------- */
/*                                    Main                                    */
/* -------------------------------------------------------------------------- */
#[tokio::main]
async fn main() {
    println!("Trying to connect to the server");
    let mut stream = TcpStream::connect(get_server_address())
        .await
        .expect("Can't Connect to the server");

    loop {
        let mut user_imput = String::new();
        if let Err(imput_error) = stdin().read_line(&mut user_imput) {
            eprintln!("Error Occurred while reading user imput: {imput_error}, please close the terminal and restart the client");
        }
        let trimed_user_imput = user_imput.trim();

        if trimed_user_imput.eq_ignore_ascii_case("quit") {
            // here we want to remplace this with a match to se what command the user is tell
            break;
        }

        if let Err(e) =
            send_message(&mut stream, &Message::Test(trimed_user_imput.to_owned())).await
        {
            eprintln!("Error while sending message to server: {e}");
        }
    }
}
