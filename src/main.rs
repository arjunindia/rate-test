use axum::{response, routing::get, Router};
use std::{thread::sleep, time};
use tokio::net::TcpListener;
static BODY: &'static str = include_str!("../hello.html");
static _404: &'static str = include_str!("../404.html");

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { response::Html(BODY) }))
        .route(
            "/sleep",
            get(|| async {
                sleep(time::Duration::from_secs(5));
                response::Html(BODY)
            }),
        )
        .route("/404", get(|| async { response::Html(_404) }));
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
