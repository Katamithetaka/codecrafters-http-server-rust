use http_server::http_server::prelude::*;

use http_server::status_code::NOT_FOUND;
use http_server::{response::{bytes, status, text}, status_code::OK};



fn main() -> std::io::Result<()> {
    let mut server = HttpServer::new();
    
    
    let home_dir = if std::env::args().nth(1).is_some_and(|c| c == "--directory") {
        std::env::args().nth(2).unwrap_or(".".into())
    } else {
        ".".into()
    };
    
    
    
    server.get("/", |_| {
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
        match req.headers.get_require_single("user-agent") {
            Ok(Some(t)) => {
                return text(t);
            },
            Ok(None) => {},
            Err(_) => {},
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
                    return status(NOT_FOUND);
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
                    return status(201);
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
    
    let (task, _wx) = server.run("0.0.0.0", "4221", Default::default());
    
    smol::block_on(task)
}
