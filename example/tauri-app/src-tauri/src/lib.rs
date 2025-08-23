mod router;
use tauri::{ipc, Runtime};

#[tauri::command]
async fn custom_usage<R: Runtime>(
    app: tauri::AppHandle<R>,
    req: ipc::Request<'_>,
) -> Result<tauri_plugin_axum::AxumResponse, tauri_plugin_axum::Error> {
    use tauri_plugin_axum::AxumExt;
    app.axum().call(req).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // https://docs.rs/tauri/2.8.2/tauri/plugin/struct.Builder.html#known-limitations
        // Known limitations
        // URI scheme protocols are registered when the webview is created.
        // Due to this limitation, if the plugin is registered after a webview has been created,
        // protocol won’t be available.
        // macOS, iOS and Linux: axum://localhost/<path>
        // Windows and Android: http://axum.localhost/<path> by default
        // if use protocol, register it before setup hook
        .plugin(tauri_plugin_axum::init(router::router()))
        // .setup(|app| {
        // let path = app.path().app_config_dir()?;
        // app.handle().plugin(tauri_plugin_axum::block_init(async {
        //     println!("do something with path: {:?}", path);
        //     router::router()
        // }))?;
        // app.handle()
        //     .plugin(tauri_plugin_axum::try_block_init(async {
        //         return Err("throw error in setup".into());
        //         // Ok(router::router())
        //     })?)?;
        // Ok(())
        // })
        .invoke_handler(tauri::generate_handler![custom_usage])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
