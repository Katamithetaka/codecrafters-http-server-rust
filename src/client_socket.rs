
use std::fmt::Display;

use futures::FutureExt;
use smol::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use smol::io::{AsyncReadExt};

#[derive(Debug)]
pub(crate) enum ReadError {
    IoError(std::io::Error),
    MaxSizeExceeded,
    Timeout,
    Cancellation,
    UnexpectedError,
}

impl PartialEq for ReadError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(l0), Self::IoError(r0)) => l0.kind() == r0.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

#[derive(Debug)]
pub(crate) enum WriteError {
    IoError(std::io::Error),
    Timeout,
    Cancellation,
    #[allow(unused)]
    UnexpectedError,
}

impl PartialEq for WriteError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(l0), Self::IoError(r0)) => l0.kind() == r0.kind(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Into<std::io::Error> for WriteError {
    fn into(self) -> std::io::Error {
        match self {
            WriteError::IoError(e) => e,
            WriteError::Timeout => std::io::Error::new(std::io::ErrorKind::TimedOut, "Write timeout"),
            WriteError::Cancellation => std::io::Error::new(std::io::ErrorKind::Interrupted, "Write cancelled"),
            WriteError::UnexpectedError => std::io::Error::new(std::io::ErrorKind::Other, "Unexpected error"),
        }
    }
}



impl Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::IoError(e) => write!(f, "IO Error: {}", e),
            ReadError::MaxSizeExceeded => write!(f, "Max size exceeded"),
            ReadError::Timeout => write!(f, "Read timeout"),
            ReadError::Cancellation => write!(f, "Read cancelled"),
            ReadError::UnexpectedError => write!(f, "Unexpected error"),
        }
    }
}


impl Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::IoError(e) => write!(f, "IO Error: {}", e),
            WriteError::Timeout => write!(f, "Read timeout"),
            WriteError::Cancellation => write!(f, "Read cancelled"),
            WriteError::UnexpectedError => write!(f, "Unexpected error"),
        }
    }
}


use crate::{socket::{BUFFER_SIZE, Bytes}, utils::bytes_contain};
pub(crate) trait SocketReader {
    
