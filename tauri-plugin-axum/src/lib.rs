#![doc = include_str!("../README.md")]

use axum::body::Body;
use axum::extract::Request;
use axum::Router;
use http_body_util::BodyExt;
use std::collections::HashMap;
use tauri::ipc::{InvokeBody, Request as IpcRequest};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};
use tower::Service;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};
pub use models::*;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the axum APIs.
pub trait AxumExt<R: Runtime> {
    fn axum(&self) -> &Axum;
}

impl<R: Runtime, T: Manager<R>> crate::AxumExt<R> for T {
    fn axum(&self) -> &Axum {
        self.state::<Axum>().inner()
    }
}

pub struct Axum(Router);

impl Axum {
    pub async fn call(&self, req: IpcRequest<'_>) -> Result<AxumResponse> {
        let mut rr = Request::builder();
        let _ = std::mem::replace(rr.headers_mut().unwrap(), req.headers().clone());

        rr = rr.uri(req.headers().get("x-uri").ok_or(Error::Uri)?.as_ref());
        rr = rr.method(req.headers().get("x-method").ok_or(Error::Method)?.as_ref());

        let bytes = match req.body() {
            InvokeBody::Json(v) => serde_json::to_vec(&v)?,
            InvokeBody::Raw(r) => r.to_vec(),
        };

        let result = rr.body(Body::from(bytes))?;
        let response = self.0.clone().call(result).await.unwrap();
        let status = response.status();
        let mut headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), String::from(v.to_str().unwrap_or_default())))
            .collect();
        let colleted = response.into_body().collect().await?;

        if let Some(t) = colleted.trailers() {
            for (k, v) in t.iter() {
                headers.insert(k.to_string(), String::from(v.to_str().unwrap_or_default()));
            }
        }

        Ok(AxumResponse {
            status,
            headers,
            body: colleted.to_bytes(),
        })
    }
}

/// Initializes the plugin.
/// ```rust,no_run
/// tauri::Builder::default()
///     .plugin(tauri_plugin_axum::init(
///         Router::new()
///             .route("/", routing::get(|| async { "Hello, World!" }))
///             .route("/post", routing::post(post_handle))
///     ))
/// ```
pub fn init<R: Runtime>(router: Router) -> TauriPlugin<R> {
    Builder::new("axum")
        .invoke_handler(tauri::generate_handler![commands::call])
        .setup(|app, __api| {
            app.manage(Axum(router));
            Ok(())
        })
        .build()
}
