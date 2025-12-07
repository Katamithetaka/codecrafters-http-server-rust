#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::Once;
    use std::time::Duration;
    use http_server::http_server::prelude::*;
    use http_server::response::{bytes, status, text, text_response};
    use http_server::utils::{bytes_split, gzip_compress};

    static START: Once = Once::new();

    fn start_server() {
        START.call_once(|| {
            std::thread::spawn(|| {
                let mut server = HttpServer::new();
                
                // Basic GET
                server.get("/", |_req| {
                    text("Hello, World!")
                });

                server.get("/echo/:data", |req| {
                    let data = req.path_params.get("data").cloned().unwrap_or_default();
                    text(data)
                });
                
                // Different HTTP methods
                server.post("/echo", |req| {
                    bytes(req.body)
                });
                
                server.put("/update", |_req| {
                    status(200)
                });
                
                server.delete("/remove", |_req| {
                    status(204)
                });
                
                server.patch("/modify", |_req| {
                    status(200)
                });
                
                // Path parameters
                server.get("/users/:id", |req| {
                    let id = req.path_params.get("id").cloned().unwrap_or_default();
                    text(format!("User ID: {}", id))
                });
                
                server.get("/posts/:post_id/comments/:comment_id", |req| {
                    let post_id = req.path_params.get("post_id").cloned().unwrap_or_default();
                    let comment_id = req.path_params.get("comment_id").cloned().unwrap_or_default();
                    text(format!("Post: {}, Comment: {}", post_id, comment_id))
                });
                
                // Headers
                server.get("/headers", |req| {
                    let user_agent = req.headers.get_single("user-agent").cloned().unwrap_or_default();
                    let custom = req.headers.get_single("x-custom-header").cloned().unwrap_or_default();
                    text(format!("UA: {}, Custom: {}", user_agent, custom))
                });
                
                // JSON-like response
                server.get("/json", |_req| {
                    text_response(
                        200,
                        "application/json",
                        r#"{"status":"ok","data":{"id":123,"name":"test"}}"#.as_bytes().to_vec()
                    )
                });
                
                // Different status codes
                server.get("/notfound", |_req| {
                    status(404)
                });
                
                server.get("/error", |_req| {
                    status(500)
                });
                
                server.get("/redirect", |_req| {
                    status(302).header("Location", "/")
                });
                
                // Large body handling
                server.post("/large", |req| {
                    text(format!("Received {} bytes", req.body.len()))
                });
                
                // Content type tests
                server.post("/content-type", |req| {
                    let content_type = req.headers.get_single("content-type").cloned().unwrap_or_default();
                    text(format!("Content-Type: {}", content_type))
                });
                
                // Custom headers test
                server.get("/custom-headers", |_req| {
                    text("OK")
                        .header("X-Custom-Header", "CustomValue")
                        .header("X-Request-Id", "12345")
                });
                
                let (task, _wx) = server.run("0.0.0.0", "5000", Default::default());
                smol::block_on(task).unwrap();
            });
            std::thread::sleep(Duration::from_millis(200));
        });
    }

    fn make_request(request: &str) -> String {
        let mut stream = TcpStream::connect("127.0.0.1:5000").expect("Failed to connect");
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap_or_default();
        response
    }

    fn get_status_code(response: &str) -> i32 {
        response
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1)
    }

    fn get_body(response: &str) -> String {
        response
            .split("\r\n\r\n")
            .nth(1)
            .unwrap_or("")
            .to_string()
    }

    fn get_header<'a>(response: &'a str, header: &str) -> Option<String> {
        let header_lower = header.to_lowercase();
        response
            .lines()
            .skip(1)
            .take_while(|line| !line.is_empty())
            .find(|line| line.to_lowercase().starts_with(&format!("{}:", header_lower)))
            .and_then(|line| line.split_once(':'))
            .map(|(_, value)| value.trim().to_string())
    }

    // ===== Basic HTTP Methods =====
    
    #[test]
    fn test_get_request() {
        start_server();
        let response = make_request("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "Hello, World!");
    }

    #[test]
    fn test_post_echo() {
        start_server();
        let response = make_request(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\n\r\nHello World"
        );
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "Hello World");
    }

    #[test]
    fn test_put_request() {
        start_server();
        let response = make_request("PUT /update HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
    }

    #[test]
    fn test_delete_request() {
        start_server();
        let response = make_request("DELETE /remove HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 204);
    }

    #[test]
    fn test_patch_request() {
        start_server();
        let response = make_request("PATCH /modify HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
    }

    // ===== Path Parameters =====
    
    #[test]
    fn test_single_path_param() {
        start_server();
        let response = make_request("GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "User ID: 42");
    }

    #[test]
    fn test_single_path_param_string() {
        start_server();
        let response = make_request("GET /users/alice HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "User ID: alice");
    }

    #[test]
    fn test_multiple_path_params() {
        start_server();
        let response = make_request("GET /posts/123/comments/456 HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "Post: 123, Comment: 456");
    }

    // ===== Headers =====
    
    #[test]
    fn test_request_headers() {
        start_server();
        let response = make_request(
            "GET /headers HTTP/1.1\r\nHost: localhost\r\nUser-Agent: TestAgent/1.0\r\nX-Custom-Header: custom-value\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        assert!(get_body(&response).contains("TestAgent/1.0"));
        assert!(get_body(&response).contains("custom-value"));
    }

    #[test]
    fn test_content_type_header() {
        start_server();
        let response = make_request(
            "GET /json HTTP/1.1\r\nHost: localhost\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        let content_type = get_header(&response, "Content-Type");
        assert!(content_type.is_some());
        assert!(content_type.unwrap().contains("application/json"));
    }

    // ===== Status Codes =====
    
    #[test]
    fn test_404_response() {
        start_server();
        let response = make_request("GET /notfound HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 404);
    }

    #[test]
    fn test_500_response() {
        start_server();
        let response = make_request("GET /error HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 500);
    }

    #[test]
    fn test_302_redirect() {
        start_server();
        let response = make_request("GET /redirect HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 302);
        let location = get_header(&response, "Location");
        assert!(location.is_some(), "Location header missing");
        assert_eq!(location.unwrap(), "/");
    }

    #[test]
    fn test_unregistered_route() {
        start_server();
        let response = make_request("GET /does-not-exist HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 404);
    }

    // ===== Body Handling =====
    
    #[test]
    fn test_post_with_json_body() {
        start_server();
        let body = r#"{"name":"test","value":123}"#;
        let response = make_request(&format!(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        ));
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), body);
    }

    #[test]
    fn test_post_with_form_data() {
        start_server();
        let body = "name=John&email=john@example.com";
        let response = make_request(&format!(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        ));
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), body);
    }

    #[test]
    fn test_large_post_body() {
        start_server();
        let body = "x".repeat(10000);
        let response = make_request(&format!(
            "POST /large HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        ));
        assert_eq!(get_status_code(&response), 200);
        assert!(get_body(&response).contains("10000 bytes"));
    }

    #[test]
    fn test_empty_post_body() {
        start_server();
        let response = make_request(
            "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        assert_eq!(get_body(&response), "");
    }

    // ===== Content Type =====
    
    #[test]
    fn test_content_type_detection() {
        start_server();
        let response = make_request(
            "POST /content-type HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 0\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        assert!(get_body(&response).contains("application/json"));
    }

    #[test]
    fn test_plain_text_content_type() {
        start_server();
        let response = make_request(
            "POST /content-type HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        assert!(get_body(&response).contains("text/plain"));
    }

    // ===== Compression =====
    
    #[test]
    fn test_gzip_compression() {
        start_server();
        let mut stream = TcpStream::connect("127.0.0.1:5000").expect("Failed to connect");
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.write_all(b"GET /echo/test HTTP/1.1\r\nHost: localhost\r\nAccept-Encoding: gzip\r\n\r\n").unwrap();
        
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        // Split headers and body
        let headers = bytes_split(&buf[..n].to_vec(), b"\r\n\r\n");
        assert!(headers.is_some(), "Failed to split headers and body");
        let (header_bytes, body_bytes) = headers.unwrap();
        let response = String::from_utf8_lossy(&header_bytes);
        assert_eq!(get_status_code(&response), 200);

        let result = gzip_compress("test".as_bytes()).expect("Failed to compress test data");

        assert_eq!(body_bytes, result, "Response body is not correctly gzip compressed");
        
        let encoding = get_header(&response, "Content-Encoding");
        // Server should compress text responses when client supports gzip
        assert!(encoding.is_some(), "No Content-Encoding header found");
        assert_eq!(encoding.unwrap(), "gzip");
    }

    // ===== Edge Cases =====
    
    #[test]
    fn test_case_insensitive_headers() {
        start_server();
        let response = make_request(
            "GET /headers HTTP/1.1\r\nHOST: localhost\r\nuser-AGENT: MixedCase/1.0\r\n\r\n"
        );
        assert_eq!(get_status_code(&response), 200);
        assert!(get_body(&response).contains("MixedCase"));
    }

    #[test]
    fn test_path_with_trailing_slash() {
        start_server();
        let response = make_request("GET /users/123/ HTTP/1.1\r\nHost: localhost\r\n\r\n");
        // Depending on implementation, this might match /users/:id or not
        let status = get_status_code(&response);
        assert!(status == 200 || status == 404);
    }

    #[test]
    fn test_multiple_requests_same_connection() {
        start_server();
        let mut stream = TcpStream::connect("127.0.0.1:5000").expect("Failed to connect");
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        
        // First request
        stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap();
        let response1 = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(get_status_code(&response1), 200);
        
        // Second request on same connection
        stream.write_all(b"GET /users/42 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let n = stream.read(&mut buf).unwrap();
        let response2 = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(get_status_code(&response2), 200);
    }

    #[test]
    fn test_json_response_body() {
        start_server();
        let response = make_request("GET /json HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        let body = get_body(&response);
        assert!(body.contains("\"status\":\"ok\""));
        assert!(body.contains("\"id\":123"));
    }

    #[test]
    fn test_custom_response_headers() {
        start_server();
        let response = make_request("GET /custom-headers HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        
        let custom_header = get_header(&response, "X-Custom-Header");
        assert!(custom_header.is_some(), "X-Custom-Header missing");
        assert_eq!(custom_header.unwrap(), "CustomValue");
        
        let request_id = get_header(&response, "X-Request-Id");
        assert!(request_id.is_some(), "X-Request-Id missing");
        assert_eq!(request_id.unwrap(), "12345");
    }

    #[test]
    fn test_options_method() {
        start_server();
        let response = make_request("OPTIONS /users/123 HTTP/1.1\r\nHost: localhost\r\n\r\n");
        assert_eq!(get_status_code(&response), 200);
        
        let allow_header = get_header(&response, "Allow");
        assert!(allow_header.is_some(), "Allow header missing");
        let allow = allow_header.unwrap();
        assert!(allow.contains("GET"), "GET not in Allow header");
    }

    #[test]
    fn test_expect_100_continue() {
        start_server();
        
        let mut stream = TcpStream::connect("127.0.0.1:5000").expect("Failed to connect");
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        
        // Send headers with Expect: 100-continue
        let request_headers = "POST /echo HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\nContent-Length: 11\r\n\r\n";
        stream.write_all(request_headers.as_bytes()).expect("Failed to send headers");
        
        // Read 100 Continue response
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).expect("Failed to read 100 Continue");
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(response.contains("100 Continue"), "Did not receive 100 Continue: {}", response);
        
        // Now send the body
        stream.write_all(b"Hello World").expect("Failed to send body");
        
        // Read final response
        let mut response_buf = Vec::new();
        let mut temp = [0u8; 1024];
        loop {
            match stream.read(&mut temp) {
                Ok(0) => break,
                Ok(n) => response_buf.extend_from_slice(&temp[..n]),
                Err(_) => break,
            }
            if response_buf.len() > 100 {
                break;
            }
        }
        
        let final_response = String::from_utf8_lossy(&response_buf);
        assert!(final_response.contains("200 OK"), "Did not receive 200 OK: {}", final_response);
        let body = get_body(&final_response);
        assert_eq!(body, "Hello World");
    }
}
