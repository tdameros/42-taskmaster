use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct Message {
    message: String,
}

async fn handle_message(message: web::Json<Message>) -> impl Responder {
    println!("Received message: {}", message.message);
    HttpResponse::Ok().body("Message received")
}

/// this is only to demonstrate that a POST request is indeed send to the address specified in the config
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server starting on http://127.0.0.1:8080");

    HttpServer::new(|| App::new().route("/", web::post().to(handle_message)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
