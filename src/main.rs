use axum::{
    extract::{Path, State},
    response,
    routing::get,
    Router,
};
use sled::Db;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};
use tokio::net::TcpListener;
static BODY: &'static str = include_str!("../hello.html");
static _404: &'static str = include_str!("../404.html");
struct AppState {
    db: Db,
}

async fn handle_db_read(
    Path(id): Path<String>,
    State(appstate): State<Arc<AppState>>,
) -> response::Html<String> {
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

    // let key = id.parse::<u64>().unwrap().to_be_bytes();
    // let value = appstate.db.get(key).unwrap().unwrap();
    response::Html(format!("Value: {}", value))
}

#[tokio::main]
async fn main() {
    let appstate = AppState {
        db: sled::open("my_db").unwrap(),
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
