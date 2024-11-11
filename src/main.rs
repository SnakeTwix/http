mod http;
mod server;

use crate::http::Method;
use std::io::{Write};


fn handle_home(mut req: server::Request) {
    let html_response = include_str!("./public/index.html");

    let response_buffer = format!(
        "HTTP/1.1 200 OK\nContent-Length:{}\n\n{}",
        html_response.len(),
        html_response
    );

    req.send_response(&response_buffer).unwrap()
}

fn main() {
    let mut server = server::Server::new("127.0.0.1:3000");
    server.register_route(Method::Get, "/".to_string(), handle_home);

    println!("Listening on port 3000!");
    server.process_requests();
}
