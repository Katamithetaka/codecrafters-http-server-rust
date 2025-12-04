use crate::client_socket::ClientSocket;
use crate::http_method::HttpMethod;
use crate::request::{Map, Request, parse_request};
use crate::response::{Response, status};
use crate::status_code::StatusCode;
use crate::utils::bytes_contain;
use std::io::prelude::*;

use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

const BUFFER_SIZE: usize = 8192;

#[derive(Clone)]
struct HttpListener {
    path: String,
    method: HttpMethod,
    callback: Arc<dyn Fn(Request) -> Response + Send + Sync>,
}

fn path_matches(listener: &HttpListener, path: &str) -> bool {
    let registered_path = &listener.path;
    if registered_path.contains(":") {
        let registered_parts: Vec<&str> = registered_path.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();
        if registered_parts.len() != path_parts.len() {
            return false;
        }
        for (reg_part, path_part) in registered_parts.iter().zip(path_parts.iter()) {
            if reg_part.starts_with(":") {
                continue;
            }
            if reg_part != path_part {
                return false;
            }
        }
        true
    } else {
        registered_path == path
    }
}

fn get_path_params(listener: &HttpListener, path: &str) -> Map<String> {
    let mut params: Map<String> = Map::default();
    let registered_parts: Vec<&str> = listener.path.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();


    
    for (reg_part, path_part) in registered_parts.iter().zip(path_parts.iter()) {
        if reg_part.starts_with(":") {
            let key = reg_part.trim_start_matches(":").to_string();
            let value = path_part.to_string();
            params.add(&key, value);
        }
    }
    params
}

fn method_matches(listener: &HttpListener, method: &HttpMethod) -> bool {
    &listener.method == method || &listener.method == &HttpMethod::ALL
}

pub struct HttpServer {
    callbacks: Vec<HttpListener>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl HttpServer {
    pub fn new() -> Self {
        HttpServer { callbacks: vec![], threads: vec![] }
    }
    
    pub fn send_response(client: &mut ClientSocket, req: Request, mut res: Response) -> std::io::Result<()> {
        let mut response_header = format!(
            "HTTP/1.1 {} {}\r\n",
            res.status_code,
            StatusCode::from_u16(res.status_code).map(|code| code.reason).unwrap_or("Unknown"),
        );
        
        if req.headers.has("connection") && req.headers.get("connection").unwrap().to_lowercase() == "close" {
            response_header.push_str("Connection: close\r\n");
        }
        
        
        if req.headers.has("accept-encoding") && req.headers.get("accept-encoding").unwrap().to_lowercase().contains("gzip") {
            response_header.push_str("Content-Encoding: gzip\r\n");
            res.bytes = crate::utils::gzip_compress(&res.bytes)?;
        }
        
        response_header.push_str(&format!("Content-Type: {}\r\n", res.content_type));
        response_header.push_str(&format!("Content-Length: {}\r\n", res.bytes.len()));
        
        if res.bytes.len() > BUFFER_SIZE {
            response_header.push_str("Transfer-Encoding: chunked\r\n");
        }
        
        response_header.push_str("\r\n");
        
        if res.bytes.len() > BUFFER_SIZE {
            client.socket.write_all(response_header.as_bytes())?;
            let mut start = 0;
            while start < res.bytes.len() {
                let end = std::cmp::min(start + BUFFER_SIZE, res.bytes.len());
                let chunk_size = end - start;
                let chunk_size_hex = format!("{:X}\r\n", chunk_size);
                client.socket.write_all(chunk_size_hex.as_bytes())?;
                client.socket.write_all(&res.bytes[start..end])?;
                client.socket.write_all(b"\r\n")?;
                start += chunk_size;
            }
            client.socket.write_all(b"0\r\n\r\n")?;
        } else {
            let header_bytes = response_header.as_bytes();
            let mut full_response = Vec::with_capacity(header_bytes.len() + res.bytes.len());
            full_response.extend_from_slice(header_bytes);
            full_response.extend_from_slice(&res.bytes);
            
            client.socket.write_all(&full_response)?;
        }
        
        
        
        
        Ok(())
    }
    
    pub fn send_simple_response(client: &mut ClientSocket, res: Response) -> std::io::Result<()> {
        let response_header = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n",
            res.status_code,
            StatusCode::from_u16(res.status_code).map(|code| code.reason).unwrap_or("Unknown"),
            res.bytes.len()
        );
        let header_bytes = response_header.as_bytes();
        let mut full_response = Vec::with_capacity(header_bytes.len() + res.bytes.len());
        full_response.extend_from_slice(header_bytes);
        full_response.extend_from_slice(&res.bytes);
        
        client.socket.write_all(&full_response)
    }

