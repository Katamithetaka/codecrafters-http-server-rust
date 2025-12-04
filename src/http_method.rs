#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HttpMethod {
    ALL,
    GET,
    POST,
    PUT,
    DELETE,
    OPTIONS,
    HEAD,
    CONNECT,
    TRACE,
    PATCH,
    UPDATE
}

pub fn parse_method(method: &str) -> Option<HttpMethod> {
    match method {
        "GET" => Some(HttpMethod::GET),
        "POST" => Some(HttpMethod::POST),
        "PUT" => Some(HttpMethod::PUT),
        "DELETE" => Some(HttpMethod::DELETE),
        "OPTIONS" => Some(HttpMethod::OPTIONS),
        "HEAD" => Some(HttpMethod::HEAD),
        "CONNECT" => Some(HttpMethod::CONNECT),
        "TRACE" => Some(HttpMethod::TRACE),
        "PATCH" => Some(HttpMethod::PATCH),
        "UPDATE" => Some(HttpMethod::UPDATE),
        _ => return None,
    }
}
