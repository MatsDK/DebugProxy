mod cert;
mod procs;
mod proxy;

use procs::{Api, ApiImpl, Scripts, ScriptsImpl};
use proxy::ProxyState;
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
                .bigint(specta_typescript::BigIntExportBehavior::String)
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
        .invoke_handler(router.into_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
