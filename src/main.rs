use crate::ServerError::ParseError;
use log::log;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;

#[derive(Debug)]
enum HttpMethods {
    Get,
}

#[derive(Debug)]
enum ServerError {
    ParseError(String),
}

#[derive(Debug)]
struct Request<'a> {
    method: HttpMethods,
    url: &'a str,
    path: &'a str,
}

impl<'a> TryFrom<&'a str> for Request<'a> {
    type Error = ServerError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let first_line = value
            .lines()
            .next()
            .ok_or(ServerError::ParseError("Couldn't get first line".into()))?;

        let mut request = Request {
            method: HttpMethods::Get,
            url: "",
            path: "",
        };

        let mut values = first_line.split_ascii_whitespace();
        let method = values
            .next()
            .ok_or(ServerError::ParseError("Couldn't get the method".into()))?;

        request.method = match method {
            "GET" => HttpMethods::Get,
            _ => return Err(ServerError::ParseError("Couldn't match method".into())),
        };

        request.url = values
            .next()
            .ok_or(ServerError::ParseError("Couldn't get url".into()))?;

        // TODO Implement URL syntax
        // WTF is wrong with urls
        if request.url.starts_with("http") {
            let mut patterns = request.url.splitn(3, '/');
            request.path = patterns
                .nth(3)
                .ok_or(ServerError::ParseError("Couldn't get relative url".into()))?;

            let query_start = request.path.find("?").unwrap_or(request.path.len());
            request.path = &request.path[0..query_start];
        } else if request.url.starts_with("/") {
            request.path = request.url;
            let query_start = request.path.find("?").unwrap_or(request.path.len());
            request.path = &request.path[0..query_start];
        } else {
            return Err(ServerError::ParseError("Couldn't parse url".into()));
        }

        Ok(request)
    }
}

type Handler = fn(stream: &mut std::net::TcpStream, req: Request) -> ();

struct Server {
    listener: TcpListener,
    routes: HashMap<String, Handler>,
    read_buffer: [u8; 2048],
    not_found_handler: Handler,
}

impl Server {
    fn new(addr: &str) -> Self {
        let listener = TcpListener::bind(addr).unwrap();
        let routes = HashMap::new();
        let read_buffer = [0; 2048];

        Self {
            not_found_handler: handle_not_found,
            listener,
            routes,
            read_buffer,
        }
    }

    fn process_requests(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let read_amount = stream.read(&mut self.read_buffer).unwrap();
                    println!("Read this much: {}", read_amount);

                    let read_str = std::str::from_utf8(&self.read_buffer[0..read_amount]).unwrap();
                    // println!("{}", read_str);

                    let request = match Request::try_from(read_str) {
                        Ok(request) => request,
                        Err(e) => {
                            log!(log::Level::Error, "{:?}", e);
                            continue;
                        }
                    };

                    let handler = self.routes.get(request.path);
                    if handler.is_none() {
                        (self.not_found_handler)(&mut stream, request);
                        continue;
                    }

                    let handler = handler.unwrap();
                    handler(&mut stream, request);
                }

                Err(e) => {
                    println!("Got an error: {}", e);
                }
            }
        }
    }

    fn register_route(&mut self, route: String, handler: Handler) {
        self.routes.insert(route, handler);
    }
}

fn handle_home(stream: &mut std::net::TcpStream, req: Request) {
    let html_response = include_str!("./public/index.html");

    let response_buffer = format!(
        "HTTP/1.1 200 OK\nContent-Length:{}\n\n{}",
        html_response.len(),
        html_response
    );

    stream.write_all(response_buffer.as_bytes()).unwrap()
}

fn handle_not_found(stream: &mut std::net::TcpStream, req: Request) {
    let html_response = include_str!("./public/404.html");

    let response_buffer = format!(
        "HTTP/1.1 404 NOT FOUND\nContent-Length:{}\n\n{}",
        html_response.len(),
        html_response
    );

    stream.write_all(response_buffer.as_bytes()).unwrap()
}

fn main() {
    let mut server = Server::new("127.0.0.1:3000");
    server.register_route("/".to_string(), handle_home);

    println!("Listening on port 3000!");
    server.process_requests();
}
