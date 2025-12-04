use crate::socket::Bytes;

pub fn bytes_contain(bytes: &Bytes, delimiter: &[u8]) -> bool {
    bytes
        .windows(delimiter.len())
        .any(|characters| characters == delimiter)
}


pub fn gzip_compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    encoder.finish()
}
