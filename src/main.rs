use axum::{
    extract::{Path, State},
    response,
    routing::get,
    Router,
};
use compio::net::TcpListener;
use sled::Db;
use std::sync::Arc;
static BODY: &'static str = include_str!("../hello.html");
static _404: &'static str = include_str!("../404.html");
struct AppState {
    db: Db,
}

async fn handle_db_read(
    Path(id): Path<String>,
    State(appstate): State<Arc<AppState>>,
) -> response::Html<String> {
    let key = id.parse::<u64>().unwrap().to_be_bytes();
    let value = appstate.db.get(key).unwrap().unwrap();
    response::Html(format!("Value: {}", String::from_utf8_lossy(&value)))
}

#[compio::main]
async fn main() {
    let appstate = Arc::new(AppState {
        db: sled::open("my_db").unwrap(),
    });
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { response::Html(BODY) }))
        .route(
            "/db",
            get(|appstate: State<Arc<AppState>>| async move {
                let db = &appstate.clone().db;
                let id = db.generate_id().unwrap();
                db.insert(&id.to_be_bytes(), &id.to_be_bytes()).unwrap();
                response::Html(format!("Inserted: {}", id))
            }),
        )
        .route("/db/{id}", get(handle_db_read))
        .route("/404", get(|| async { response::Html(_404) }))
        .with_state(appstate);
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    cyper_axum::serve(listener, app).await.unwrap();
}