    fn handle_connection(callbacks: Vec<HttpListener>, mut client: ClientSocket) -> std::io::Result<()> {
        loop {
            let (request, extra_body_bytes) = client.read_until("\r\n\r\n".as_bytes())?;
    
            if !bytes_contain(&request, "\r\n\r\n".as_bytes()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid HTTP request.",
                ));
            }
    
            let request = parse_request(&mut client, request, extra_body_bytes);
            match request {
                Ok(Ok(mut req)) => {
                    if req.path.contains("http") {
                        // get the final part after hostname
                        // e.g. http://example.com/path -> /path
                        req.path = req.path.split('/').skip(3).collect::<Vec<&str>>().join("/");
                    }

                    let connection_close = req.headers.has("connection") && req.headers.get("connection").unwrap().to_lowercase() == "close"; 

                    let mut tmp = callbacks.iter();
                    let mut sent = false;
                    while let Some(listener) = tmp.next() {
                        if path_matches(&listener, &req.path) && method_matches(&listener, &req.method) {
                            let path_params = get_path_params(&listener, &req.path);
                            req.path_params = path_params;
                            let kept_request = Request {
                                method: req.method.clone(),
                                headers: req.headers.clone(),
                                ..Default::default()
                            };
                            let res = (listener.callback)(req);
                            Self::send_response(&mut client, kept_request, res);
                            sent = true;
                            break;
                        }
                    }

                    if !sent {
                        Self::send_simple_response(&mut client, status(404))?;
                    }


                    if connection_close {
                        return Ok(())
                    }


           
                    
                },
                Ok(Err(res)) => Self::send_simple_response(&mut client, res)?,
                Err(err) => {
                    println!("Error parsing request: {:?}", err);
                    return Err(err);
                }
            }
        }
    }

    pub fn run(mut self, address: &str, port: &str) -> std::io::Result<()> {
        let server = TcpListener::bind(format!("{address}:{port}").as_str())?;
        let is_running = true;
        println!("Server listening on http://localhost:{port}/");
        while is_running {
            match server.accept() {
                Ok(client) => {
                    let callbacks = self.callbacks.clone();
                    self.threads.push(thread::spawn(|| {
                        match Self::handle_connection(callbacks, ClientSocket { socket: client.0 }) {
                            Ok(_) => println!("Connection handled successfully."),
                            Err(_) => println!("Client disconnected or error occurred."),
                        };
                    }));
                }
                Err(e) => {
                    println!("Couldn't accept client: {e:?}");
                }
            }
        }

        Ok(())
    }
    
    pub fn get<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::GET,
            callback: Arc::new(callback),
        });
    }
    
    pub fn all<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::ALL,
            callback: Arc::new(callback),
        });
    }
    
    pub fn post<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::POST,
            callback: Arc::new(callback),
        });
    }
    
    pub fn patch<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::PATCH,
            callback: Arc::new(callback),
        });
    }
    
    pub fn delete<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::DELETE,
            callback: Arc::new(callback),
        });
    }
    
    pub fn put<T: Into<String>>(&mut self, path: T, callback: impl Fn(Request) -> Response + Send + Sync + 'static) {

        self.callbacks.push(HttpListener {
            path: path.into(),
            method: HttpMethod::PUT,
            callback: Arc::new(callback),
        });
    }
}
