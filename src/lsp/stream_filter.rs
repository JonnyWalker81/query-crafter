use std::pin::Pin;
use std::task::{Context, Poll};
use futures::io::{AsyncRead, AsyncBufRead, AsyncBufReadExt, BufReader};
use futures::stream::{Stream, StreamExt};
use pin_project::pin_project;
use bytes::{Bytes, BytesMut};
use std::io;

/// A wrapper that filters out debug messages from an LSP server's stdout
#[pin_project]
pub struct FilteredLspStream<R> {
    #[pin]
    reader: BufReader<R>,
    buffer: BytesMut,
    in_header: bool,
    content_length: Option<usize>,
    content_read: usize,
}

impl<R: AsyncRead + Unpin> FilteredLspStream<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: BytesMut::with_capacity(8192),
            in_header: true,
            content_length: None,
            content_read: 0,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for FilteredLspStream<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut this = self.as_mut().project();
        
        loop {
            // If we have buffered data, return it
            if !this.buffer.is_empty() {
                let len = std::cmp::min(buf.len(), this.buffer.len());
                buf[..len].copy_from_slice(&this.buffer[..len]);
                this.buffer.advance(len);
                return Poll::Ready(Ok(len));
            }
            
            // Read more data
            let mut line = String::new();
            match futures::ready!(this.reader.as_mut().poll_read_line(cx, &mut line)) {
                Ok(0) => return Poll::Ready(Ok(0)), // EOF
                Ok(_) => {
                    // Filter out debug lines that start with timing info
                    if line.contains("complete:") && line.contains("ms") {
                        // Skip this line, it's debug output
                        continue;
                    }
                    
                    // Process LSP protocol
                    if *this.in_header {
                        if line.trim().is_empty() {
                            // End of headers
                            *this.in_header = false;
                            this.buffer.extend_from_slice(b"\r\n");
                        } else if line.starts_with("Content-Length:") {
                            // Parse content length
                            if let Some(len_str) = line.trim().strip_prefix("Content-Length:") {
                                *this.content_length = len_str.trim().parse().ok();
                            }
                            this.buffer.extend_from_slice(line.as_bytes());
                        } else {
                            // Other headers
                            this.buffer.extend_from_slice(line.as_bytes());
                        }
                    } else {
                        // In content - pass through
                        this.buffer.extend_from_slice(line.as_bytes());
                        
                        // Check if we've read a complete message
                        if let Some(expected_len) = *this.content_length {
                            *this.content_read += line.len();
                            if *this.content_read >= expected_len {
                                // Reset for next message
                                *this.in_header = true;
                                *this.content_length = None;
                                *this.content_read = 0;
                            }
                        }
                    }
                }
                Err(e) => return Poll::Ready(Err(e)),
            }
        }
    }
}