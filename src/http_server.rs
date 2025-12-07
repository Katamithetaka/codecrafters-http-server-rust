use std::net::SocketAddr;
use std::sync::{Arc, Weak};
use std::time::Duration;

use crate::client_socket::{ClientSocket, ReadError, Socket};
use crate::http_server_trait::{HttpListener, get_path_params, method_matches, path_matches};
pub use crate::middleware::{HttpMiddleware, MiddlewareEntry, MiddlewareHandler, MiddlewareType};
use crate::request::{Request, RequestParsingError, parse_request};
use crate::response::{Response, status};
use crate::status_code::{METHOD_NOT_ALLOWED, NOT_FOUND, PAYLOAD_TOO_LARGE};
use crate::utils::bytes_contain;

use futures::{AsyncRead, AsyncWrite, FutureExt};
use rustls::ServerConfig;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use smol::net::{TcpListener, TcpStream};

pub use crate::http_server_trait::HttpCallbacks;

const BUFFER_SIZE: usize = 8192;

pub struct HttpServer<'a> {
    callbacks: Vec<HttpListener<Request, Response>>,
    middlewares: Vec<MiddlewareEntry<'a>>,
}

#[derive(Clone, Copy)]
pub struct HttpServerSizeConfig {
    pub request_header_max_size: usize,
    pub request_body_max_size: usize,
}

