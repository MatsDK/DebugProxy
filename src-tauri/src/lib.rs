mod proxy;

use proxy::{ProxyState, start_proxy, stop_proxy, get_ca_cert, get_local_ip, toggle_ssl_intercept, is_ssl_intercept_enabled, submit_script_result, toggle_scripting, set_script_patterns};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(ProxyState::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_proxy,
            stop_proxy,
            get_ca_cert,
            get_local_ip,
            toggle_ssl_intercept,
            is_ssl_intercept_enabled,
            submit_script_result,
            toggle_scripting,
            set_script_patterns
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
