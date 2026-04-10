#![doc = include_str!("../README.md")]

use axum::{body::Body, extract::Request, http::Response, Router};
use futures_util::FutureExt;
use http_body_util::BodyExt;
use std::{collections::HashMap, future::Future, marker::PhantomData, ops::Deref};
use tauri::{
    async_runtime::block_on,
    ipc::{InvokeBody, Request as IpcRequest},
    plugin::TauriPlugin,
    Manager, Runtime,
};
use tower::{Service, ServiceExt};

#[cfg(feature = "tokio-rwlock")]
use std::sync::Arc;
#[cfg(feature = "tokio-rwlock")]
use tokio::sync::RwLock;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};
pub use models::*;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the axum APIs.
pub trait AxumExt<R: Runtime> {
    fn axum(&self) -> &Axum;
    #[cfg(feature = "tokio-rwlock")]
    fn set_router(&self, router: Router) -> impl std::future::Future<Output = ()>;
}

impl<R: Runtime, T: Manager<R>> crate::AxumExt<R> for T {
    fn axum(&self) -> &Axum {
        self.state::<Axum>().inner()
    }

    #[cfg(feature = "tokio-rwlock")]
    fn set_router(&self, router: Router) -> impl std::future::Future<Output = ()> {
        async move {
            let axum = self.state::<Axum>();
            *axum.write().await = router;
        }
    }
}

#[cfg(feature = "tokio-rwlock")]
pub struct Axum(pub Arc<RwLock<Router>>);

#[cfg(not(feature = "tokio-rwlock"))]
pub struct Axum(pub Router);

impl Axum {
    pub async fn inner(&self) -> Router {
        #[cfg(feature = "tokio-rwlock")]
        {
            self.read().await.clone()
        }

        #[cfg(not(feature = "tokio-rwlock"))]
        {
            self.0.clone()
        }
    }
}

impl Deref for Axum {
    #[cfg(feature = "tokio-rwlock")]
    type Target = Arc<RwLock<Router>>;
    #[cfg(not(feature = "tokio-rwlock"))]
    type Target = Router;

    fn deref(&self) -> &Self::Target {
        &self.0
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

        let response = self.inner().await.call(result).await.unwrap();

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
            .inner()
            .await
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
        let router = self.router;

        #[cfg(feature = "catch-panic")]
        let router = router.layer(tower_http::catch_panic::CatchPanicLayer::new());

        #[cfg(feature = "cors")]
        let router = router.layer(tower_http::cors::CorsLayer::permissive());

        #[cfg(feature = "tokio-rwlock")]
        let router = Arc::new(RwLock::new(router));

        let svc = router.clone();

        tauri::plugin::Builder::new("axum")
            .register_asynchronous_uri_scheme_protocol("axum", move |_ctx, request, responder| {
                let svc = svc.clone();
                tauri::async_runtime::spawn(async move {
                    #[cfg(feature = "tokio-rwlock")]
                    let svc = svc.read().await.clone();
                    #[cfg(not(feature = "tokio-rwlock"))]
                    let svc = svc;

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
                commands::fetch_read_body,
                commands::fetch_cancel_body
            ])
            .setup(|app, __api| {
                app.manage(Axum(router));
                Ok(())
            })
            .build()
    }
}

impl<R: Runtime> Builder<R> {}
