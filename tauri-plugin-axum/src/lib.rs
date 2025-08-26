#![doc = include_str!("../README.md")]

use axum::body::Body;
use axum::extract::Request;
use axum::http::Response;
use axum::Router;
use futures_util::FutureExt;
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use tauri::async_runtime::block_on;
use tauri::ipc::{InvokeBody, Request as IpcRequest};
use tauri::{plugin::TauriPlugin, Manager, Runtime};
use tower::{Service, ServiceExt};

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

pub struct Axum(pub Router);

impl Deref for Axum {
    type Target = Router;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Axum {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Axum {
    pub async fn call(&self, req: IpcRequest<'_>) -> Result<AxumResponse> {
        let mut rr = Request::builder();
        *rr.headers_mut().unwrap() = req.headers().clone();

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

    pub(crate) async fn call_json(&self, req: IpcRequest<'_>) -> Result<Vec<u8>> {
        let body = match req.body() {
            InvokeBody::Raw(raw) => raw.to_vec(),
            InvokeBody::Json(_) => {
                return Err(Error::Canceled);
            }
        };

        let mut rr = Request::builder().header("Content-Type", "application/json");

        rr = rr.uri(req.headers().get("x-uri").ok_or(Error::Uri)?.as_ref());
        rr = rr.method(req.headers().get("x-method").ok_or(Error::Method)?.as_ref());

        let res = self
            .0
            .clone()
            .call(rr.body(Body::from(body))?)
            .await
            .unwrap();

        let body = res.into_body().collect().await?.to_bytes().to_vec();

        Ok(body)
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
    Builder::new(router).build()
}

pub fn block_init<R: Runtime, F: Future<Output = Router>>(f: F) -> TauriPlugin<R> {
    block_on(f.map(init))
}

pub fn try_block_init<
    R: Runtime,
    F: Future<Output = std::result::Result<Router, Box<dyn std::error::Error>>>,
>(
    f: F,
) -> std::result::Result<TauriPlugin<R>, Box<dyn std::error::Error>> {
    Ok(block_on(f).map(init)?)
}

pub struct Builder<R: Runtime> {
    router: Router,
    _r: PhantomData<R>,
}

impl<R: Runtime> Builder<R> {
    pub fn new(router: Router) -> Self {
        Self {
            router,
            _r: PhantomData,
        }
    }

    pub fn build(self) -> TauriPlugin<R> {
        let mut router_clone = self.router.clone();

        #[cfg(feature = "catch-panic")]
        {
            router_clone = router_clone.layer(tower_http::catch_panic::CatchPanicLayer::new());
        }

        #[cfg(feature = "cors")]
        {
            router_clone = router_clone.layer(tower_http::cors::CorsLayer::permissive());
        }

        tauri::plugin::Builder::new("axum")
            .register_asynchronous_uri_scheme_protocol("axum", move |_ctx, request, responder| {
                let svc = router_clone.clone();
                tauri::async_runtime::spawn(async move {
                    let (mut parts, body) = svc
                        .oneshot(request.map(Body::from))
                        .await
                        .unwrap()
                        .into_parts();

                    let body = match body.collect().await {
                        Ok(b) => b.to_bytes().to_vec(),
                        Err(e) => {
                            parts.status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                            e.to_string().into_bytes()
                        }
                    };
                    responder.respond(Response::from_parts(parts, body));
                });
            })
            .invoke_handler(tauri::generate_handler![
                commands::call,
                commands::call_json,
                commands::fetch,
                commands::fetch_cancel,
                commands::fetch_send,
                commands::fetch_read_body
            ])
            .setup(|app, __api| {
                app.manage(Axum(self.router));
                Ok(())
            })
            .build()
    }
}

impl<R: Runtime> Builder<R> {}
