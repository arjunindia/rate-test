use axum::{
    extract::{Path, State},
    response,
    routing::get,
    Router,
};
use sled::Db;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{atomic::AtomicU32, Arc},
};
use surrealkv::{Options, Store};
use tokio::net::TcpListener;
static BODY: &'static str = include_str!("../hello.html");
static _404: &'static str = include_str!("../404.html");
pub type SharedState = Arc<RwLock<AppState>>;

struct AppState {
    db: Db,
    store: Store,
    id: u32,
}

async fn handle_db_read(
    Path(id): Path<String>,
    State(appstate): State<Arc<AppState>>,
) -> response::Html<String> {
    let key = id.parse::<i32>().unwrap().to_be_bytes();
    let mut value: String;
    appstate
        .store
        .view(|txn| {
            let id = txn
                .get(&key)
                .unwrap()
                .map(|v| String::from_utf8(v.to_vec()).unwrap());
            value = id.unwrap();
            Ok(())
        })
        .unwrap();
    response::Html(format!("Value: {}", value))
}

#[tokio::main]
async fn main() {
    // Create a new store
    let mut opts = Options::new();
    opts.dir = "./data".into();
    let store = Store::new(opts).expect("failed to create store");
    let mut id: u32 = 0;
    store
        .view(|txn| {
            let counter = txn.get(b"counter").unwrap();
            if let Some(counter) = counter {
                id = u32::from_be_bytes(counter.to_vec().try_into().unwrap());
            }
            Ok(())
        })
        .unwrap();

    let state = SharedState::default();
    let appstate = AppState {
        db: sled::open("my_db").unwrap(),
        store,
        id,
    };

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { response::Html(BODY) }))
        .route(
            "/db",
            get(|mut appstate: State<Arc<AppState>>| async move {
                let writable = state.write().unwrap();
                let store = &appstate.store;
                let mut txn = store.begin_with_mode(surrealkv::Mode::WriteOnly).unwrap();
                appstate.id += 1;
                let key = appstate.id.to_be_bytes();
                txn.set(&key, b"Hello, World!").unwrap();
                txn.set(b"counter", &key).unwrap();
                txn.commit().await.unwrap();
                response::Html(format!("Inserted: {}", appstate.id))
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
