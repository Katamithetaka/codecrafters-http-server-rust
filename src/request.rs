use crate::{
    client_socket::ClientSocket,
    http_method::{HttpMethod, parse_method},
    response::{Response, status},
    utils::bytes_contain,
};

#[derive(Clone)]
pub enum DuplicateMap {
    Single(String),
    List(Vec<String>),
}

#[derive(Clone)]
pub struct Map<T> {
    params: Vec<(String, T)>,
}

impl<T> Map<T> {
    pub fn has(&self, index: &str) -> bool {
        return self.params.iter().any(|key| key.0 == index);
    }

    pub fn get(&self, index: &str) -> Option<&T> {
        return self
            .params
            .iter()
            .find(|x| x.0.as_str() == index)
            .map(|value| &value.1);
    }
}

impl Map<DuplicateMap> {
    pub fn add(&mut self, key: &str, value: String) {
        for entry in self.params.iter_mut() {
            if &entry.0 == key {
                match &mut entry.1 {
                    DuplicateMap::Single(original_value) => {
                        entry.1 = DuplicateMap::List(vec![original_value.clone(), value]);
                        return;
                    }
                    DuplicateMap::List(items) => {
                        items.push(value);
                        return;
                    }
                }
            }
        }
        self.params
            .push((key.to_owned(), DuplicateMap::Single(value)))
    }
}

impl Map<String> {
    pub fn add(&mut self, key: &str, value: String) {
        self.params.push((key.to_owned(), value))
    }
}

impl<T> Default for Map<T> {
    fn default() -> Self {
        Self {
            params: Default::default(),
        }
    }
}

pub struct Request {
    pub method: HttpMethod,
    pub body: Vec<u8>,
    pub path: String,
    pub query_params: Map<DuplicateMap>,
    pub headers: Map<String>,
    pub path_params: Map<String>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: HttpMethod::GET,
            body: Default::default(),
            path: Default::default(),
            query_params: Default::default(),
            headers: Default::default(),
            path_params: Default::default(),
        }
    }
}

const DUPLICATABLE_HEADER_NAMES: [&'static str; 9] = [
    "set-cookie",
    "warning",
    "www-authenticate",
    "proxy-authenticate",
    "accept",
    "via",
    "accept-language",
    "link",
    "forwarded",
];

fn is_tchar(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '-'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
        )
}

fn is_valid_header_name(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || is_tchar(c))
}

fn is_valid_header_value(value: &str) -> bool {
    value.bytes().all(|b| b == 9 || (b >= 32 && b != 127))
}

fn header_can_be_duplicate(name: &str) -> bool {
    return DUPLICATABLE_HEADER_NAMES
        .iter()
        .any(|header_name| header_name == &name);
}

fn bad_request() -> std::io::Result<Result<Request, Response>> {
    return Ok(Err(status(400)));
}

pub fn parse_request(
    client: &mut ClientSocket,
    request_headers: Vec<u8>,
    extra_bytes: Vec<u8>,
) -> std::io::Result<Result<Request, Response>> {
    let headers_s = match String::from_utf8(request_headers) {
        Ok(headers) => headers,
        Err(err) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ));
        }
    };

    let (header_line, headers) = match headers_s.split_once("\r\n").ok_or_else(|| status(400)) {
        Ok(v) => v,
        Err(e) => return Ok(Err(e)),
    };

    let tokens = header_line.split(" ").collect::<Vec<_>>();

    if tokens.len() < 3 {
        return bad_request();
    }

    let http_method = match parse_method(tokens[0]) {
        Some(method) => method,
        None => return Ok(Err(status(400))),
    };

    if tokens[2] != "HTTP/1.1" && tokens[2] != "HTTP/1.0" {
        return Ok(Err(status(505)));
    }

    let path = tokens[1];


    let mut header_map: Map<String> = Map::default();
    for header in headers.split("\r\n") {
        if header.is_empty() {
            continue;
        }

        let (header_name, header_value) = match header.split_once(":") {
            Some(value) => (value.0.trim_end(), value.1.trim_start()),
            None => continue,
        };

        let header_name = header_name.to_lowercase();
        if !is_valid_header_name(&header_name) || !is_valid_header_value(header_value) {
            return bad_request();
        }

        if header_map.has(&header_name) && !header_can_be_duplicate(&header_name) {
            return bad_request();
        }

        header_map.add(&header_name, header_value.to_owned());
    }
    
    for key in header_map.params.iter().map(|(k, _)| k) {
        println!("Header length {}", key.len());
    }

    if tokens[2] == "HTTP/1.1" && !header_map.has("host") {
        return bad_request();
    }

    if header_map.has("content-length") && header_map.has("transfer-encoding") {
        return bad_request();
    }

    let mut body = vec![];
    if header_map.has("content-length") {
        let content_length = match header_map.get("content-length") {
            Some(content_length) => content_length,
            None => return bad_request(),
        };

        /* DATA: In theory if we received more bytes than usize::max this would be an issue. */
        let content_length = match usize::from_str_radix(content_length, 10) {
            Ok(value) => value,
            Err(_) => return bad_request(),
        };

        let already_received = extra_bytes.len();
        body.extend(extra_bytes);
        if already_received < content_length {
            body.extend(client.read_n(content_length - already_received)?);
        }
    } else if header_map.has("transfer-encoding") {
        let transfer_encoding = match header_map.get("transfer-encoding") {
            Some(transfer_encoding) => transfer_encoding,
            None => return bad_request(),
        };

        if transfer_encoding != "chunked" {
            return bad_request();
        }

        let mut buffer = extra_bytes;
        if !bytes_contain(&buffer, "\r\n0\r\n".as_bytes()) {
            let (read, _) = client.read_until("\r\n0\r\n".as_bytes())?;
            buffer.extend(read);
        }

        let mut index = 0;
        loop {
            // from this point onwards, we have all the data to be read in buffer and we don't need to read with the client anymore

            let line_end = match buffer[index..].windows(2).position(|w| w == b"\r\n") {
                Some(pos) => pos + index,
                None => return bad_request(),
            };

            let chunk_size_s = match String::from_utf8(buffer[index..line_end].to_vec()) {
                Ok(s) => s,
                Err(_) => return bad_request(),
            };

            let chunk_size = match usize::from_str_radix(chunk_size_s.trim(), 16) {
                Ok(size) => size,
                Err(_) => return bad_request(),
            };

            index = line_end + 2;
            if chunk_size == 0 {
                break;
            }
            if buffer.len() < index + chunk_size + 2 {
                return bad_request();
            }
            body.extend(&buffer[index..index + chunk_size]);
            index += chunk_size + 2; // skip \r\n            
        }
    }

    let mut query_params: Map<DuplicateMap> = Map::default();
    let query_start = match path.find("?") {
        Some(pos) => pos,
        None => path.len(),
    };

    if query_start < path.len() {
        let query_string = &path[query_start + 1..];
        for param in query_string.split("&") {
            let (key, value) = match param.split_once("=") {
                Some(kv) => kv,
                None => (param, ""),
            };
            query_params.add(key, value.to_owned());
        }
    }

    Ok(Ok(Request {
        path: path.to_owned(),
        method: http_method,
        body: body,
        headers: header_map,
        query_params: query_params,
        ..Default::default()
    }))
}