impl Default for HttpServerSizeConfig {
    fn default() -> Self {
        HttpServerSizeConfig {
            request_header_max_size: 8192,
            request_body_max_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

#[derive(Clone, Copy)]
pub struct HttpServerTimeoutConfig {
    pub read_timeout_duration: Duration,
    pub write_timeout_duration: Duration,
}

impl Default for HttpServerTimeoutConfig {
    fn default() -> Self {
        HttpServerTimeoutConfig {
            read_timeout_duration: Duration::from_secs(5),
            write_timeout_duration: Duration::from_secs(5),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ShutdownMode {
    Immediate,
    Graceful(Duration),
}

impl Default for ShutdownMode {
    fn default() -> Self {
        ShutdownMode::Graceful(Duration::from_secs(30))
    }
}

#[derive(Clone, Copy, Default)]
pub struct HttpServerConfig {
    pub size_config: HttpServerSizeConfig,
    pub timeout_config: HttpServerTimeoutConfig,
    pub shutdown_mode: ShutdownMode,
}

pub struct HttpsServerConfig {
    pub size_config: HttpServerSizeConfig,
    pub timeout_config: HttpServerTimeoutConfig,
    pub shutdown_mode: ShutdownMode,
    pub cert_path: String,
    pub key_path: String,
}

pub enum AcceptError {
    Shutdown,
    IoError(std::io::Error),
}

impl<'a> HttpServer<'a> {
    pub fn new() -> Self {
        HttpServer {
            callbacks: vec![],
            middlewares: vec![],
        }
    }

    async fn send_response<T: Socket>(
        client: &mut T,
        req: Request,
        mut res: Response,
    ) -> std::io::Result<()> {
        let mut response_header = format!(
            "HTTP/1.1 {} {}\r\n",
            res.status_code.code, res.status_code.reason
        );

        if req
            .headers
            .get_single("connection")
            .is_some_and(|c| c.to_lowercase() == "close")
        {
            response_header.push_str("Connection: close\r\n");
        }

        if req
            .headers
            .get_single("accept-encoding")
            .is_some_and(|e| e.contains("gzip"))
            && !res.content_type.is_binary
        {
            response_header.push_str("Content-Encoding: gzip\r\n");
            res.bytes = crate::utils::gzip_compress(&res.bytes)?;
        }

        response_header.push_str(&format!("Content-Type: {}\r\n", res.content_type));
        response_header.push_str(&format!("Content-Length: {}\r\n", res.bytes.len()));

        if res.bytes.len() > BUFFER_SIZE {
            response_header.push_str("Transfer-Encoding: chunked\r\n");
        }

        // Add custom headers
        for (key, value) in &res.headers {
            response_header.push_str(&format!("{}: {}\r\n", key, value));
        }

        response_header.push_str("\r\n");

        if res.bytes.len() > BUFFER_SIZE {
            client
                .write_all(response_header.as_bytes())
                .await
                .map_err(|e| -> std::io::Error { e.into() })?;
            let mut start = 0;
            while start < res.bytes.len() {
                let end = std::cmp::min(start + BUFFER_SIZE, res.bytes.len());
                let chunk_size = end - start;
                let chunk_size_hex = format!("{:X}\r\n", chunk_size);
                client
                    .write_all(chunk_size_hex.as_bytes())
                    .await
                    .map_err(|e| -> std::io::Error { e.into() })?;
                client
                    .write_all(&res.bytes[start..end])
                    .await
                    .map_err(|e| -> std::io::Error { e.into() })?;
                client
                    .write_all(b"\r\n")
                    .await
                    .map_err(|e| -> std::io::Error { e.into() })?;
                start += chunk_size;
            }
            client
                .write_all(b"0\r\n\r\n")
                .await
                .map_err(|e| -> std::io::Error { e.into() })?;
        } else {
            let header_bytes = response_header.as_bytes();
            let mut full_response = Vec::with_capacity(header_bytes.len() + res.bytes.len());
            full_response.extend_from_slice(header_bytes);
            full_response.extend_from_slice(&res.bytes);

            client
                .write_all(&full_response)
                .await
                .map_err(|e| -> std::io::Error { e.into() })?;
        }

        Ok(())
    }

    async fn send_simple_response<T: Socket>(client: &mut T, res: Response) -> std::io::Result<()> {
        let response_header = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n\r\n",
            res.status_code.code,
            res.status_code.reason,
            res.bytes.len()
        );
        let header_bytes = response_header.as_bytes();
        let mut full_response = Vec::with_capacity(header_bytes.len() + res.bytes.len());
        full_response.extend_from_slice(header_bytes);
        full_response.extend_from_slice(&res.bytes);

        client
            .write_all(&full_response)
            .await
            .map_err(|e| -> std::io::Error { e.into() })
    }

    async fn process_request<T: Socket>(
        request: Vec<u8>,
        extra_body_bytes: Vec<u8>,
        callbacks: &[HttpListener<Request, Response>],
        config: HttpServerConfig,
        client: &mut T,
    ) -> std::io::Result<bool> {
        let request = parse_request(client, request, extra_body_bytes, config).await;
        match request {
            Ok(mut req) => {
                if req.path.contains("http") {
                    // get the final part after hostname
                    // e.g. http://example.com/path -> /path
                    req.path = req.path.split('/').skip(3).collect::<Vec<&str>>().join("/");
                }

                let connection_close = req
                    .headers
                    .get_single("connection")
                    .is_some_and(|c| c.to_lowercase() == "close");

                // Handle OPTIONS request
                if req.method == crate::http_method::HttpMethod::OPTIONS {
                    let mut allowed_methods = vec![];
                    for listener in callbacks {
                        if path_matches(&listener, &req.path) {
                            if listener.method == crate::http_method::HttpMethod::ALL {
                                allowed_methods = vec![
                                    "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD",
                                ];
                                break;
                            } else {
                                let method_str = match listener.method {
                                    crate::http_method::HttpMethod::GET => "GET",
                                    crate::http_method::HttpMethod::POST => "POST",
                                    crate::http_method::HttpMethod::PUT => "PUT",
                                    crate::http_method::HttpMethod::DELETE => "DELETE",
                                    crate::http_method::HttpMethod::PATCH => "PATCH",
                                    crate::http_method::HttpMethod::HEAD => "HEAD",
                                    crate::http_method::HttpMethod::OPTIONS => "OPTIONS",
                                    _ => continue,
                                };
                                if !allowed_methods.contains(&method_str) {
                                    allowed_methods.push(method_str);
                                }
                            }
                        }
                    }
                    if !allowed_methods.is_empty() {
                        if !allowed_methods.contains(&"OPTIONS") {
                            allowed_methods.push("OPTIONS");
                        }
                        allowed_methods.sort();
                        let allow_header = allowed_methods.join(", ");
                        let res = status(200).header("Allow", allow_header);
                        Self::send_response(client, req, res).await?;
                        return Ok(connection_close);
                    } else {
                        Self::send_simple_response(client, status(NOT_FOUND)).await?;
                        return Ok(connection_close);
                    }
                }

                let mut sent = false;
                let mut found_path = false;
                for listener in callbacks {
                    if !found_path && path_matches(&listener, &req.path) {
                        found_path = true;
                    }
                    if path_matches(&listener, &req.path) && method_matches(&listener, &req.method)
                    {
                        let path_params = get_path_params(&listener, &req.path);
                        req.path_params = path_params;
                        let kept_request = Request {
                            headers: req.headers.clone(),
                            ..Default::default()
                        };
                        let res = (listener.callback)(req);
                        match Self::send_response(client, kept_request, res).await {
                            Ok(_) => {}
                            Err(_) => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::BrokenPipe,
                                    "Client disconnected.",
                                ));
                            }
                        }
                        sent = true;
                        break;
                    }
                }

                if !sent && !found_path {
                    Self::send_simple_response(client, status(NOT_FOUND)).await?;
                } else if !sent && found_path {
                    Self::send_simple_response(client, status(METHOD_NOT_ALLOWED)).await?;
                }

                if connection_close {
                    return Ok(true);
                }
            }
            Err(
                RequestParsingError::InvalidBody
                | RequestParsingError::InvalidHeader
                | RequestParsingError::InvalidRequest
                | RequestParsingError::UnhandledRequest,
            ) => {
                let res = status(400);
                Self::send_simple_response(client, res).await?;
            }
            Err(RequestParsingError::PayloadTooLarge) => {
                let res = status(PAYLOAD_TOO_LARGE);
                Self::send_simple_response(client, res).await?;
            }
            Err(RequestParsingError::Cancellation) => {
                println!("Request parsing cancelled.");
                return Ok(true);
            }
            Err(RequestParsingError::IoError(e)) => {
                println!("IO Error during request parsing: {:?}", e);
                return Err(e);
            }
            Err(RequestParsingError::Timeout) => {
                println!("Request parsing timed out.");
                return Ok(true);
            }
            Err(RequestParsingError::UnexpectedError) => {
                println!("Unexpected error during request parsing.");
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn handle_connection<T: Socket>(
        callbacks: &[HttpListener<Request, Response>],
        config: HttpServerConfig,
        mut client: T,
    ) -> std::io::Result<()> {
        loop {
            match client
                .read_until(
                    "\r\n\r\n".as_bytes(),
                    config.size_config.request_header_max_size,
                )
                .await
            {
                Ok((request, _)) if !bytes_contain(&request, b"\r\n\r\n") => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid HTTP request.",
                    ));
                }
                Ok((request, _)) if request.is_empty() => {
                    return Ok(());
                }
                Ok((request, extra_bytes)) => {
                    match Self::process_request(
                        request,
                        extra_bytes,
                        callbacks,
                        config,
                        &mut client,
                    )
                    .await
                    {
                        Ok(should_close) => {
                            if should_close {
                                return Ok(());
                            } else {
                                continue;
                            }
                        }
                        Err(e) => {
                            println!("Error processing request: {:?}", e);
                            return Err(e);
                        }
                    }
                }
                Err(ReadError::Cancellation) => {
                    println!("Connection cancelled.");
                    return Ok(());
                }
                Err(ReadError::Timeout) => {
                    println!("Read timeout from client.");
                    return Ok(());
                }
                Err(ReadError::MaxSizeExceeded) => {
                    let res = status(PAYLOAD_TOO_LARGE);
                    Self::send_simple_response(&mut client, res).await?;
                    continue;
                }
                Err(ReadError::IoError(e)) => {
                    println!("Error reading from client: {:?}", e);
                    return Err(e);
                }
                Err(ReadError::UnexpectedError) => {
                    println!("Highly unexpected state reached.");
                    return Ok(());
                }
            }
        }
    }

    pub async fn shutdown_server(
        shutdown_mode: ShutdownMode,
        cancellation_token: smol::channel::Sender<()>,
    ) -> std::io::Result<()> {
        match shutdown_mode {
            ShutdownMode::Immediate => {
                println!("Shutting down server immediately.");
                // Send cancellation to all active connections
                cancellation_token.send(()).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to send cancellation: {}", e),
                    )
                })?;
            }
            ShutdownMode::Graceful(timeout) => {
                println!("Shutting down server gracefully (timeout: {:?}).", timeout);
                // Wait for timeout, then cancel remaining connections
                smol::Timer::after(timeout).await;
                println!("Graceful shutdown period ended, cancelling remaining connections.");
                cancellation_token.send(()).await.map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to send cancellation: {}", e),
                    )
                })?;
            }
        }
        Ok(())
    }

    pub async fn accept_connection(
        server: &TcpListener,
        config: &HttpServerConfig,
        cancellation_token: smol::channel::Receiver<()>,
        cancel_tx: smol::channel::Sender<()>,
    ) -> Result<(TcpStream, std::net::SocketAddr), AcceptError> {
        futures::select! {
            accept_result = server.accept().fuse() => {
                return accept_result.map_err(|e| AcceptError::IoError(e));
            },
            _ = cancellation_token.recv().fuse() => {
                Self::shutdown_server(config.shutdown_mode, cancel_tx).await.map_err(|_| AcceptError::Shutdown)?;
                return Err(AcceptError::Shutdown);
            }
        }
    }

    pub fn run(
        self,
        address: &str,
        port: &str,
        config: HttpServerConfig,
    ) -> (smol::Task<std::io::Result<()>>, smol::channel::Sender<()>) {
        let (tx, rx) = smol::channel::bounded::<()>(1);
        let (cancel_tx, cancel_rx) = smol::channel::bounded::<()>(1);
        let address = address.to_string();
        let port = port.to_string();
        let callbacks = Arc::new(self.callbacks);
        let task = smol::spawn(async move {
            let server = TcpListener::bind(format!("{address}:{port}").as_str()).await?;
            println!("Server listening on http://localhost:{port}/");
                loop {
                    let client_connection =
                        match Self::accept_connection(&server, &config, rx.clone(), cancel_tx.clone())
                            .await
                        {
                            Ok((stream, addr)) => (stream, addr),
                            Err(AcceptError::Shutdown) => {
                                println!("Server is shutting down.");
                                break;
                            }
                            Err(AcceptError::IoError(e)) => {
                                println!("Error accepting connection: {:?}", e);
                                continue;
                            }
                        };

                    Self::run_connection(Arc::downgrade(&callbacks), config, cancel_rx.clone(), client_connection);

                }   
            Ok(())
        });
        (task, tx)
    }
    
    pub fn run_connection< T: AsyncRead + AsyncWrite + Unpin + 'static + Send>(callbacks: Weak<Vec<HttpListener<Request, Response>>> , config: HttpServerConfig, cancellation_token: smol::channel::Receiver<()>, (connection, addr): (T, SocketAddr)) {
        
        let callbacks = match callbacks.upgrade() {
            Some(cbs) => cbs,
            None => {
                println!("Callbacks have been dropped, closing connection from {}.", addr);
                return;
            }
        };
        smol::spawn(async move {
            match Self::handle_connection(
                callbacks.as_ref(),
                config,
                ClientSocket {
                    socket: connection,
                    cancellation_token: cancellation_token,
                    read_timeout: config.timeout_config.read_timeout_duration,
                },
            )
            .await
            {
                Ok(_) => {
                    println!("Connection from {} closed.", addr);
                }
                Err(e) => {
                    println!("Error handling connection from {}: {:?}", addr, e);
                }
            }
        }).detach();
        
    } 

    pub fn setup_https(
        cert_path: &str,
        key_path: &str,
    ) -> std::io::Result<futures_rustls::TlsAcceptor> {
        let certs = CertificateDer::from_pem_file(&cert_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load certificate: {}", e),
            )
        })?;
        let key = PrivateKeyDer::from_pem_file(&key_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to load private key: {}", e),
            )
        })?;

        let tls_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certs], key)
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create TLS config: {}", e),
                )
            })?;

        let acceptor = futures_rustls::TlsAcceptor::from(std::sync::Arc::new(tls_config));

        Ok(acceptor)
    }

    pub fn run_https(
        self,
        address: &str,
        port: &str,
        cert_path: &str,
        key_path: &str,
        config: HttpServerConfig,
    ) -> (smol::Task<std::io::Result<()>>, smol::channel::Sender<()>) {
        let (tx, rx) = smol::channel::bounded::<()>(1);
        let (cancel_tx, cancel_rx) = smol::channel::bounded::<()>(1);
        let address = address.to_string();
        let port = port.to_string();
        let callbacks = Arc::new(self.callbacks);
        let cert_path = cert_path.to_string();
        let key_path = key_path.to_string();
        let task = smol::spawn(async move {
            let server = TcpListener::bind(format!("{address}:{port}").as_str()).await?;
            println!("HTTPS Server listening on https://localhost:{port}/");

            let acceptor = Arc::new(Self::setup_https(&cert_path, &key_path)?);
            loop {
                let client_connection =
                    match Self::accept_connection(&server, &config, rx.clone(), cancel_tx.clone())
                        .await
                    {
                        Ok((stream, addr)) => (stream, addr),
                        Err(AcceptError::Shutdown) => {
                            println!("Server is shutting down.");
                            break;
                        }
                        Err(AcceptError::IoError(e)) => {
                            println!("Error accepting connection: {:?}", e);
                            continue;
                        }
                    };

                let (client, addr) = client_connection;
                let acceptor = acceptor.clone();

                match acceptor.accept(client).await {
                    Ok(tls_stream) => {
                        Self::run_connection(Arc::downgrade(&callbacks), config, cancel_rx.clone(), (tls_stream, addr));
                    }
                    Err(e) => {
                        println!("TLS handshake failed with {}: {:?}", addr, e);
                    }
                }
            }
            Ok(())
        });
        (task, tx)
    }
}

impl<'a> HttpCallbacks for HttpServer<'a> {
    type Request = Request;

    type Response = Response;

    fn add_callback(&mut self, callback: HttpListener<Self::Request, Self::Response>) {
        self.callbacks.push(callback);
    }
}

impl<'a> HttpMiddleware<'a> for HttpServer<'a> {
    fn add_middleware(&mut self, middleware_type: MiddlewareType, handler: MiddlewareHandler<'a>) {
        self.middlewares.push(MiddlewareEntry {
            middleware_type,
            handler,
        });
    }
}

pub mod prelude {
    pub use super::HttpCallbacks;
    pub use super::{HttpMiddleware, MiddlewareEntry, MiddlewareType};
    pub use crate::middleware::PathParameter;
    pub use crate::middleware::MiddlewareResult;
    pub use super::HttpServer;
}
