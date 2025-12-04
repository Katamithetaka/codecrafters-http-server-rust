use crate::mime_type::TEXT_PLAIN;

pub struct Response {
    pub content_type: String,
    pub bytes: Vec<u8>,
    pub status_code: u16,
}

pub fn status(status: u16) -> Response {
    return Response {
        content_type: TEXT_PLAIN.to_string(),
        bytes: Vec::new(),
        status_code: status,
    }
}

pub fn text<S: AsRef<str>>(text: S) -> Response {
    return Response {
        content_type: TEXT_PLAIN.to_string(),
        bytes: text.as_ref().as_bytes().to_vec(),
        status_code: 200,
    };
}

pub fn bytes(bytes: Vec<u8>) -> Response {
    return Response {
        content_type: TEXT_PLAIN.to_string(),
        bytes,
        status_code: 200,
    };
}
