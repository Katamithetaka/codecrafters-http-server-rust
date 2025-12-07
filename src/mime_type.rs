use std::{borrow::Cow, fmt::Display};

// Text
pub const TEXT_PLAIN: MimeType       = MimeType::new("text/plain", false);
pub const TEXT_HTML: MimeType        = MimeType::new("text/html", false);
pub const TEXT_CSS: MimeType         = MimeType::new("text/css", false);
pub const TEXT_JAVASCRIPT: MimeType  = MimeType::new("text/javascript", false);
pub const TEXT_CSV: MimeType         = MimeType::new("text/csv", false);

// Application
pub const APPLICATION_JSON: MimeType           = MimeType::new("application/json", false);
pub const APPLICATION_XML: MimeType            = MimeType::new("application/xml", false);
pub const APPLICATION_OCTET_STREAM: MimeType   = MimeType::new("application/octet-stream", true);
pub const APPLICATION_PDF: MimeType            = MimeType::new("application/pdf", true);
pub const APPLICATION_ZIP: MimeType            = MimeType::new("application/zip", true);
pub const APPLICATION_FORM_URLENCODED: MimeType= MimeType::new("application/x-www-form-urlencoded", false);

// Images
pub const IMAGE_PNG: MimeType  = MimeType::new("image/png", true);
pub const IMAGE_JPEG: MimeType = MimeType::new("image/jpeg", true);
pub const IMAGE_GIF: MimeType  = MimeType::new("image/gif", true);
pub const IMAGE_WEBP: MimeType = MimeType::new("image/webp", true);

// Audio
pub const AUDIO_MPEG: MimeType = MimeType::new("audio/mpeg", true);
pub const AUDIO_OGG: MimeType  = MimeType::new("audio/ogg", true);

// Video
pub const VIDEO_MP4: MimeType = MimeType::new("video/mp4", true);
pub const VIDEO_WEBM: MimeType = MimeType::new("video/webm", true);

// Multipart
pub const MULTIPART_FORM_DATA: MimeType = MimeType::new("multipart/form-data", false);

pub const ALL: &[MimeType] = &[
    TEXT_PLAIN, TEXT_HTML, TEXT_CSS, TEXT_JAVASCRIPT, TEXT_CSV,
    APPLICATION_JSON, APPLICATION_XML, APPLICATION_OCTET_STREAM,
    APPLICATION_PDF, APPLICATION_ZIP, APPLICATION_FORM_URLENCODED,
    IMAGE_PNG, IMAGE_JPEG, IMAGE_GIF, IMAGE_WEBP,
    AUDIO_MPEG, AUDIO_OGG,
    VIDEO_MP4, VIDEO_WEBM,
    MULTIPART_FORM_DATA,
];


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MimeType {
    pub name: Cow<'static, str>,
    pub is_binary : bool,
}

impl MimeType {
    pub const fn new(name: &'static str, is_binary: bool) -> Self {
        MimeType { name: Cow::Borrowed(name), is_binary }
    }

    /// Convert a &str into a known MimeType
    pub fn from_str(s: &str) -> Option<Self> {
        for mime in ALL {
            if mime.name == s {
                return Some(mime.clone());
            }
        }
        None
    }
}

impl Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
