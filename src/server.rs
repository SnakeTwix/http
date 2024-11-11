use crate::http;
use crate::http::Method;
use log::log;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{io, thread};

#[derive(Debug)]
pub enum ServerError {
    ParseError(String),
}

type Handler = fn(req: Request) -> ();

pub struct Server {
    listener: TcpListener,
    routes: HashMap<(String, http::Method), Handler>,
    not_found_handler: Handler,
}

impl Server {
    pub fn new(addr: &str) -> Self {
        let listener = TcpListener::bind(addr).unwrap();
        let routes = HashMap::new();

        Self {
            not_found_handler: handle_not_found,
            listener,
            routes,
        }
    }

    pub fn process_requests(&mut self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(|| {
                        self.handle_connection(stream);
                    });
                }

                Err(e) => {
                    println!("Got an error: {}", e);
                }
            }
        }
    }

    pub fn handle_connection(&self, stream: TcpStream) {
        let mut read_buffer = [0; 2048];

        let request = match Request::parse(stream, &mut read_buffer) {
            Ok(request) => request,
            Err(e) => {
                log!(log::Level::Error, "{:?}", e);
                return;
            }
        };
        let handler = self.routes.get(&(request.path.to_string(), request.method));
        if handler.is_none() {
            (self.not_found_handler)(request);
            return;
        }

        let handler = handler.unwrap();
        handler(request);
    }

    pub fn register_route(&mut self, http_method: http::Method, route: String, handler: Handler) {
        self.routes.insert((route, http_method), handler);
    }
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

#[derive(Debug)]
pub struct Request<'a> {
    method: http::Method,
    url: &'a str,
    path: &'a str,
    stream: TcpStream,
}

impl<'a> Request<'a> {
    pub fn parse(mut stream: TcpStream, read_buffer: &'a mut [u8]) -> Result<Self, ServerError> {
        let read_amount = stream.read(read_buffer).unwrap();
        println!("Read this much: {}", read_amount);

        let read_str = std::str::from_utf8(&read_buffer[0..read_amount]).unwrap();
        // println!("{}", read_str);

        let first_line = read_str
            .lines()
            .next()
            .ok_or(ServerError::ParseError("Couldn't get first line".into()))?;

        let mut request = Request {
            method: Method::Get,
            stream,
            url: "",
            path: "",
        };

        let mut values = first_line.split_ascii_whitespace();
        let method = values
            .next()
            .ok_or(ServerError::ParseError("Couldn't get the method".into()))?;

        request.method = match method {
            "GET" => Method::Get,
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

    pub fn method(&self) -> http::Method {
        self.method
    }

    pub fn url(&self) -> &'a str {
        self.url
    }

    pub fn path(&self) -> &'a str {
        self.path
    }

    pub fn send_response(&mut self, resp: &str) -> io::Result<()> {
        self.stream.write_all(resp.as_bytes())
    }
}
