mod test;

use std::fmt::Display;

use crate::{
    client_socket::{ReadError, Socket, SocketReader},
    http_method::{HttpMethod, parse_method},
    http_server::HttpServerConfig,
    http_version::{HttpVersion, parse_http_version},
    map::{DuplicateMap, Map},
};

#[derive(Debug)]
pub enum RequestParsingError {
    UnhandledRequest,
    InvalidRequest,
    InvalidHeader,
    InvalidBody,
    PayloadTooLarge,
    IoError(std::io::Error),
    Timeout,
    Cancellation,
    UnexpectedError,
}

impl PartialEq for RequestParsingError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(l0), Self::IoError(r0)) => l0.kind() == r0.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Display for RequestParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestParsingError::UnhandledRequest => write!(f, "UnhandledRequest"),
            RequestParsingError::InvalidRequest => write!(f, "InvalidRequest"),
            RequestParsingError::InvalidHeader => write!(f, "InvalidHeader"),
            RequestParsingError::InvalidBody => write!(f, "InvalidBody"),
            RequestParsingError::PayloadTooLarge => write!(f, "PayloadTooLarge"),
            RequestParsingError::IoError(e) => write!(f, "IoError: {}", e),
            RequestParsingError::Timeout => write!(f, "Timeout"),
            RequestParsingError::Cancellation => write!(f, "Cancellation"),
            RequestParsingError::UnexpectedError => write!(f, "UnexpectedError"),
        }
    }
}

pub struct Request {
    pub method: HttpMethod,
    pub http_version: HttpVersion,
    pub body: Vec<u8>,
    pub path: String,
    pub query_params: Map<DuplicateMap>,
    pub headers: Map<DuplicateMap>,
    pub path_params: Map<String>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: HttpMethod::GET,
            http_version: HttpVersion::Http1_1,
            body: Default::default(),
            path: Default::default(),
            query_params: Default::default(),
            headers: Default::default(),
            path_params: Default::default(),
        }
    }
}

const DUPLICATABLE_HEADER_NAMES: [&'static str; 9] = [
    "set-cookie",
    "warning",
    "www-authenticate",
    "proxy-authenticate",
    "accept",
    "via",
    "accept-language",
    "link",
    "forwarded",
];

fn is_tchar(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

pub(crate) fn is_valid_header_name(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || is_tchar(c))
}

pub(crate) fn is_valid_header_value(value: &str) -> bool {
    value.bytes().all(|b| b == 9 || (b >= 32 && b != 127))
}

pub(crate) fn header_can_be_duplicate(name: &str) -> bool {
    return DUPLICATABLE_HEADER_NAMES
        .iter()
        .any(|header_name| header_name == &name);
}



pub(crate) fn parse_http_request_line(
    line: &str,
) -> Result<(HttpMethod, HttpVersion, String), RequestParsingError> {
    let tokens = line.split(" ").collect::<Vec<_>>();

    if tokens.len() < 3 {
        return Err(RequestParsingError::UnhandledRequest);
    }

    let http_method = match parse_method(tokens[0]) {
        Some(method) => method,
        None => return Err(RequestParsingError::UnhandledRequest),
    };

    let http_version = parse_http_version(tokens[2]).ok_or_else(|| RequestParsingError::UnhandledRequest)?;

    let path = tokens[1];

    return Ok((http_method, http_version, path.to_owned()));
}

pub(crate) fn parse_header_line(header: &str) -> Result<Option<(String, String)>, RequestParsingError> {
    if header.is_empty() {
        return Ok(None);
    }

    let (header_name, header_value) = match header.split_once(":") {
        Some(value) => (value.0.trim_end(), value.1.trim_start()),
        None => return Ok(None),
    };

    let header_name = header_name.to_lowercase();
    if !is_valid_header_name(&header_name) || !is_valid_header_value(header_value) {
        return Err(RequestParsingError::InvalidHeader);
    }

    return Ok(Some((header_name, header_value.to_owned())));
}

pub(crate) fn parse_headers<'a, T: Iterator<Item = &'a str>>(headers: T) -> Result<Map<DuplicateMap>, RequestParsingError> {
    let mut header_map: Map<DuplicateMap> = Map::default();

    for header in headers {
        match parse_header_line(header)? {
            Some((name, value)) => {
                if header_can_be_duplicate(&name) {
                    header_map.add(&name, value);
                } else {
                    match header_map.add_require_single(&name, value) {
                        Ok(_) => {}
                        Err(_) => return Err(RequestParsingError::InvalidHeader),
                    }
                }
            }
            None => {
                if !header.is_empty() {
                    return Err(RequestParsingError::InvalidHeader);
                }
            },
        }
    }

    Ok(header_map)
}

