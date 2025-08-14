use axum::{routing, Json, Router};
use serde::{Deserialize, Serialize};
use tauri::{ipc, Runtime};

#[tauri::command]
async fn custom_usage<R: Runtime>(
    app: tauri::AppHandle<R>,
    req: ipc::Request<'_>,
) -> Result<tauri_plugin_axum::AxumResponse, tauri_plugin_axum::Error> {
    use tauri_plugin_axum::AxumExt;
    app.axum().call(req).await
}

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_axum::init(
            Router::new()
                .route("/", routing::get(|| async { "Hello, World!" }))
                .route("/post", routing::post(post_handle)),
        ))
        .invoke_handler(tauri::generate_handler![custom_usage])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
