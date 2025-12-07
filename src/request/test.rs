#![cfg(test)]

use macro_rules_attribute::apply;
use smol_macros::{test};

use crate::request::*;
use crate::{
    client_socket::{SocketReader, SocketWriter, WriteError},
};

struct MockSocketReader {
    data: Vec<u8>,
    position: usize,
}

impl SocketReader for MockSocketReader {
    async fn read_buffer(&mut self, buf: &mut [u8]) -> Result<usize, ReadError> {
        if self.position >= self.data.len() {
            return Ok(0); // EOF
        }

        let bytes_to_read = std::cmp::min(buf.len(), self.data.len() - self.position);
        buf[..bytes_to_read]
            .copy_from_slice(&self.data[self.position..self.position + bytes_to_read]);
        self.position += bytes_to_read;
        Ok(bytes_to_read)
    }
}

impl SocketWriter for MockSocketReader {
    async fn write_all(&mut self, _buf: &[u8]) -> Result<(), WriteError> {
        // Mock implementation - don't actually write anything in tests
        Ok(())
    }
}

#[test]
fn test_is_valid_header_name() {
    assert!(is_valid_header_name("content-type"));
    assert!(is_valid_header_name("x-custom-header"));
    assert!(!is_valid_header_name("invalid header"));
    assert!(!is_valid_header_name("invalid@header"));
}

#[test]
fn test_is_valid_header_value() {
    assert!(is_valid_header_value("application/json"));
    assert!(is_valid_header_value("Some value with spaces"));
    assert!(!is_valid_header_value("Invalid\x7FValue"));
    assert!(!is_valid_header_value("Invalid\x01Value"));
}

#[test]
fn test_parse_query_params() {
    let params = parse_query_params("/path?key1=value1&key2=value2&key1=value3");
    assert_eq!(
        params.get("key1").unwrap().as_slice(),
        &[&"value1".to_string(), &"value3".to_string()]
    );
    assert_eq!(
        params.get("key2").unwrap().as_slice(),
        &[&"value2".to_string()]
    );
}

#[apply(test!)]
async fn test_parse_chunked_body() {
    use crate::http_server::HttpServerConfig;

    let mut mock_socket = MockSocketReader {
        data: b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n".to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_chunked_body(&mut mock_socket, vec![], config).await
        .unwrap();
    assert_eq!(result, b"Wikipedia");
}

#[apply(test!)]
async fn test_parse_body_from_content_length() {
    use crate::http_server::HttpServerConfig;

    let mut mock_socket = MockSocketReader {
        data: b"Hello, World!".to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_body_from_content_length(&mut mock_socket, 13, vec![], config).await
        .unwrap();
    assert_eq!(result, b"Hello, World!");
}

#[test]
fn test_parse_http_request_line() {
    use crate::http_method::HttpMethod;
    use crate::http_version::HttpVersion;

    let (method, version, path) = parse_http_request_line("GET /index.html HTTP/1.1").unwrap();
    assert_eq!(method, HttpMethod::GET);
    assert_eq!(version, HttpVersion::Http1_1);
    assert_eq!(path, "/index.html");
}

#[test]
fn test_parse_header_line() {
    let (name, value) = parse_header_line("Content-Type: application/json")
        .unwrap()
        .unwrap();
    assert_eq!(name, "content-type");
    assert_eq!(value, "application/json");
}

#[test]
fn test_parse_headers() {
    let headers = vec![
        "Content-Type: application/json",
        "Set-Cookie: sessionId=abc123",
        "Set-Cookie: theme=dark",
    ];

    let header_map = parse_headers(headers.into_iter()).unwrap();
    assert_eq!(
        header_map.get("content-type").unwrap().as_slice(),
        &[&"application/json".to_string()]
    );
    assert_eq!(
        header_map.get("set-cookie").unwrap().as_slice(),
        &[&"sessionId=abc123".to_string(), &"theme=dark".to_string()]
    );
}

#[apply(test!)]
async fn test_parse_request() {
    use crate::http_server::HttpServerConfig;

    let request =
        "GET /path?key=value HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHello";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();

    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await
    .unwrap();

    assert_eq!(result.method, crate::http_method::HttpMethod::GET);
    assert_eq!(result.path, "/path?key=value");
    assert_eq!(
        result.query_params.get("key").unwrap().as_slice(),
        &[&"value".to_string()]
    );
    assert_eq!(
        result.headers.get("host").unwrap().as_slice(),
        &[&"example.com".to_string()]
    );
    assert_eq!(result.body, b"Hello");
}

#[apply(test!)]
async fn test_bad_request_missing_host() {
    use crate::http_server::HttpServerConfig;

    let request = "GET /path HTTP/1.1\r\nContent-Length: 5\r\n\r\nHello";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();

    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), RequestParsingError::InvalidRequest);
}

