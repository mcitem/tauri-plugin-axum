use tauri::ipc::Request as IpcRequest;
use tauri::{command, AppHandle, Runtime};

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
