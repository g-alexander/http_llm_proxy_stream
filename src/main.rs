use actix_web::{web, App, HttpServer};
use clap::Parser;
use http_llm_proxy_stream::{proxy_handler, AppState};
use log::{error, info};
use reqwest::{Client, Url};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Порт, на котором будет слушать сервис
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// URL, куда проксировать запросы
    #[arg(short, long)]
    target_url: String,

    /// Дополнительный HTTP заголовок в формате "Header-Name: Header-Value"
    #[arg(short, long)]
    add_header: Option<String>,

    /// Наименование файла слогами.
    #[arg(short, long, default_value = "log.csv")]
    log_file: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Инициализируем логгер с явной настройкой вывода в консоль
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .init();

    let args = Args::parse();

    info!("Запуск HTTP прокси-сервиса");
    info!("Порт: {}", args.port);
    info!("Target URL: {}", args.target_url);
    info!("Файл с логом: {}", args.log_file);

    // Валидация target URL
    if let Err(e) = Url::parse(&args.target_url) {
        error!("Неверный формат target URL: {}", e);
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
        .expect("Не удалось создать HTTP клиент");

    let app_state = web::Data::new(AppState {
        target_url: args.target_url,
        client,
        additional_header: header,
        log_file: args.log_file,
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .default_service(web::route().to(proxy_handler))
    })
        .bind(("0.0.0.0", args.port))?;

    server.run().await
}
