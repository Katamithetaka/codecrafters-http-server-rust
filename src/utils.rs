use crate::socket::Bytes;

pub fn bytes_contain(bytes: &Bytes, delimiter: &[u8]) -> bool {
    bytes
        .windows(delimiter.len())
        .any(|characters| characters == delimiter)
}

pub fn bytes_split(bytes: &Bytes, delimiter: &[u8]) -> Option<(Bytes, Bytes)> {
    if let Some(position) = bytes
        .windows(delimiter.len())
        .position(|characters| characters == delimiter)
    {
        let before_delimiter = bytes[..position].to_vec();
        let after_delimiter = bytes[position + delimiter.len()..].to_vec();
        Some((before_delimiter, after_delimiter))
    } else {
        None
    }
}


pub fn gzip_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}
