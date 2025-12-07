#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum HttpVersion {
    Http1_0,
    Http1_1
}

pub fn parse_http_version(input: &str) -> Option<HttpVersion> {
    match input {
        "HTTP/1.0" => Some(HttpVersion::Http1_0),
        "HTTP/1.1" => Some(HttpVersion::Http1_1),
        _ => None,
    }
}
