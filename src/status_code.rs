use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode {
    pub code: u16,
    pub reason: &'static str,
}

impl StatusCode {
    pub const fn new(code: u16, reason: &'static str) -> Self {
        StatusCode { code, reason }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code, self.reason)
    }
}

// ---------------------
//  Status Code Constants
// ---------------------

// 1xx — Informational
pub const CONTINUE: StatusCode = StatusCode::new(100, "Continue");
pub const SWITCHING_PROTOCOLS: StatusCode = StatusCode::new(101, "Switching Protocols");
pub const PROCESSING: StatusCode = StatusCode::new(102, "Processing");
pub const EARLY_HINTS: StatusCode = StatusCode::new(103, "Early Hints");

// 2xx — Success
pub const OK: StatusCode = StatusCode::new(200, "OK");
pub const CREATED: StatusCode = StatusCode::new(201, "Created");
pub const ACCEPTED: StatusCode = StatusCode::new(202, "Accepted");
pub const NON_AUTHORITATIVE_INFORMATION: StatusCode = StatusCode::new(203, "Non-Authoritative Information");
pub const NO_CONTENT: StatusCode = StatusCode::new(204, "No Content");
pub const RESET_CONTENT: StatusCode = StatusCode::new(205, "Reset Content");
pub const PARTIAL_CONTENT: StatusCode = StatusCode::new(206, "Partial Content");
pub const MULTI_STATUS: StatusCode = StatusCode::new(207, "Multi-Status");
pub const ALREADY_REPORTED: StatusCode = StatusCode::new(208, "Already Reported");
pub const IM_USED: StatusCode = StatusCode::new(226, "IM Used");

// 3xx — Redirection
pub const MULTIPLE_CHOICES: StatusCode = StatusCode::new(300, "Multiple Choices");
pub const MOVED_PERMANENTLY: StatusCode = StatusCode::new(301, "Moved Permanently");
pub const FOUND: StatusCode = StatusCode::new(302, "Found");
pub const SEE_OTHER: StatusCode = StatusCode::new(303, "See Other");
pub const NOT_MODIFIED: StatusCode = StatusCode::new(304, "Not Modified");
pub const USE_PROXY: StatusCode = StatusCode::new(305, "Use Proxy");
pub const TEMPORARY_REDIRECT: StatusCode = StatusCode::new(307, "Temporary Redirect");
pub const PERMANENT_REDIRECT: StatusCode = StatusCode::new(308, "Permanent Redirect");

// 4xx — Client Errors
pub const BAD_REQUEST: StatusCode = StatusCode::new(400, "Bad Request");
pub const UNAUTHORIZED: StatusCode = StatusCode::new(401, "Unauthorized");
pub const PAYMENT_REQUIRED: StatusCode = StatusCode::new(402, "Payment Required");
pub const FORBIDDEN: StatusCode = StatusCode::new(403, "Forbidden");
pub const NOT_FOUND: StatusCode = StatusCode::new(404, "Not Found");
pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode::new(405, "Method Not Allowed");
pub const NOT_ACCEPTABLE: StatusCode = StatusCode::new(406, "Not Acceptable");
pub const PROXY_AUTHENTICATION_REQUIRED: StatusCode = StatusCode::new(407, "Proxy Authentication Required");
pub const REQUEST_TIMEOUT: StatusCode = StatusCode::new(408, "Request Timeout");
pub const CONFLICT: StatusCode = StatusCode::new(409, "Conflict");
pub const GONE: StatusCode = StatusCode::new(410, "Gone");
pub const LENGTH_REQUIRED: StatusCode = StatusCode::new(411, "Length Required");
pub const PRECONDITION_FAILED: StatusCode = StatusCode::new(412, "Precondition Failed");
pub const PAYLOAD_TOO_LARGE: StatusCode = StatusCode::new(413, "Payload Too Large");
pub const URI_TOO_LONG: StatusCode = StatusCode::new(414, "URI Too Long");
pub const UNSUPPORTED_MEDIA_TYPE: StatusCode = StatusCode::new(415, "Unsupported Media Type");
pub const RANGE_NOT_SATISFIABLE: StatusCode = StatusCode::new(416, "Range Not Satisfiable");
pub const EXPECTATION_FAILED: StatusCode = StatusCode::new(417, "Expectation Failed");
pub const IM_A_TEAPOT: StatusCode = StatusCode::new(418, "I'm a teapot");
pub const MISDIRECTED_REQUEST: StatusCode = StatusCode::new(421, "Misdirected Request");
pub const UNPROCESSABLE_CONTENT: StatusCode = StatusCode::new(422, "Unprocessable Content");
pub const LOCKED: StatusCode = StatusCode::new(423, "Locked");
pub const FAILED_DEPENDENCY: StatusCode = StatusCode::new(424, "Failed Dependency");
pub const TOO_EARLY: StatusCode = StatusCode::new(425, "Too Early");
pub const UPGRADE_REQUIRED: StatusCode = StatusCode::new(426, "Upgrade Required");
pub const PRECONDITION_REQUIRED: StatusCode = StatusCode::new(428, "Precondition Required");
pub const TOO_MANY_REQUESTS: StatusCode = StatusCode::new(429, "Too Many Requests");
pub const REQUEST_HEADER_FIELDS_TOO_LARGE: StatusCode = StatusCode::new(431, "Request Header Fields Too Large");
pub const UNAVAILABLE_FOR_LEGAL_REASONS: StatusCode = StatusCode::new(451, "Unavailable For Legal Reasons");

