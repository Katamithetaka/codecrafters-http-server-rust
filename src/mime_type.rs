use std::fmt::Display;

// Text
pub const TEXT_PLAIN: MimeType       = MimeType::new("text/plain");
pub const TEXT_HTML: MimeType        = MimeType::new("text/html");
pub const TEXT_CSS: MimeType         = MimeType::new("text/css");
pub const TEXT_JAVASCRIPT: MimeType  = MimeType::new("text/javascript");
pub const TEXT_CSV: MimeType         = MimeType::new("text/csv");

// Application
pub const APPLICATION_JSON: MimeType           = MimeType::new("application/json");
pub const APPLICATION_XML: MimeType            = MimeType::new("application/xml");
pub const APPLICATION_OCTET_STREAM: MimeType   = MimeType::new("application/octet-stream");
pub const APPLICATION_PDF: MimeType            = MimeType::new("application/pdf");
pub const APPLICATION_ZIP: MimeType            = MimeType::new("application/zip");
pub const APPLICATION_FORM_URLENCODED: MimeType= MimeType::new("application/x-www-form-urlencoded");

// Images
pub const IMAGE_PNG: MimeType  = MimeType::new("image/png");
pub const IMAGE_JPEG: MimeType = MimeType::new("image/jpeg");
pub const IMAGE_GIF: MimeType  = MimeType::new("image/gif");
pub const IMAGE_WEBP: MimeType = MimeType::new("image/webp");

// Audio
pub const AUDIO_MPEG: MimeType = MimeType::new("audio/mpeg");
pub const AUDIO_OGG: MimeType  = MimeType::new("audio/ogg");

// Video
pub const VIDEO_MP4: MimeType = MimeType::new("video/mp4");
pub const VIDEO_WEBM: MimeType = MimeType::new("video/webm");

// Multipart
pub const MULTIPART_FORM_DATA: MimeType = MimeType::new("multipart/form-data");

pub const ALL: &[MimeType] = &[
    TEXT_PLAIN, TEXT_HTML, TEXT_CSS, TEXT_JAVASCRIPT, TEXT_CSV,
    APPLICATION_JSON, APPLICATION_XML, APPLICATION_OCTET_STREAM,
    APPLICATION_PDF, APPLICATION_ZIP, APPLICATION_FORM_URLENCODED,
    IMAGE_PNG, IMAGE_JPEG, IMAGE_GIF, IMAGE_WEBP,
    AUDIO_MPEG, AUDIO_OGG,
    VIDEO_MP4, VIDEO_WEBM,
    MULTIPART_FORM_DATA,
];


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MimeType {
    pub name: &'static str,
}

impl MimeType {
    pub const fn new(name: &'static str) -> Self {
        MimeType { name }
    }

    /// Convert a &str into a known MimeType
    pub fn from_str(s: &str) -> Option<Self> {
        for &mime in ALL {
            if mime.name == s {
                return Some(mime);
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
