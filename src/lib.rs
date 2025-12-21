mod handler;

pub use handler::*;

use std::pin::Pin;
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
                            Err(e) => error!("Ошибка получения данных запроса {}", e)
                        }
                        // this.handler.on_iteration_complete(bytes.to_vec());
                    },
                    None => {
                        this.handler.on_iteration_complete(this.buffer.to_vec());
                        info!("Получено Ready - None из потока");
                    }
                }
            },
            Poll::Pending => {
                info!("Получено Pending из потока");
            }
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner_stream.size_hint()
    }
}

fn log_post_request_to_csv(method: &str, url: &str, request_body: &str, response_body: &str) {
    // Экранируем кавычки в данных для CSV формата
    let request_body_escaped = request_body.replace('"', "\"\"");
    let response_body_escaped = response_body.replace('"', "\"\"");

    let csv_line = format!(
        "{},{},\"{}\",\"{}\"\n",
        method, url, request_body_escaped, response_body_escaped
    );

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("log.csv").unwrap();

    file.write_all(csv_line.as_bytes()).unwrap();
}