    async fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, ReadError>;
    async fn read_n(&mut self, size: usize) -> Result<Bytes, ReadError> {
        let mut output_buffer = Bytes::new();
        let mut buffer = [0; BUFFER_SIZE];
        
        loop {
            let socket_read_result = if (size - output_buffer.len()) < BUFFER_SIZE {
                let read_size = size - output_buffer.len();
                self.read_buffer(buffer[0..read_size].as_mut())
            }
            else {
                self.read_buffer(&mut buffer)
            };
            
            match socket_read_result.await {
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
    async fn read_until(&mut self, delimiter: &[u8], max_size: usize) -> Result<(Bytes, Bytes), ReadError> {
        let mut buffer = [0; BUFFER_SIZE];
        let mut output_buffer = Bytes::new();
        loop {
            let buffer_size = if (max_size - output_buffer.len()) < BUFFER_SIZE {
                max_size - output_buffer.len()
            } else {
                BUFFER_SIZE
            };
            match self.read_buffer(&mut buffer[..buffer_size]).await {
                Ok(0) => return Ok((output_buffer, vec![])),
                Ok(size) => {
                    output_buffer.extend_from_slice(&buffer[0..size]);
                    if let Some(index) = output_buffer
                        .windows(delimiter.len())
                        .position(|characters| characters == delimiter)
                    {
                        let (before, after) = output_buffer.split_at(index + delimiter.len());
                        return Ok((before.to_owned(), after.to_owned()));
                    }
                    
                    if output_buffer.len() >= max_size {
                        return Err(ReadError::MaxSizeExceeded);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
    async fn read_chunked(&mut self, extra_bytes: Vec<u8>, chunk_size_delim: &[u8], chunk_delim: &[u8], max_size: usize) -> Result<Bytes, ReadError> {
        let mut output_buffer = Bytes::new();
        let mut extra_bytes_out = extra_bytes;
        loop {
            if output_buffer.len() >= max_size {
                break;
            }
            
            let mut chunk_size_bytes = Bytes::new();
            let mut extra_bytes = Bytes::new();
            
            if !extra_bytes_out.is_empty() {
                while !bytes_contain(&extra_bytes_out, chunk_size_delim) {
                    let needed_size = 5;
                    
                    if output_buffer.len() + needed_size + extra_bytes_out.len() > max_size {
                        return Err(ReadError::MaxSizeExceeded)
                    }
                    
                    let read_bytes = self.read_n(needed_size).await?;
                    extra_bytes_out.extend_from_slice(&read_bytes);
                }
                
                let index = match extra_bytes_out
                    .windows(chunk_size_delim.len())
                    .position(|characters| characters == chunk_size_delim) {
                        Some(i) => i,
                        None => return Err(ReadError::UnexpectedError)
                    }
                    + chunk_size_delim.len();
                chunk_size_bytes.extend_from_slice(&extra_bytes_out[0..index]);
                extra_bytes.extend_from_slice(&extra_bytes_out[index..]);
                extra_bytes_out.clear();
            }
            else {
                let (read_bytes, remaining_bytes) = self.read_until(chunk_size_delim, max_size - output_buffer.len()).await?;
                chunk_size_bytes.extend_from_slice(&read_bytes);
                extra_bytes.extend_from_slice(&remaining_bytes);
            }
            
            let chunk_size_str = String::from_utf8_lossy(&chunk_size_bytes[..chunk_size_bytes.len() - chunk_size_delim.len()]);
            let chunk_size = usize::from_str_radix(chunk_size_str.trim(), 16).unwrap_or(0);
            if chunk_size == 0 {
                break;
            }
            
            if output_buffer.len() + chunk_size > max_size {
                let allowed_size = max_size - output_buffer.len();
                output_buffer.extend_from_slice(&extra_bytes[0..allowed_size]);
                return Err(ReadError::MaxSizeExceeded)
            }
                    
            let already_read = extra_bytes.len();
            if already_read >= (chunk_size + chunk_delim.len()) {
                output_buffer.extend_from_slice(&extra_bytes[0..chunk_size]);
                extra_bytes_out = extra_bytes[chunk_size + chunk_delim.len()..].to_owned();
                continue;
            }
            else {
                output_buffer.extend_from_slice(&extra_bytes);
                let remaining_size = (chunk_size + chunk_delim.len()) - already_read;
                
                
                let chunk_data = self.read_n(remaining_size).await?;
                output_buffer.extend_from_slice(&chunk_data[0..chunk_size]);
                extra_bytes.clear();
            }
            
        }
        
        if output_buffer.len() >= max_size {
            return Err(ReadError::MaxSizeExceeded);
        }
        else {
            Ok(output_buffer)
        }
    }
}

pub(crate) trait SocketWriter {
    async fn write_all(&mut self, data: &[u8]) -> Result<(), WriteError>;
}


pub(crate) trait Socket: SocketReader + SocketWriter {}

impl <T: SocketReader + SocketWriter> Socket for T {}


pub struct ClientSocket<T: AsyncRead + AsyncWrite + Unpin>  {
    pub socket: T,
    pub cancellation_token: smol::channel::Receiver<()>,
    pub read_timeout: std::time::Duration,
}

impl<T: AsyncRead + AsyncWrite + Unpin> SocketReader for ClientSocket<T> {
    async fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<usize, ReadError> {
        futures::select! {
            read_result = self.socket.read(buffer).fuse() => {
                match read_result {
                    Ok(size) => Ok(size),
                    Err(e) => Err(ReadError::IoError(e)),
                }
            },
            _ = smol::Timer::after(self.read_timeout).fuse() => {
                Err(ReadError::Timeout)
            },
            _ = self.cancellation_token.recv().fuse() => {
                Err(ReadError::Cancellation)
            },
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> SocketWriter for ClientSocket<T> {
    async fn write_all(&mut self, data: &[u8]) -> Result<(), WriteError> {
        futures::select! {
            write_result = self.socket.write_all(data).fuse() => {
                match write_result {
                    Ok(size) => Ok(size),
                    Err(e) => Err(WriteError::IoError(e)),
                }
            },
            _ = smol::Timer::after(self.read_timeout).fuse() => {
                Err(WriteError::Timeout)
            },
            _ = self.cancellation_token.recv().fuse() => {
                Err(WriteError::Cancellation)
            },
        }
    }
}
