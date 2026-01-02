use crate::{log_post_request_to_csv, AppState, CompletableStream, OnIterationComplete};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use flate2::read::GzDecoder;
use log::{error, info, trace, warn};
use reqwest::Url;
use std::io::Read;

fn format_target_url(base_url: &str, req: &HttpRequest) -> Result<String, Box<dyn std::error::Error>> {
    let mut url = Url::parse(base_url)?;

    // Add path and query parameters from the original request
    url.set_path(req.path());
    url.set_query(req.uri().query());

    Ok(url.to_string())
}

struct CompleteHandler {
    gzip: bool,
    method: String,
    query: String,
    request_data: String,
    file_name: String,
}

impl CompleteHandler {
    fn new(gzip: bool, method: String, query: String, request_data: String, file_name: String) -> CompleteHandler {
        CompleteHandler {gzip, method, query, request_data, file_name}
    }
}

impl OnIterationComplete for CompleteHandler {
    fn on_iteration_complete(&self, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        info!("Stream completion handler called. Data size: {}", data.len());
        let response_data ;
        if self.gzip {
            let mut decoded = Vec::new();
            GzDecoder::new(&data[..]).read_to_end(&mut decoded).unwrap();
            response_data = String::from_utf8_lossy(&decoded).to_string();
        } else {
            response_data = String::from_utf8_lossy(&data[..]).to_string();
        }
        log_post_request_to_csv(&self.method, &self.query, &self.request_data, &response_data,
                                &self.file_name);
    }
}

pub async fn proxy_handler(req: HttpRequest, body: web::Bytes, data: web::Data<AppState>) -> impl Responder {
    info!("Received {} request on {}", req.method(), req.path());

    let target_url = match format_target_url(&data.target_url, &req) {
        Ok(url) => url,
        Err(e) => {
            error!("URL formatting error: {}", e);
            return HttpResponse::InternalServerError().json("URL formatting error");
        }
    };
    let method = req.method().as_str().to_string();
    // Create HTTP request to target server
    let mut request_builder = match method.as_str() {
        "GET" => data.client.get(&target_url),
        "POST" => data.client.post(&target_url),
        "PUT" => data.client.put(&target_url),
        "DELETE" => data.client.delete(&target_url),
        "PATCH" => data.client.patch(&target_url),
        method => {
            warn!("Unsupported HTTP method: {}", method);
            return HttpResponse::BadRequest().json(format!("Unsupported method: {}", method));
        }
    };

    // Copy headers from incoming request
    for (header_name, header_value) in req.headers() {
        // Skip some headers that may cause problems
        if header_name.as_str().to_lowercase() == "host" {
            continue;
        }
        if let Some(h) = &data.additional_header {
            if header_name.as_str().to_lowercase() == h.0.to_lowercase() {
                continue;
            }
        }
        if let Ok(header_str) = header_value.to_str() {
            // println!("{}: {}", &header_name.as_str(), &header_str);
            request_builder = request_builder.header(header_name.as_str(), header_str);
        }
    }
    if let Some(h) = &data.additional_header {
        request_builder = request_builder.header(h.0.as_str(), h.1.as_str());
    }
    // Add request body for POST, PUT, PATCH
    if matches!(req.method().as_str(), "POST" | "PUT" | "PATCH") && !body.is_empty() {
        request_builder = request_builder.body(body.to_vec());
    }

    let url = req.uri().to_string();
    let request_body = String::from_utf8_lossy(body.to_vec().as_slice()).to_string();

    match request_builder.send().await {
        Ok(response) => {
            let mut gzip = false;
            let headers = response.headers().clone();
            headers.into_iter().for_each(|(k, v)| {
                let key = k.unwrap().as_str().to_owned();
                let val = v.to_str().unwrap().to_owned();
                if key.to_lowercase() == "content-encoding" && val.to_lowercase() == "gzip" {
                    gzip = true;
                }
                trace!("Header: {}: {}", key, val);
            });
            let mut client_response = HttpResponse::build(StatusCode::from_u16(response.status().as_u16()).unwrap());
            for (header_name, header_value) in response.headers() {
                client_response.insert_header((header_name.as_str(), header_value.to_str().unwrap()));
            }
            let stream = CompletableStream::new(Box::from(response.bytes_stream()),
                                                Box::new(CompleteHandler::new(gzip, method, url, request_body,
                                                                              data.log_file.clone())));
            let res = client_response.streaming(Box::pin(stream));
            info!("Request stream passed for processing");
            res
        },
        Err(e) => {
                error!("Request execution error: {}", e);
                HttpResponse::BadGateway().json("Request execution error to target server")
            }
        }
}
