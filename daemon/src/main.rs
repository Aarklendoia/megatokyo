mod config;
mod control;
mod poll;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use megatokyo_core::image_cache::ImageCache;
use megatokyo_core::store::Store;

use config::Config;
use control::AppState;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config_path = Config::default_path();
    let config = match Config::load_or_init(&config_path) {
        Ok(config) => config,
        Err(err) => {
            log::error!("could not load config at {}: {err}", config_path.display());
            std::process::exit(1);
        }
    };
    log::info!("config loaded from {}", config_path.display());

    let store = match Store::open(&Store::default_db_path()) {
        Ok(store) => store,
        Err(err) => {
            log::error!("could not open the database: {err}");
            std::process::exit(1);
        }
    };

    let image_cache = ImageCache::new(ImageCache::default_cache_dir());

    let bind: std::net::SocketAddr = match config.bind.parse() {
        Ok(addr) => addr,
        Err(err) => {
            log::error!("invalid bind address {:?}: {err}", config.bind);
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState {
        store,
        image_cache,
        token: config.api_token.clone(),
        config: tokio::sync::RwLock::new(config),
        config_path,
        backfilling: AtomicBool::new(false),
        check_requested: tokio::sync::Notify::new(),
    });

    let client = reqwest::Client::new();
    let poll_state = state.clone();
    let poll_client = client.clone();
    tokio::spawn(async move {
        poll::run_loop(poll_client, poll_state).await;
    });

    if let Err(err) = control::serve(bind, state).await {
        log::error!("HTTP server error: {err}");
        std::process::exit(1);
    }
}
