#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Arc, Once};
    use std::time::{Duration, Instant};
    use http_server::http_server::prelude::*;
    use http_server::response::{bytes, status};
    use rustls::pki_types::ServerName;
    use rustls::client::danger::{ServerCertVerifier, HandshakeSignatureValid, ServerCertVerified};
    use rustls::pki_types::{CertificateDer, UnixTime};
    use rustls::{DigitallySignedStruct, SignatureScheme};

    // ---- Certificate verifier that accepts self-signed cert but still verifies signatures ----
    #[derive(Debug)]
    struct SelfSignedVerifier {
        provider: Arc<rustls::crypto::CryptoProvider>,
    }

    impl SelfSignedVerifier {
        fn new() -> Self {
            Self {
                provider: rustls::crypto::CryptoProvider::get_default()
                    .expect("No default crypto provider")
                    .clone()
            }
        }
    }

    impl ServerCertVerifier for SelfSignedVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer,
            _intermediates: &[CertificateDer],
            _server_name: &ServerName,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            // Accept the certificate without checking if it's in a trusted root store
            // This is necessary for self-signed certificates in testing
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            // Actually verify the signature is valid
            rustls::crypto::verify_tls12_signature(
                message,
                cert,
                dss,
                &self.provider.signature_verification_algorithms,
            )
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            // Actually verify the signature is valid
            rustls::crypto::verify_tls13_signature(
                message,
                cert,
                dss,
                &self.provider.signature_verification_algorithms,
            )
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            self.provider
                .signature_verification_algorithms
                .supported_schemes()
        }
    }

    // ---- Start server once ----
    static START: Once = Once::new();

    fn start_server() {
        START.call_once(|| {
            std::thread::spawn(|| {
                let mut server = HttpServer::new();
                    
                server.all("/", |req| {
                    let body = req.body;
                    if body.is_empty() {
                        return status(200)
                    } else {
                        return bytes(body)
                    }
                });
                
                let (task, _wx) = server.run_https("0.0.0.0", "4443", "local_certs/server.cert", "local_certs/server.key", Default::default()); 
                
                smol::block_on(task).unwrap();
            });
            std::thread::sleep(Duration::from_millis(200)); // allow startup
        });
    }

    // ---- Test case structure ----
    struct TestCase<'a> {
        request: &'a str,
        #[allow(unused)]
        description: &'a str,
        expected_status: &'a [(i32, i32)],
        expected_timeout: bool,
        expected_body: Option<&'a str>,
    }

    // ---- Core test runner ----
    fn run_test_case(tc: &TestCase) -> bool {
        let mut tcp_stream = match TcpStream::connect(("127.0.0.1", 4443)) {
            Ok(s) => s,
            Err(_) => {
                return false;
            }
        };
        
        // Set timeout before creating TLS stream
        tcp_stream.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        
        // Set up TLS client configuration with self-signed certificate support
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SelfSignedVerifier::new()))
            .with_no_client_auth();
        
        let server_name = ServerName::try_from("localhost")
            .expect("Invalid server name")
            .to_owned();
        
        let mut tls_conn = rustls::ClientConnection::new(Arc::new(config), server_name)
            .expect("Failed to create TLS connection");
        
        let mut tls_stream = rustls::Stream::new(&mut tls_conn, &mut tcp_stream);
        
        tls_stream.write_all(tc.request.as_bytes()).unwrap();

        let mut buf = [0u8; 4096];
        let start = Instant::now();

        // ---- Wait up to 500ms for server to reply ----
        loop {
            if start.elapsed() >= Duration::from_millis(500) {
                if tc.expected_timeout {
                    return true;
                } else {
                    return false;
                }
            }

            match tls_stream.read(&mut buf) {
                Ok(0) => continue,
                Ok(n) => {
                    let resp = String::from_utf8_lossy(&buf[..n]).to_string();
                    return evaluate_response(tc, &resp);
                }
                Err(_) => continue,
            }
        }
    }

    fn evaluate_response(tc: &TestCase, resp: &str) -> bool {
        let status = parse_status_code(resp);
        let body = parse_body(resp);

        let status_ok = tc.expected_status.iter().any(|(min, max)| {
            status >= *min && status <= *max
        });

        let body_ok = match (&tc.expected_body, status) {
            (Some(expected), 200) => body == *expected,
            _ => true,
        };

        status_ok && body_ok

       
    }

    fn parse_status_code(response: &str) -> i32 {
        response
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1)
    }

    fn parse_body(response: &str) -> String {
        match response.split("\r\n\r\n").nth(1) {
            Some(body) => body.to_string(),
            None => "".to_string(),
        }
    }
    
    

    // ---- Run tests ----
    #[test]
    fn fragmented_method() {
        start_server();
        let case = TestCase {
            request: "G",
            description: "Fragmented method",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_url_1() {
        start_server();
        let case = TestCase {
            request: "GET ",
            description: "Fragmented URL 1",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_url_2() {
        start_server();
        let case = TestCase {
            request: "GET /hello",
            description: "Fragmented URL 2",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_url_3() {
        start_server();
        let case = TestCase {
            request: "GET /hello ",
            description: "Fragmented URL 3",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_http_version() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP",
            description: "Fragmented HTTP version",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_request_line() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1",
            description: "Fragmented request line",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_request_line_newline_1() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r",
            description: "Fragmented request line newline 1",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_request_line_newline_2() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\n",
            description: "Fragmented request line newline 2",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_field_name() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHos",
            description: "Fragmented field name",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_field_value_1() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost:",
            description: "Fragmented field value 1",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_field_value_2() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost: ",
            description: "Fragmented field value 2",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_field_value_3() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost: localhost",
            description: "Fragmented field value 3",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_field_value_4() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost: localhost\r",
            description: "Fragmented field value 4",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_request() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost: localhost\r\n",
            description: "Fragmented request",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn fragmented_request_termination() {
        start_server();
        let case = TestCase {
            request: "GET /hello HTTP/1.1\r\nHost: localhost\r\n\r",
            description: "Fragmented request termination",
            expected_status: &[(-1, -1)],
            expected_timeout: true,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    //
    // --- Now the non-timeout tests ---
    //
    
    #[test]
    fn request_without_http_version() {
        start_server();
        let case = TestCase {
            request: "GET / \r\n\r\n",
            description: "Request without HTTP version",
            expected_status: &[(400, 599)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn expect_header() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nExpect: 100-continue\r\n\r\n",
            description: "Request with Expect header",
            expected_status: &[(100, 100), (200, 299)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn valid_get() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n",
            description: "Valid GET request",
            expected_status: &[(200, 299)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn valid_get_edge() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nhoSt:\texample.com\r\nempty:\r\n\r\n",
            description: "Valid GET request with edge cases",
            expected_status: &[(200, 299)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn invalid_header_chars() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nX-Invalid[]: test\r\n\r\n",
            description: "Invalid header characters",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn missing_host_header() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nContent-Length: 5\r\n\r\n",
            description: "Missing Host header",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn multiple_host_headers() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nHost: example.org\r\n\r\n",
            description: "Multiple Host headers",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn overflowing_negative_content_length() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: -123456789123456789123456789\r\n\r\n",
            description: "Overflowing negative Content-Length header",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn negative_content_length() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: -1234\r\n\r\n",
            description: "Negative Content-Length header",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn non_numeric_content_length() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nContent-Length: abc\r\n\r\n",
            description: "Non-numeric Content-Length header",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn empty_header_value() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nX-Empty-Header: \r\n\r\n",
            description: "Empty header value",
            expected_status: &[(200, 299)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn invalid_control_char() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\nX-Bad-Control-Char: test\x07\r\n\r\n",
            description: "Header containing invalid control character",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn invalid_http_version() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/9.9\r\nHost: example.com\r\n\r\n",
            description: "Invalid HTTP version",
            expected_status: &[(400, 499), (500, 599)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn invalid_prefix_request() {
        start_server();
        let case = TestCase {
            request: "Extra lineGET / HTTP/1.1\r\nHost: example.com\r\n\r\n",
            description: "Invalid prefix of request",
            expected_status: &[(400, 499), (500, 599)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn invalid_line_ending() {
        start_server();
        let case = TestCase {
            request: "GET / HTTP/1.1\r\nHost: example.com\r\n\rSome-Header: Test\r\n\r\n",
            description: "Invalid line ending",
            expected_status: &[(400, 499)],
            expected_timeout: false,
            expected_body: None,
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn valid_post_with_body() {
        start_server();
        let case = TestCase {
            request: "POST / HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nhello",
            description: "Valid POST request with body",
            expected_status: &[(200, 299), (404, 404)],
            expected_timeout: false,
            expected_body: Some("hello"),
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn chunked_transfer_encoding() {
        start_server();
        let case = TestCase {
            request: "POST / HTTP/1.1\r\nHost: example.com\r\nTransfer-Encoding: chunked\r\n\r\nc\r\nHellO world1\r\n0\r\n\r\n",
            description: "Chunked Transfer-Encoding",
            expected_status: &[(200, 299)],
            expected_timeout: false,
            expected_body: Some("HellO world1"),
        };
        assert!(run_test_case(&case));
    }
    
    #[test]
    fn conflicting_te_and_cl() {
        start_server();
        let case = TestCase {
            request: "POST / HTTP/1.1\r\nHost: example.com\r\ncontent-LengtH: 5\r\nTransFer-Encoding: chunked\r\n\r\nc\r\nHellO world1\r\n0\r\n\r\n",
            description: "Conflicting Transfer-Encoding and Content-Length in varying case",
            expected_status: &[(400, 499), (200, 299)],
            expected_timeout: false,
            expected_body: Some("HellO world1"),
        };
        assert!(run_test_case(&case));
    }
}
