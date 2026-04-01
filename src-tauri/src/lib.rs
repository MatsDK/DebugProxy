mod proxy;

use proxy::{ProxyState, start_proxy, stop_proxy, get_ca_cert, get_local_ip, toggle_ssl_intercept, is_ssl_intercept_enabled};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build()) // if tauri-plugin-log exists
        .plugin(tauri_plugin_opener::init())
        .manage(ProxyState::default())
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
