mod cert;
mod procs;
mod proxy;
mod settings;

use procs::{Api, ApiImpl, Scripts, ScriptsImpl};
use proxy::ProxyState;
use tauri::Manager;
use taurpc::Router;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    // ProxyState is Clone and all its fields are Arc<...>, so cloning
    // gives both handlers the *same* underlying shared state. There is probably a better way to share the state...
    let state = ProxyState::default();

    let router = Router::new()
        .export_config(
            specta_typescript::Typescript::default()
                .header("// My header\n")
                .bigint(specta_typescript::BigIntExportBehavior::String),
        )
        .merge(
            ApiImpl {
                state: state.clone(),
            }
            .into_handler(),
        )
        .merge(
            ScriptsImpl {
                state: state.clone(),
            }
            .into_handler(),
        );

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Emit a GLOBAL event via app_handle so all listeners see it
                use tauri::Emitter;
                let _ = window.app_handle().emit("window-closed", window.label());
            }
        })
        .setup({
            let state = state.clone();
            move |app| {
                *state.app_handle.lock().unwrap() = Some(app.handle().clone());
                Ok(())
            }
        })
        .invoke_handler(router.into_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
