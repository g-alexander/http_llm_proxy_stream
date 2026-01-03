mod handler;

pub use handler::*;

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use bytes::{Bytes, BytesMut};
use futures_util::Stream;
use log::{error, info};
use reqwest::{Client, Error};
use std::fs::OpenOptions;
use std::io::Write;

pub struct AppState {
    pub target_url: String,
    pub client: Client,
    pub additional_header: Option<(String, String)>,
    pub log_writer: Arc<Mutex<LogWriter>>,
    pub request_timeout_minutes: u64,
}

pub struct LogWriter {
    file_path: String,
}

impl LogWriter {
    pub fn new(file_path: String) -> Self {
        LogWriter { file_path }
    }

    pub fn write_log(&self, method: &str, url: &str, request_body: &str, response_body: &str) {
        // Escape quotes in data for CSV format
        let request_body_escaped = request_body.replace('"', "\"\"");
        let response_body_escaped = response_body.replace('"', "\"\"");

        let csv_line = format!(
            "{},{},\"{}\",\"{}\"\n",
            method, url, request_body_escaped, response_body_escaped
        );

        match OpenOptions::new().create(true).append(true).open(&self.file_path) {
            Ok(mut file) => {
                match file.write_all(csv_line.as_bytes()) {
                    Ok(_) => {
                        match file.flush() {
                            Ok(_) => info!("Log write successful, wrote {} characters.", csv_line.len()),
                            Err(e) => error!("Error finalizing file write: {}", e)
                        }
                    },
                    Err(e) => error!("Error writing to file: {}", e)
                }
            },
            Err(e) => error!("Error opening file: {}", e)
        }
    }
}

pub trait OnIterationComplete {
    fn on_iteration_complete(&self,  data: Vec<u8>);
}
pub struct CompletableStream {
    inner_stream: Box<dyn Stream<Item=Result<Bytes, Error>> + Unpin>,
    buffer: BytesMut,
    handler: Box<dyn OnIterationComplete>
}

impl CompletableStream {
    pub fn new(inner_stream: Box<dyn Stream<Item=Result<Bytes, Error>> + Unpin>, handler: Box<dyn OnIterationComplete>) -> CompletableStream {
        CompletableStream {inner_stream, buffer: BytesMut::new(), handler}
    }
}

impl Stream for CompletableStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let result = Pin::new(&mut this.inner_stream).poll_next(cx);

        match &result {
            Poll::Ready(v) => {
                match v {
                    Some(byte_res) => {
                        match byte_res {
                            Ok(bytes) => {
                                // this.handler.on_iteration_complete(bytes.to_vec());
                                this.buffer.extend_from_slice(&bytes[..]);
                            },
                            Err(e) => error!("Error receiving request data: {}", e)
                        }
                        // this.handler.on_iteration_complete(bytes.to_vec());
                    },
                    None => {
                        this.handler.on_iteration_complete(this.buffer.to_vec());
                        info!("Received Ready - None from stream");
                    }
                }
            },
            Poll::Pending => {
                info!("Received Pending from stream");
            }
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner_stream.size_hint()
    }
}
