use std::io::prelude::*;
use std::net::TcpStream;

use crate::socket::{BUFFER_SIZE, Bytes};

pub struct ClientSocket {
    pub socket: TcpStream,
}

impl ClientSocket {
    pub fn read_until(&mut self, delimiter: &[u8]) -> std::io::Result<(Bytes, Bytes)> {
        let mut buffer = [0; BUFFER_SIZE];
        let mut output_buffer = Bytes::new();
        loop {
            match self.socket.read(&mut buffer[..]) {
                Ok(0) => break,
                Ok(size) => {
                    output_buffer.extend_from_slice(&buffer[0..size]);
                    if let Some(index) = output_buffer
                        .windows(delimiter.len())
                        .position(|characters| characters == delimiter)
                    {
                        let (before, after) = output_buffer.split_at(index + delimiter.len());
                        return Ok((before.to_owned(), after.to_owned()));
                    }
                }
                Err(e) => return Err(e),
            }
        }

        return Ok((output_buffer, vec![]));
    }
    
    pub fn read_n(&mut self, size: usize) -> std::io::Result<Bytes> {
        let mut output_buffer = Bytes::new();
        let mut buffer = [0; BUFFER_SIZE];
        
        loop {
            let socket_read_result = if (size - output_buffer.len()) < BUFFER_SIZE {
                let read_size = size - output_buffer.len();
                self.socket.read(buffer[0..read_size].as_mut())
            }
            else {
                self.socket.read(&mut buffer)
            };
            
            match socket_read_result {
                Ok(0) => break,
                Ok(read_size) => {
                    output_buffer.extend_from_slice(&buffer[0..read_size]);
                    if output_buffer.len() >= size {
                        break;
                    }
                }
                Err(e) => return Err(e),
            }
        }
        
        return Ok(output_buffer);
    }
}
