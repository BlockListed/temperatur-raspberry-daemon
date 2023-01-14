use once_cell::sync::Lazy;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
mod retry;

mod data;
mod ping;
mod web;

static CONFIG: Lazy<config::ConfigManager> = Lazy::new(|| {
    config::ConfigManager::new(
        std::env::var("CONFIG_FILE").unwrap_or_else(|_| "/etc/conf.d/msh_daemon.toml".to_string()),
    )
    .unwrap()
});

#[tokio::main]
async fn main() {
    let filter_layer = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::fmt()
        .with_env_filter(filter_layer)
        .finish()
        .try_init()
        .unwrap();

    let mut handles = Vec::new();
    handles.push(tokio::spawn(web::web()));
    handles.push(tokio::spawn(data::data()));
    handles.push(tokio::spawn(ping::ping()));

    for h in handles {
        h.await.unwrap();
    }
}
