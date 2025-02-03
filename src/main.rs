use axum::{
    extract::{Path, State},
    response,
    routing::get,
    Router,
};
use fred::{clients::Pool, prelude::*};

use sled::Db;
use std::{
    env,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};
use tokio::net::TcpListener;
static BODY: &'static str = include_str!("../hello.html");
static _404: &'static str = include_str!("../404.html");
struct AppState {
    db: Db,
    pool: Option<Pool>,
}

async fn handle_db_read(
    Path(id): Path<String>,
    State(appstate): State<Arc<AppState>>,
) -> response::Html<String> {
    let val = if let Some(pool) = &appstate.pool {
        pool.get::<String, &str>(&id).await
    } else {
        Err(fred::error::Error::new(
            fred::error::ErrorKind::NotFound,
            "Not Found".to_string(),
        ))
    };
    let val = match val {
        Ok(v) => v,
        Err(_) => {
            let key = id.parse::<u64>().unwrap();
            let value = appstate
                .db
                .get(key.to_be_bytes())
                .unwrap()
                .unwrap_or_default();
            let value = value
                .to_vec()
                .into_iter()
                .map(|b| b as char)
                .collect::<String>();
            if let Some(pool) = &appstate.pool {
                pool.set::<(), u64, &str>(key, &value, None, None, false)
                    .await
                    .unwrap();
            }
            value
        }
    };
    // let key = id.parse::<u64>().unwrap().to_be_bytes();
    // let value = appstate.db.get(key).unwrap().unwrap();
    response::Html(format!("Value: {}", val))
}

#[tokio::main]
async fn main() {
    let pool_size = env::var("REDIS_POOL_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8);
    let redis_url = "redis://127.0.0.1:6379";
    let config = Config::from_url(&redis_url).unwrap();
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size);
    let pool = match pool {
        Ok(pool) => match pool.init().await {
            Ok(_) => Some(pool),
            Err(_) => None,
        },
        Err(_) => None,
    };
    let appstate = AppState {
        db: sled::open("my_db").unwrap(),
        pool,
    };
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { response::Html(BODY) }))
        .route(
            "/db",
            get(|appstate: State<Arc<AppState>>| async move {
                let db = &appstate.db;
                let id = db.generate_id().unwrap();
                db.insert(id.to_be_bytes(), b"Hello, World!").unwrap();
                response::Html(format!("Inserted: {}", id))
            }),
        )
        .route("/db/{id}", get(handle_db_read))
        .route("/404", get(|| async { response::Html(_404) }))
        .with_state(Arc::new(appstate));
    let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 8080))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
