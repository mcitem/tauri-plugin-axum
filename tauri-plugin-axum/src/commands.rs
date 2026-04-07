use axum::{
    body::{Body, BodyDataStream},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Uri},
    response::Response,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};
use tauri::ipc::Request as IpcRequest;
use tauri::{command, AppHandle, Manager, ResourceId, Runtime, Webview};
use tokio::sync::{
    oneshot::{channel, Receiver, Sender},
    Mutex,
};
use tower::ServiceExt;

use crate::Result;
use crate::{AxumExt, AxumResponse};

#[command]
pub(crate) async fn call<R: Runtime>(
    app: AppHandle<R>,
    req: IpcRequest<'_>,
) -> Result<AxumResponse> {
    app.axum().call(req).await
}

#[command]
pub(crate) async fn call_json<R: Runtime>(
    app: AppHandle<R>,
    req: IpcRequest<'_>,
) -> Result<Vec<u8>> {
    app.axum().call_json(req).await
}

struct FetchRequest(Mutex<Pin<Box<dyn Future<Output = Result<Response>> + Send>>>);
struct AbortSender(Sender<()>);
struct AbortReceiver(Receiver<()>);
struct FetchBody(Mutex<BodyDataStream>);

impl tauri::Resource for FetchRequest {}
impl tauri::Resource for AbortSender {}
impl tauri::Resource for AbortReceiver {}
impl tauri::Resource for FetchBody {}

#[derive(Deserialize)]
pub struct FetchConf {
    uri: String,
    method: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

#[derive(Serialize)]
pub struct FetchReturn {
    rid: ResourceId,
    txid: ResourceId,
    rxid: ResourceId,
}

#[command]
pub(crate) async fn fetch<R: Runtime>(app: Webview<R>, conf: FetchConf) -> Result<FetchReturn> {
    let mut headers = HeaderMap::new();
    for (k, v) in conf.headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) {
            headers.append(name, value);
        } else {
            if cfg!(debug_assertions) {
                eprintln!("Invalid header: {}: {}", k, v);
            }
        }
    }

    let method = Method::from_bytes(conf.method.as_bytes()).unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            eprintln!("Invalid method: {}, defaulting to GET", conf.method);
        }
        Method::GET
    });

    let uri: Uri = conf.uri.parse().unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            eprintln!("Invalid URI: {}, defaulting to /", conf.uri);
        };
        Uri::from_static("/")
    });

    let body = match conf.body {
        Some(b) => Body::from(b),
        None => Body::empty(),
    };

    let mut req_builder = Request::builder().method(method).uri(uri);

    *req_builder.headers_mut().unwrap() = headers;

    let request = req_builder.body(body)?;

    let svc = app.axum().0.clone();

    let fut = async move {
        let resp = svc
            .oneshot(request)
            .await
            .expect("axum Router service should be infallible");
        Ok::<_, crate::Error>(resp)
    };

    let mut table = app.resources_table();
    let (tx, rx) = channel::<()>();
    Ok(FetchReturn {
        rid: table.add(FetchRequest(Mutex::new(Box::pin(fut)))),
        txid: table.add(AbortSender(tx)),
        rxid: table.add(AbortReceiver(rx)),
    })
}

#[command]
pub(crate) fn fetch_cancel<R: Runtime>(app: AppHandle<R>, txid: ResourceId) -> Result<()> {
    let mut table = app.resources_table();
    let abort_tx = table.take::<AbortSender>(txid)?;
    if let Some(abort_tx) = Arc::into_inner(abort_tx) {
        abort_tx.0.send(()).ok();
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchSendResponseMeta {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    bodyid: ResourceId,
}

#[command]
pub(crate) async fn fetch_send<R: Runtime>(
    app: Webview<R>,
    rid: ResourceId,
    rxid: ResourceId,
    txid: ResourceId,
) -> Result<FetchSendResponseMeta> {
    let (req, abort_rx) = {
        let mut table = app.resources_table();
        let req = table.get::<FetchRequest>(rid)?;
        let abort_rx = table.take::<AbortReceiver>(rxid)?;
        (req, abort_rx)
    };

    let Some(abort_rx) = Arc::into_inner(abort_rx) else {
        return Err(crate::Error::Canceled);
    };

    let mut fut = req.0.lock().await;

    let res: Response = tokio::select! {
        r = fut.as_mut() => r?,
        _ = abort_rx.0 => {
            let mut table =app.resources_table();
            table.close(rid)?;
            return Err(crate::Error::Canceled);
        }
    };

    let status = res.status();

    let mut headers_vec = Vec::new();

    for (k, v) in res.headers().iter() {
        headers_vec.push((
            k.as_str().into(),
            v.to_str().unwrap_or_default().to_string(),
        ));
    }

    let mut table = app.resources_table();

    let bodyid = table.add(FetchBody(Mutex::new(res.into_body().into_data_stream())));

    table.close(rid).ok();
    table.close(rxid).ok();
    table.close(txid).ok();

    Ok(FetchSendResponseMeta {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or_default().to_string(),
        headers: headers_vec,
        bodyid,
    })
}

#[command]
pub(crate) async fn fetch_read_body<R: Runtime>(
    webview: Webview<R>,
    bodyid: ResourceId,
) -> Result<Vec<u8>> {
    let body = {
        let table = webview.resources_table();
        table.get::<FetchBody>(bodyid)?
    };

    let mut body = body.0.lock().await;

    let Some(chunk) = body.frame().await.transpose()? else {
        // when Option::None
        let mut resources_table = webview.resources_table();
        resources_table.close(bodyid)?;

        return Ok(vec![1]);
    };

    let chunk = match chunk.into_data() {
        Ok(c) => c,
        Err(_) => {
            // ignore Trailers
            return Ok(vec![0]);
        }
    };

    let mut chunk = chunk.to_vec();

    // append a 0 byte to indicate that the body is not empty
    chunk.push(0);

    Ok(chunk.to_vec())
}

#[command]
pub async fn fetch_cancel_body<R: Runtime>(
    webview: Webview<R>,
    bodyid: ResourceId,
) -> crate::Result<()> {
    let mut table = webview.resources_table();
    table.close(bodyid)?;
    Ok(())
}