pub(crate) async fn parse_chunked_body<T: SocketReader>(
    client: &mut T,
    extra_bytes: Vec<u8>,
    config: HttpServerConfig,
) -> Result<Vec<u8>, ReadError> {
    let chunks = client.read_chunked(
        extra_bytes,
        b"\r\n",
        b"\r\n",
        config.size_config.request_body_max_size,
    );

    chunks.await
}

pub(crate) async fn parse_body_from_content_length<T: SocketReader>(
    client: &mut T,
    content_length: usize,
    extra_bytes: Vec<u8>,
    config: HttpServerConfig,
) -> Result<Vec<u8>, ReadError> {
    let mut body = vec![];
    let already_received = extra_bytes.len();
    body.extend(extra_bytes);
    
    if content_length > config.size_config.request_body_max_size {
        return Err(ReadError::MaxSizeExceeded);
    }
    
    if already_received < content_length {
        body.extend(client.read_n(content_length - already_received).await?);
    }

    return Ok(body);
}

pub(crate) async fn parse_body<T: SocketReader>(
    client: &mut T,
    header_map: &Map<DuplicateMap>,
    extra_bytes: Vec<u8>,
    config: HttpServerConfig,
) -> Result<Vec<u8>, RequestParsingError> {
    let body = if let Ok(Some(content_length)) = header_map.get_require_single("content-length") {
        /* DATA: In theory if we received more bytes than usize::max this would be an issue. */
        let content_length = match usize::from_str_radix(content_length, 10) {
            Ok(value) => value,
            Err(_) => return Err(RequestParsingError::InvalidHeader),
        };

        if content_length > config.size_config.request_body_max_size {
            return Err(RequestParsingError::PayloadTooLarge);
        }

        parse_body_from_content_length(client, content_length, extra_bytes, config).await
    } else if let Ok(Some(transfer_encoding)) = header_map.get_require_single("transfer-encoding") {
        if transfer_encoding != "chunked" {
            return Err(RequestParsingError::InvalidHeader);
        }

        parse_chunked_body(client, extra_bytes, config).await
    } else {
        Ok(vec![])
    };
    match body {
        Ok(v) => Ok(v),
        Err(ReadError::MaxSizeExceeded) => Err(RequestParsingError::PayloadTooLarge),
        Err(ReadError::IoError(e)) => Err(RequestParsingError::IoError(e)),
        Err(ReadError::Timeout) => Err(RequestParsingError::Timeout),
        Err(ReadError::Cancellation) => Err(RequestParsingError::Cancellation),
        Err(ReadError::UnexpectedError) => Err(RequestParsingError::InvalidBody),
    }
}

pub(crate) fn parse_query_params(path: &str) -> Map<DuplicateMap> {
    let mut query_params: Map<DuplicateMap> = Map::default();
    let query_start = match path.find("?") {
        Some(pos) => pos,
        None => path.len(),
    };

    if query_start < path.len() {
        let query_string = &path[query_start + 1..];
        for param in query_string.split("&") {
            let (key, value) = match param.split_once("=") {
                Some(kv) => kv,
                None => (param, ""),
            };
            query_params.add(key, value.to_owned());
        }
    }

    return query_params;
}

pub(crate) async fn parse_request<T: Socket>(
    client: &mut T,
    request_headers: Vec<u8>,
    extra_bytes: Vec<u8>,
    config: HttpServerConfig,
) -> Result<Request, RequestParsingError> {
    let headers_s = match String::from_utf8(request_headers) {
        Ok(headers) => headers,
        Err(_) => {
            return Err(RequestParsingError::InvalidRequest);
        }
    };

    let (header_line, headers) = match headers_s.split_once("\r\n") {
        Some(v) => v,
        None => return Err(RequestParsingError::InvalidRequest),
    };

    let (http_method, http_version, path) = parse_http_request_line(header_line)?;

    let header_map = parse_headers(headers.split("\r\n"))?;

    if matches!(http_version, HttpVersion::Http1_1) && !header_map.has("host") {
        return Err(RequestParsingError::InvalidRequest);
    }

    if header_map.has("content-length") && header_map.has("transfer-encoding") {
        return Err(RequestParsingError::InvalidRequest);
    }

    // Check for Expect: 100-continue header
    let needs_continue = header_map
        .get_single("expect")
        .is_some_and(|e| e.to_lowercase().contains("100-continue"));

    // If Expect: 100-continue is present, send 100 Continue response before reading body
    if needs_continue {
        let continue_response = b"HTTP/1.1 100 Continue\r\n\r\n";
        if let Err(e) = client.write_all(continue_response).await {
            return Err(RequestParsingError::IoError(e.into()));
        }
    }

    let body = parse_body(client, &header_map, extra_bytes, config).await?;

    let query_params = parse_query_params(&path);

    Ok(Request {
        path: path,
        http_version: http_version,
        method: http_method,
        body: body,
        headers: header_map,
        query_params: query_params,
        ..Default::default()
    })
}
