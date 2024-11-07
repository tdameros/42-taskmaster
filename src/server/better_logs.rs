/* -------------------------------------------------------------------------- */
/*                                   Import                                   */
/* -------------------------------------------------------------------------- */
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;

/* -------------------------------------------------------------------------- */
/*                                  Function                                  */
/* -------------------------------------------------------------------------- */
pub fn send_http_message(address: String, message: String) {
    thread::spawn(move || {
        // Connect to the server
        let mut stream = TcpStream::connect(address.to_owned()).unwrap();

        // Prepare the JSON payload
        let body = format!("{{\"message\":\"{}\"}}", message);

        // Construct the HTTP POST request with JSON content type
        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             \r\n\
             {}",
            address,
            body.len(),
            body
        );

        // Send the request
        stream.write_all(request.as_bytes()).unwrap();

        // Read the response
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        println!("Server response: {}", response);
    });
}
