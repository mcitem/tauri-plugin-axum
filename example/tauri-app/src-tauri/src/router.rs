use std::{convert::Infallible, time::Duration};

use axum::{response::Response, routing, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

#[derive(Serialize, Deserialize)]
struct Greet {
    axum: String,
    tauri: String,
}

async fn post_handle(Json(j): Json<Greet>) -> Json<Greet> {
    Json(Greet {
        axum: format!("axum, {}!", j.axum),
        tauri: format!("tauri, {}!", j.tauri),
    })
}

async fn stream_body() -> Response {
    let s = futures::stream::unfold(0u64, |mut counter| async move {
        match counter {
            999 => None,
            _ => {
                counter += 1;

                sleep(Duration::from_millis(500)).await;
                println!("Sending chunk: {}", counter);
                Some((
                    Ok::<_, Infallible>(axum::body::Bytes::from(counter.to_string())),
                    counter,
                ))
            }
        }
    });
    Response::new(axum::body::Body::from_stream(s))
}

pub fn router() -> Router {
    Router::new()
        .route("/", routing::get(|| async { "Hello, World!" }))
        .route("/post", routing::post(post_handle))
        .route("/stream-body", routing::get(stream_body))
}
