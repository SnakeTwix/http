use log::log;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{thread, time};

#[derive(Debug, Eq, Hash, PartialEq, Copy, Clone)]
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
    stream: TcpStream,
}

impl<'a> Request<'a> {
    fn parse(mut stream: TcpStream, read_buffer: &'a mut [u8]) -> Result<Self, ServerError> {
        let read_amount = stream.read(read_buffer).unwrap();
        println!("Read this much: {}", read_amount);

        let read_str = std::str::from_utf8(&read_buffer[0..read_amount]).unwrap();
        // println!("{}", read_str);

        let first_line = read_str
            .lines()
            .next()
            .ok_or(ServerError::ParseError("Couldn't get first line".into()))?;

        let mut request = Request {
            method: HttpMethods::Get,
            stream,
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

type Handler = fn(req: Request) -> ();

struct Server {
    listener: TcpListener,
    routes: HashMap<(String, HttpMethods), Handler>,
    not_found_handler: Handler,
}

impl Server {
    fn new(addr: &str) -> Self {
        let listener = TcpListener::bind(addr).unwrap();
        let routes = HashMap::new();

        Self {
            not_found_handler: handle_not_found,
            listener,
            routes,
        }
    }

    fn process_requests(&mut self) {
        let mut read_buffer = [0; 2048];

        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let request = match Request::parse(stream, &mut read_buffer) {
                        Ok(request) => request,
                        Err(e) => {
                            log!(log::Level::Error, "{:?}", e);
                            continue;
                        }
                    };

                    thread::sleep(time::Duration::from_secs(10));
                    
                    let handler = self.routes.get(&(request.path.to_string(), request.method));
                    if handler.is_none() {
                        (self.not_found_handler)(request);
                        continue;
                    }

                    let handler = handler.unwrap();
                    handler(request);
                }

                Err(e) => {
                    println!("Got an error: {}", e);
                }
            }
        }
    }

    fn register_route(&mut self, http_method: HttpMethods, route: String, handler: Handler) {
        self.routes.insert((route, http_method), handler);
    }
}

fn handle_home(mut req: Request) {
    let html_response = include_str!("./public/index.html");

    let response_buffer = format!(
        "HTTP/1.1 200 OK\nContent-Length:{}\n\n{}",
        html_response.len(),
        html_response
    );

    req.stream.write_all(response_buffer.as_bytes()).unwrap()
}

fn handle_not_found(mut req: Request) {
    let html_response = include_str!("./public/404.html");

    let response_buffer = format!(
        "HTTP/1.1 404 NOT FOUND\nContent-Length:{}\n\n{}",
        html_response.len(),
        html_response
    );

    req.stream.write_all(response_buffer.as_bytes()).unwrap()
}

fn main() {
    let mut server = Server::new("127.0.0.1:3000");
    server.register_route(HttpMethods::Get, "/".to_string(), handle_home);

    println!("Listening on port 3000!");
    server.process_requests();
}
