use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Uri},
    response::Response,
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};
use tauri::{
    command,
    ipc::{InvokeResponseBody, Request as IpcRequest},
    AppHandle, Manager, ResourceId, Runtime,
};
use tokio::sync::{
    broadcast::{channel, Receiver, Sender},
    Mutex,
};
use tower::ServiceExt;

use crate::models::*;
use crate::AxumExt;
use crate::Result;

#[command]
pub(crate) async fn call<R: Runtime>(
    app: AppHandle<R>,
    req: IpcRequest<'_>,
) -> Result<AxumResponse> {
    app.axum().call(req).await
}

struct FetchRequest(Mutex<Pin<Box<dyn Future<Output = Result<Response>> + Send>>>);
struct AbortSender(Sender<()>);
struct AbortReceiver(Receiver<()>);
struct FetchBody(Mutex<Body>);

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
pub(crate) async fn fetch<R: Runtime>(app: AppHandle<R>, conf: FetchConf) -> Result<FetchReturn> {
    let mut headers = HeaderMap::new();
    for (k, v) in conf.headers.iter() {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) {
            headers.append(name, value);
        }
    }

    let method = Method::from_bytes(conf.method.as_bytes()).unwrap_or(Method::GET);
    let uri: Uri = conf.uri.parse().unwrap_or_else(|_| Uri::from_static("/"));
    let body = match conf.body {
        Some(b) => Body::from(b),
        None => Body::empty(),
    };
    let mut req_builder = Request::builder().method(method).uri(uri.clone());
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
    let (tx, rx) = channel::<()>(1);
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
    app: AppHandle<R>,
    rid: ResourceId,
    rxid: ResourceId,
    txid: ResourceId,
) -> Result<FetchSendResponseMeta> {
    let (req, abort_rx) = {
        let table = app.resources_table();
        let req = table.get::<FetchRequest>(rid)?;
        let abort_rx = table.get::<AbortReceiver>(rxid)?;
        (req, abort_rx)
    };
    let mut ab = abort_rx.0.resubscribe();
    let mut fut = req.0.lock().await;
    let res: Response = tokio::select! {
        r = fut.as_mut() => r?,
        _ = ab.recv() => {
            let mut table =app.resources_table();
            table.close(rid).ok();
            table.close(txid).ok();
            table.close(rxid).ok();
            return Err(crate::Error::Canceled);
        }
    };

    let status = res.status();
    let mut headers_vec = Vec::new();
    for (k, v) in res.headers().iter() {
        headers_vec.push((
            k.as_str().to_string(),
            v.to_str().unwrap_or_default().to_string(),
        ));
    }

    let mut table = app.resources_table();

    let bodyid = table.add(FetchBody(Mutex::new(res.into_body())));

    table.close(rid).ok();

    Ok(FetchSendResponseMeta {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: headers_vec,
        bodyid,
    })
}

#[command]
pub(crate) async fn fetch_read_body<R: Runtime>(
    app: AppHandle<R>,
    bodyid: ResourceId,
    rxid: ResourceId,
    txid: ResourceId,
    stream_channel: tauri::ipc::Channel<InvokeResponseBody>,
) -> Result<()> {
    let body = {
        let mut table = app.resources_table();
        table.take::<FetchBody>(bodyid)?
    };
    let Some(body) = Arc::into_inner(body) else {
        return Err(crate::Error::Canceled);
    };
    let mut ab = {
        let mut table = app.resources_table();
        let ab = table.take::<AbortReceiver>(rxid)?;
        ab.0.resubscribe()
    };
    let f = async {
        let mut guard = body.0.lock().await;
        while let Some(chunk) = guard.frame().await {
            match chunk {
                Ok(data) => {
                    if let Ok(data) = data.into_data() {
                        let mut data = data.to_vec();
                        data.push(0);
                        if stream_channel.send(InvokeResponseBody::Raw(data)).is_err() {
                            break;
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        stream_channel.send(InvokeResponseBody::Raw(vec![1]))
    };
    tokio::select! {
        _ = f => {},
        _ = ab.recv() => {
            return Err(crate::Error::Canceled);
        }
    };
    let mut table = app.resources_table();
    table.close(txid).ok();
    Ok(())
}
