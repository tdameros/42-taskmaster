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
        let result = TcpStream::connect(address.to_owned());
        if result.is_err() {
            return;
        }
        let mut stream = result.unwrap();

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

#[cfg(feature = "reqwest")]
pub fn send_notification(token: String, title: String, body: String) {
    thread::spawn(move || {
        // Connect to the Pushbullet API server
        let mut stream = TcpStream::connect("api.pushbullet.com:443").unwrap();

        // Prepare the JSON payload
        let json_payload = format!(
            r#"{{"type":"note","title":"{}","body":"{}"}}"#,
            title.replace("\"", "\\\""),
            body.replace("\"", "\\\"")
        );

        // Construct the HTTP POST request
        let request = format!(
            "POST /v2/pushes HTTP/1.1\r\n\
         Host: api.pushbullet.com\r\n\
         Authorization: Bearer {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
            token,
            json_payload.len(),
            json_payload
        );

        // Send the request
        stream.write_all(request.as_bytes()).unwrap();

        // Read and discard the response
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        println!("--{response}--");
    });
}