// 5xx — Server Errors
pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode::new(500, "Internal Server Error");
pub const NOT_IMPLEMENTED: StatusCode = StatusCode::new(501, "Not Implemented");
pub const BAD_GATEWAY: StatusCode = StatusCode::new(502, "Bad Gateway");
pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode::new(503, "Service Unavailable");
pub const GATEWAY_TIMEOUT: StatusCode = StatusCode::new(504, "Gateway Timeout");
pub const HTTP_VERSION_NOT_SUPPORTED: StatusCode = StatusCode::new(505, "HTTP Version Not Supported");
pub const VARIANT_ALSO_NEGOTIATES: StatusCode = StatusCode::new(506, "Variant Also Negotiates");
pub const INSUFFICIENT_STORAGE: StatusCode = StatusCode::new(507, "Insufficient Storage");
pub const LOOP_DETECTED: StatusCode = StatusCode::new(508, "Loop Detected");
pub const NOT_EXTENDED: StatusCode = StatusCode::new(510, "Not Extended");
pub const NETWORK_AUTHENTICATION_REQUIRED: StatusCode = StatusCode::new(511, "Network Authentication Required");


// ---------------------------------------------------
//     Static Slice of All Status Codes
// ---------------------------------------------------

pub const ALL: &[StatusCode] = &[
    CONTINUE, SWITCHING_PROTOCOLS, PROCESSING, EARLY_HINTS,
    OK, CREATED, ACCEPTED, NON_AUTHORITATIVE_INFORMATION, NO_CONTENT,
    RESET_CONTENT, PARTIAL_CONTENT, MULTI_STATUS, ALREADY_REPORTED, IM_USED,
    MULTIPLE_CHOICES, MOVED_PERMANENTLY, FOUND, SEE_OTHER, NOT_MODIFIED,
    USE_PROXY, TEMPORARY_REDIRECT, PERMANENT_REDIRECT,
    BAD_REQUEST, UNAUTHORIZED, PAYMENT_REQUIRED, FORBIDDEN, NOT_FOUND,
    METHOD_NOT_ALLOWED, NOT_ACCEPTABLE, PROXY_AUTHENTICATION_REQUIRED,
    REQUEST_TIMEOUT, CONFLICT, GONE, LENGTH_REQUIRED, PRECONDITION_FAILED,
    PAYLOAD_TOO_LARGE, URI_TOO_LONG, UNSUPPORTED_MEDIA_TYPE,
    RANGE_NOT_SATISFIABLE, EXPECTATION_FAILED, IM_A_TEAPOT,
    MISDIRECTED_REQUEST, UNPROCESSABLE_CONTENT, LOCKED, FAILED_DEPENDENCY,
    TOO_EARLY, UPGRADE_REQUIRED, PRECONDITION_REQUIRED, TOO_MANY_REQUESTS,
    REQUEST_HEADER_FIELDS_TOO_LARGE, UNAVAILABLE_FOR_LEGAL_REASONS,
    INTERNAL_SERVER_ERROR, NOT_IMPLEMENTED, BAD_GATEWAY, SERVICE_UNAVAILABLE,
    GATEWAY_TIMEOUT, HTTP_VERSION_NOT_SUPPORTED, VARIANT_ALSO_NEGOTIATES,
    INSUFFICIENT_STORAGE, LOOP_DETECTED, NOT_EXTENDED,
    NETWORK_AUTHENTICATION_REQUIRED,
];


impl StatusCode {

    pub fn from_u16(code: u16) -> Option<Self> {
        for sc in ALL {
            if sc.code == code {
                return Some(*sc);
            }
        }
        None
    }
}
