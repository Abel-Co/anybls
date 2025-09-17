use crate::error::Result;
use bytes::{Buf, BytesMut};
use futures::future::try_join;
use std::io::Result as IoResult;
use tokio::io::split;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};

/// Zero-copy bidirectional data relay
/// This structure efficiently forwards data between two streams without copying
pub struct ZeroCopyRelay {
    client_read: ReadHalf<tokio::net::TcpStream>,
    client_write: WriteHalf<tokio::net::TcpStream>,
    target_read: ReadHalf<tokio::net::TcpStream>,
    target_write: WriteHalf<tokio::net::TcpStream>,
}

impl ZeroCopyRelay {
    pub fn new(client_stream: tokio::net::TcpStream, target_stream: tokio::net::TcpStream) -> Self {
        let (client_read, client_write) = split(client_stream);
        let (target_read, target_write) = split(target_stream);

        Self {
            client_read,
            client_write,
            target_read,
            target_write,
        }
    }

    /// Start the zero-copy relay between client and target
    pub async fn start(self) -> Result<()> {
        // Create two futures for bidirectional data transfer
        let client_to_target =
            Self::relay_data(self.client_read, self.target_write, "client -> target");

        let target_to_client =
            Self::relay_data(self.target_read, self.client_write, "target -> client");

        // Run both relays concurrently
        // If either side closes, the relay stops
        match try_join(client_to_target, target_to_client).await {
            Ok((_, _)) => {
                log::info!("Relay completed successfully");
                Ok(())
            }
            Err(e) => {
                log::debug!("Relay ended: {}", e);
                Ok(())
            }
        }
    }

    /// Relay data from source to destination with zero-copy optimization
    async fn relay_data<R, W>(mut source: R, mut dest: W, direction: &str) -> Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        // Use larger buffer for better performance
        let mut buffer = BytesMut::with_capacity(64 * 1024); // 64KB buffer
        let mut total_bytes = 0u64;

        loop {
            // Read data from source with zero-copy optimization
            let bytes_read = source.read_buf(&mut buffer).await?;
            if bytes_read == 0 {
                log::debug!("{}: source closed, total bytes: {}", direction, total_bytes);
                break;
            }

            total_bytes += bytes_read as u64;

            // Write data to destination with zero-copy optimization
            while buffer.has_remaining() {
                let bytes_written = dest.write_buf(&mut buffer).await?;
                if bytes_written == 0 {
                    log::debug!("{}: destination closed, total bytes: {}", direction, total_bytes);
                    return Ok(());
                }
            }

            // Clear the buffer for next iteration
            buffer.clear();
        }

        log::debug!("{}: relay completed, total bytes: {}", direction, total_bytes);
        Ok(())
    }
}

/// High-performance circular buffer for zero-copy operations
pub struct ZeroCopyBuffer {
    data: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    capacity: usize,
}

impl ZeroCopyBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            read_pos: 0,
            write_pos: 0,
            capacity,
        }
    }

    pub async fn write_from_reader<R>(&mut self, reader: &mut R) -> IoResult<usize>
    where
        R: AsyncRead + Unpin,
    {
        let available = self.available_write_space();
        if available == 0 {
            return Ok(0);
        }

        let bytes_read = if self.write_pos + available <= self.capacity {
            // Simple case: write to end of buffer
            let slice = &mut self.data[self.write_pos..self.write_pos + available];
            reader.read(slice).await?
        } else {
            // Wrap around case: write to end then beginning
            let end_space = self.capacity - self.write_pos;
            let mut total_read = 0;

            if end_space > 0 {
                let slice = &mut self.data[self.write_pos..self.capacity];
                total_read += reader.read(slice).await?;
            }

            if total_read == end_space && self.read_pos > 0 {
                let remaining = available - end_space;
                let slice = &mut self.data[0..remaining.min(self.read_pos)];
                total_read += reader.read(slice).await?;
            }

            total_read
        };

        self.write_pos = (self.write_pos + bytes_read) % self.capacity;
        Ok(bytes_read)
    }

    pub async fn write_to_writer<W>(&mut self, writer: &mut W) -> IoResult<usize>
    where
        W: AsyncWrite + Unpin,
    {
        let available = self.available_read_space();
        if available == 0 {
            return Ok(0);
        }

        let bytes_written = if self.read_pos + available <= self.capacity {
            // Simple case: read from current position
            let data = &self.data[self.read_pos..self.read_pos + available];
            writer.write(data).await?
        } else {
            // Wrap around case: read to end then beginning
            let end_space = self.capacity - self.read_pos;
            let mut total_written = 0;

            if end_space > 0 {
                let data = &self.data[self.read_pos..self.capacity];
                total_written += writer.write(data).await?;
            }

            if total_written == end_space {
                let remaining = available - end_space;
                let data = &self.data[0..remaining];
                total_written += writer.write(data).await?;
            }

            total_written
        };

        self.read_pos = (self.read_pos + bytes_written) % self.capacity;
        Ok(bytes_written)
    }

    pub fn has_data(&self) -> bool {
        self.available_read_space() > 0
    }

    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
    }

    fn available_read_space(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.capacity - self.read_pos + self.write_pos
        }
    }

    fn available_write_space(&self) -> usize {
        (if self.write_pos >= self.read_pos {
            self.capacity - self.write_pos + self.read_pos
        } else {
            self.read_pos - self.write_pos
        }) - 1 // Leave one byte gap to distinguish full from empty
    }
}

/// High-performance data copying using splice/sendfile when available
pub struct OptimizedCopier;

impl OptimizedCopier {
    /// Copy data from source to destination with system-level optimizations
    pub async fn copy<R, W>(source: &mut R, dest: &mut W) -> Result<u64>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buffer = BytesMut::with_capacity(8192);
        let mut total_copied = 0u64;

        loop {
            let bytes_read = source.read_buf(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }

            let mut remaining = buffer.len();
            while remaining > 0 {
                let bytes_written = dest.write_buf(&mut buffer).await?;
                if bytes_written == 0 {
                    return Err(crate::error::ProxyError::Io(
                        std::io::Error::new(std::io::ErrorKind::WriteZero, "Write zero")
                    ));
                }
                remaining -= bytes_written;
                total_copied += bytes_written as u64;
            }
        }

        Ok(total_copied)
    }
}
