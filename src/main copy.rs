#![allow(unused)]

mod client_socket;
mod http_method;
mod http_server;
mod request;
mod response;
mod server_socket;
mod socket;
mod utils;
mod status_code;
mod mime_type;


use http_server::HttpServer;

use crate::{response::{bytes, status, text}, status_code::OK};

fn main() -> std::io::Result<()> {
    let mut server = HttpServer::new();
    
    
    let home_dir = if std::env::args().nth(1).unwrap_or("".into()) == "--directory" {
        std::env::args().nth(2).unwrap_or(".".into())
    } else {
        ".".into()
    };
    
    
    
    server.get("/", |req| {
        return text("Hello, World!");
    });
  
    server.get("/echo/:message", |req| {
        let path_param = match req.path_params.get("message") {
            Some(param) => param,
            None => "",
        };
        
        return text(path_param);
    });
  
    server.get("/user-agent", |req| {
        if req.headers.has("user-agent") {
            let header_value = req.headers.get("user-agent").unwrap();
            return text(header_value);
        }
        return text("No User-Agent header found");
    });
    
    {    
        let home_dir = home_dir.clone();
        server.get("/files/:path", move |req| {
            let path = match req.path_params.get("path") {
                Some(param) => param,
                None => "",
            };
            let dir = format!("{}/{}", home_dir, path);
            match std::fs::read(dir) {
                Ok(content) => {
                    return bytes(content);
                },
                Err(_) => {
                    return status(404);
                }
            }
        });
    }
  
    {    
        let home_dir = home_dir.clone();
        server.post("/files/:path", move |req| {
            let path = match req.path_params.get("path") {
                Some(param) => param,
                None => "",
            };
            let dir = format!("{}/{}", home_dir, path);
            match std::fs::write(dir, req.body) {
                Ok(_) => {
                    return status(200);
                },
                Err(_) => {
                    return status(500);
                }
            }
            
        });
    }

    
    server.all("/", |req| {
       if req.body.is_empty() {
           return status(OK.code);
       }
       else {
           return bytes(req.body);
       }
        
    });
    
    server.run("0.0.0.0", "4221")
}
