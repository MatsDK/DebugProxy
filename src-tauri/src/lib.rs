mod proxy;

use proxy::{ProxyState, start_proxy, stop_proxy, get_ca_cert, get_local_ip};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tauri::command]
async fn toggle_ssl_intercept(state: tauri::State<'_, ProxyState>, enabled: bool) -> Result<(), String> {
    let mut intercept = state.intercept_ssl.lock().await;
    *intercept = enabled;
    Ok(())
}

#[tauri::command]
async fn is_ssl_intercept_enabled(state: tauri::State<'_, ProxyState>) -> Result<bool, String> {
    let intercept = state.intercept_ssl.lock().await;
    Ok(*intercept)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build()) // if tauri-plugin-log exists
        .plugin(tauri_plugin_opener::init())
        .manage(ProxyState {
            is_running: Arc::new(Mutex::new(false)),
            ca_cert_pem: Arc::new(Mutex::new(None)),
            port: Arc::new(Mutex::new(None)),
            proxy_task: Arc::new(Mutex::new(None)),
            intercept_ssl: Arc::new(Mutex::new(true)),
        })
        .invoke_handler(tauri::generate_handler![
            start_proxy,
            stop_proxy,
            get_ca_cert,
            get_local_ip,
            toggle_ssl_intercept,
            is_ssl_intercept_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
