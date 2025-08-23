mod router;
use tauri::{ipc, Manager, Runtime};

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
        // .plugin(tauri_plugin_axum::init(router::router()))
        .setup(|app| {
            let path = app.path().app_config_dir()?;
            app.handle().plugin(tauri_plugin_axum::block_init(async {
                println!("do something with path: {:?}", path);
                router::router()
            }))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![custom_usage])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