#[apply(test!)]
async fn test_bad_request_both_content_length_and_transfer_encoding() {
    use crate::http_server::HttpServerConfig;

    let request = "POST /path HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\nHello";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();

    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), RequestParsingError::InvalidRequest);
}

#[apply(test!)]
async fn test_bad_request_invalid_header() {
    use crate::http_server::HttpServerConfig;

    let request = "GET /path HTTP/1.1\r\nHost: example.com\r\nInvalid-Header\r\n\r\n";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();

    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), RequestParsingError::InvalidHeader);
}

#[apply(test!)]
async fn test_bad_request_invalid_request_line() {
    use crate::http_server::HttpServerConfig;

    let request = "INVALID_REQUEST_LINE\r\nHost: example.com\r\n\r\n";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();

    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await;
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), RequestParsingError::UnhandledRequest);
}

#[apply(test!)]
async fn test_bad_request_oversized_body() {
    use crate::http_server::HttpServerConfig;

    let request = "POST /path HTTP/1.1\r\nHost: example.com\r\nContent-Length: 20\r\n\r\nHello";
    let (headers_part, body_part) = request.split_once("\r\n\r\n").unwrap();
    let mut mock_socket = MockSocketReader {
        data: body_part.as_bytes().to_vec(),
        position: 0,
    };

    let mut config = HttpServerConfig::default();
    config.size_config.request_body_max_size = 10; // Set max size to 10 bytes

    let result = parse_request(
        &mut mock_socket,
        headers_part.as_bytes().to_vec(),
        vec![],
        config,
    ).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), RequestParsingError::PayloadTooLarge);
}

#[apply(test!)]
async fn test_parse_chunked_body_with_extra_bytes() {
    use crate::http_server::HttpServerConfig;

    let mut mock_socket = MockSocketReader {
        data: b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n".to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let extra_bytes = b"4\r\nWiki\r\n".to_vec();
    let result = parse_chunked_body(&mut mock_socket, extra_bytes, config).await
        .unwrap();
    assert_eq!(result, b"WikiWikipedia");
}

#[apply(test!)]
async fn test_parse_body_from_content_length_with_extra_bytes() {
    use crate::http_server::HttpServerConfig;

    let mut mock_socket = MockSocketReader {
        data: b", World!".to_vec(),
        position: 0,
    };

    let config = HttpServerConfig::default();
    let extra_bytes = b"Hello".to_vec();
    let result = parse_body_from_content_length(&mut mock_socket, 13, extra_bytes, config).await
        .unwrap();
    assert_eq!(result, b"Hello, World!");
}

#[apply(test!)]
async fn test_parse_body_with_extra_bytes() {
    use crate::http_server::HttpServerConfig;
    use crate::map::DuplicateMap;
    use crate::map::Map;

    let mut mock_socket = MockSocketReader {
        data: b", World!".to_vec(),
        position: 0,
    };

    let mut header_map: Map<DuplicateMap> = Map::default();
    header_map
        .add_require_single("content-length", "13".to_string())
        .unwrap();

    let config = HttpServerConfig::default();
    let extra_bytes = b"Hello".to_vec();
    let result = parse_body(&mut mock_socket, &header_map, extra_bytes, config)
        .await
        .unwrap();
    assert_eq!(result, b"Hello, World!");
}

#[apply(test!)]
async fn test_parse_body_with_chunked_encoding_and_extra_bytes() {
    use crate::http_server::HttpServerConfig;
    use crate::map::DuplicateMap;
    use crate::map::Map;

    let mut mock_socket = MockSocketReader {
        data: b"5\r\npedia\r\n0\r\n\r\n".to_vec(),
        position: 0,
    };

    let mut header_map: Map<DuplicateMap> = Map::default();
    header_map
        .add_require_single("transfer-encoding", "chunked".to_string())
        .unwrap();

    let config = HttpServerConfig::default();
    let extra_bytes = b"4\r\nWiki\r\n".to_vec();
    let result = parse_body(&mut mock_socket, &header_map, extra_bytes, config).await
        .unwrap();
    assert_eq!(result, b"Wikipedia");
}
