use std::borrow::Cow;

use crate::{mime_type::{APPLICATION_OCTET_STREAM, MimeType, TEXT_PLAIN}, status_code::{OK, StatusCode}};

#[derive(Debug, Clone)]
pub struct Response {
    pub content_type: MimeType,
    pub bytes: Vec<u8>,
    pub status_code: StatusCode,
    pub headers: Vec<(String, String)>,
}

impl Response {
    pub fn header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
    
    pub fn status<T: Into<StatusCode>>(mut self, status: T) -> Self {
        self.status_code = status.into();
        self
    }

    pub fn content_type<S: AsRef<str>>(mut self, content_type: S) -> Self {
        self.content_type = MimeType::from_str(content_type.as_ref()).unwrap_or(MimeType { name: Cow::Owned(content_type.as_ref().to_string()), is_binary: false });
        self
    }

    pub fn body<B: AsRef<[u8]>>(mut self, body: B) -> Self {
        self.bytes = body.as_ref().to_vec();
        self
    }
}



impl Into<StatusCode> for u16 {
    fn into(self) -> StatusCode {
        StatusCode::from_u16(self).unwrap_or(StatusCode { code: self, reason: "Unknown" })
    }
}

pub fn status<T: Into<StatusCode>>(status: T) -> Response {
    return Response {
        content_type: TEXT_PLAIN,
        bytes: Vec::new(),
        status_code: status.into(),
        headers: Vec::new(),
    }
}

pub fn text<S: AsRef<str>>(text: S) -> Response {
    return Response {
        content_type: TEXT_PLAIN,
        bytes: text.as_ref().as_bytes().to_vec(),
        status_code: OK,
        headers: Vec::new(),
    };
}

pub fn bytes(bytes: Vec<u8>) -> Response {
    return Response {
        content_type: APPLICATION_OCTET_STREAM,
        bytes,
        status_code: OK,
        headers: Vec::new(),
    };
}

pub fn text_response<T: Into<StatusCode>, S: AsRef<str>>(status: T, content_type: S, bytes: Vec<u8>) -> Response {
    return Response {
        content_type: MimeType::from_str(content_type.as_ref()).unwrap_or(MimeType { name: Cow::Owned(content_type.as_ref().to_string()), is_binary: false }),
        bytes,
        status_code: status.into(),
        headers: Vec::new(),
    };
}

pub fn binary_response<T: Into<StatusCode>, S: AsRef<str>>(status: T, content_type: S, bytes: Vec<u8>) -> Response {
    return Response {
        content_type: MimeType::from_str(content_type.as_ref()).unwrap_or(MimeType { name: Cow::Owned(content_type.as_ref().to_string()), is_binary: true }),
        bytes,
        status_code: status.into(),
        headers: Vec::new(),
    };
}

pub fn response<T: Into<StatusCode>>(status: T, content_type: MimeType, bytes: Vec<u8>) -> Response {
    return Response {
        content_type,
        bytes,
        status_code: status.into(),
        headers: Vec::new(),
    };
}

pub fn empty() -> Response {
    return Response {
        content_type: TEXT_PLAIN,
        bytes: Vec::new(),
        status_code: OK,
        headers: Vec::new(),
    };
}

pub fn redirect<S: AsRef<str>>(location: S) -> Response {
    return Response {
        content_type: TEXT_PLAIN,
        bytes: Vec::new(),
        status_code: StatusCode::from_u16(302).unwrap(),
        headers: vec![("Location".to_string(), location.as_ref().to_string())],
    };
}
