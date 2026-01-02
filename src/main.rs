use actix_web::{web, App, HttpServer};
use clap::Parser;
use http_llm_proxy_stream::{proxy_handler, AppState, LogWriter};
use log::{error, info};
use reqwest::{Client, Url};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port on which the service will listen
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// URL to proxy requests to
    #[arg(short, long)]
    target_url: String,

    /// Additional HTTP header in format "Header-Name: Header-Value"
    #[arg(short, long)]
    add_header: Option<String>,

    /// Name of the log file
    #[arg(short, long, default_value = "log.csv")]
    log_file: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger with explicit console output configuration
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .init();

    let args = Args::parse();

    info!("Starting HTTP proxy service");
    info!("Port: {}", args.port);
    info!("Target URL: {}", args.target_url);
    info!("Log file: {}", args.log_file);

    // Validate target URL
    if let Err(e) = Url::parse(&args.target_url) {
        error!("Invalid target URL format: {}", e);
        std::process::exit(1);
    }

    let header = match args.add_header {
        Some(h) => {
            let v = h.split(":").collect::<Vec<&str>>();
            if v.len() != 2 {
                None
            } else {
                Some((v[0].to_string(), v[1].to_string()))
            }
        },
        None => None
    };

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let log_writer = LogWriter::new(args.log_file.clone());
    
    let app_state = web::Data::new(AppState {
        target_url: args.target_url,
        client,
        additional_header: header,
        log_writer: Arc::new(std::sync::Mutex::new(log_writer)),
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .default_service(web::route().to(proxy_handler))
    })
        .bind(("0.0.0.0", args.port))?;

    server.run().await
}
