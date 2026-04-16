use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use hudsucker::{certificate_authority::RcgenAuthority, Proxy};
use rcgen::{Issuer, KeyPair};
use tauri::Runtime;

use crate::{
    cert::load_or_create_ca,
    proxy::{ProxyEvent, ProxyHandler, ProxyState, ScriptResult},
    settings::{AppSettings, SettingsManager},
};

#[taurpc::procedures(export_to = "../src/lib/bindings.ts")]
pub trait Api {
    async fn get_local_ip() -> Result<String, String>;

    async fn is_ssl_intercept_enabled() -> Result<bool, String>;

    async fn toggle_ssl_intercept(enabled: bool) -> Result<(), String>;

    async fn get_ca_cert() -> Result<Option<String>, String>;

    async fn start_proxy<R: Runtime>(port: u16, app_handle: tauri::AppHandle<R>)
        -> Result<(), String>;

    async fn stop_proxy() -> Result<(), String>;
    async fn is_blocked() -> Result<bool, String>;
    async fn toggle_blocked(enabled: bool) -> Result<(), String>;
    async fn get_event_by_id(id: String) -> Result<Option<ProxyEvent>, String>;
    async fn open_detached_window(label: String, title: String, url: String) -> Result<(), String>;

    async fn get_settings() -> Result<AppSettings, String>;
    async fn save_settings(settings: AppSettings) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ApiImpl {
    pub state: ProxyState,
}

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn get_local_ip(self) -> Result<String, String> {
        local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .map_err(|e| e.to_string())
    }

    async fn is_ssl_intercept_enabled(self) -> Result<bool, String> {
        Ok(self.state.intercept_ssl.load(Ordering::Relaxed))
    }

    async fn toggle_ssl_intercept(self, enabled: bool) -> Result<(), String> {
        self.state.intercept_ssl.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    async fn get_ca_cert(self) -> Result<Option<String>, String> {
        Ok(self.state.ca_cert_pem.lock().await.clone())
    }

    async fn start_proxy<R: Runtime>(
        self,
        port: u16,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        if self.state.is_running.load(Ordering::Relaxed) {
            return Err("Proxy is already running".into());
        }

        let (cert_pem, key_pem) = load_or_create_ca(&app_handle)?;
        let key_pair_parsed = KeyPair::from_pem(&key_pem).map_err(|e| e.to_string())?;
        let issuer =
            Issuer::from_ca_cert_pem(&cert_pem, key_pair_parsed).map_err(|e| e.to_string())?;

        let ca = RcgenAuthority::new(issuer, 1_000, rustls::crypto::ring::default_provider());
        let handler = ProxyHandler::new(
            app_handle.clone(),
            self.state.intercept_ssl.clone(),
            self.state.is_running.clone(),
            self.state.next_id.clone(),
            self.state.next_script_id.clone(),
            self.state.scripting_enabled.clone(),
            self.state.script_pending.clone(),
            self.state.script_patterns.clone(),
            self.state.is_blocked.clone(),
            self.state.history.clone(),
            cert_pem.clone(),
        );

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let proxy = Proxy::builder()
            .with_addr(SocketAddr::from(([0, 0, 0, 0], port)))
            .with_ca(ca)
            .with_rustls_connector(rustls::crypto::ring::default_provider())
            .with_http_handler(handler)
            .with_graceful_shutdown(async move {
                rx.await.ok();
            })
            .build()
            .map_err(|e| e.to_string())?;

        let is_running_clone = self.state.is_running.clone();
        let handle = tokio::spawn(async move {
            is_running_clone.store(true, Ordering::Relaxed);
            if let Err(e) = proxy.start().await {
                log::error!("Proxy error: {}", e);
            }
            is_running_clone.store(false, Ordering::Relaxed);
        });

        *self.state.proxy_task.lock().await = Some(handle);
        *self.state.shutdown_signal.lock().await = Some(tx);
        *self.state.ca_cert_pem.lock().await = Some(cert_pem);
        *self.state.port.lock().await = Some(port);

        Ok(())
    }

    async fn stop_proxy(self) -> Result<(), String> {
        if let Some(tx) = self.state.shutdown_signal.lock().await.take() {
            let _ = tx.send(());
        }

        let mut task_opt = self.state.proxy_task.lock().await;
        if let Some(handle) = task_opt.take() {
            handle.abort();
            self.state.is_running.store(false, Ordering::Relaxed);
            *self.state.port.lock().await = None;
            return Ok(());
        }
        Err("Proxy is not running".into())
    }

    async fn is_blocked(self) -> Result<bool, String> {
        Ok(self.state.is_blocked.load(Ordering::Relaxed))
    }

    async fn toggle_blocked(self, enabled: bool) -> Result<(), String> {
        self.state.is_blocked.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    async fn get_settings(self) -> Result<AppSettings, String> {
        let app_handle_opt = self.state.app_handle.lock().unwrap().clone();
        if let Some(app) = app_handle_opt {
            let manager = SettingsManager::new(&app);
            Ok(manager.load())
        } else {
            Err("AppHandle not initialized".into())
        }
    }

    async fn save_settings(self, settings: AppSettings) -> Result<(), String> {
        let app_handle_opt = self.state.app_handle.lock().unwrap().clone();
        if let Some(app) = app_handle_opt {
            let manager = SettingsManager::new(&app);
            manager.save(&settings);
            
            // Sync specific global settings efficiently:
            self.state.intercept_ssl.store(settings.intercept_ssl, Ordering::Relaxed);
            self.state.is_blocked.store(settings.is_blocked, Ordering::Relaxed);
            // Updating port and scripts dynamically usually is more complex, but we 
            // just sync the basic states that are AtomicBool for immediate apply.
            
            Ok(())
        } else {
            Err("AppHandle not initialized".into())
        }
    }

    async fn get_event_by_id(self, id: String) -> Result<Option<ProxyEvent>, String> {
        let history = self.state.history.lock().await;
        Ok(history.get(&id).cloned())
    }

    async fn open_detached_window(
        self,
        label: String,
        title: String,
        url: String,
    ) -> Result<(), String> {
        let handle_opt = self.state.app_handle.lock().unwrap();
        let handle = handle_opt
            .as_ref()
            .ok_or("App handle not initialized")?;

        let _window = tauri::WebviewWindowBuilder::new(handle, label, tauri::WebviewUrl::App(url.into()))
            .title(title)
            .inner_size(800.0, 600.0)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[taurpc::procedures(path = "scripts", export_to = "../src/lib/bindings.ts")]
pub trait Scripts {
    async fn set_script_patterns(patterns: Vec<String>) -> Result<(), String>;

    async fn toggle_scripting(enabled: bool) -> Result<(), String>;

    async fn submit_script_result(script_id: String, result: ScriptResult) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ScriptsImpl {
    pub state: ProxyState,
}

#[taurpc::resolvers]
impl Scripts for ScriptsImpl {
    async fn set_script_patterns(self, patterns: Vec<String>) -> Result<(), String> {
        let mut script_patterns = self.state.script_patterns.write().unwrap();
        let mut next = Vec::new();
        for p in patterns {
            match regex::RegexBuilder::new(&p).case_insensitive(true).build() {
                Ok(re) => next.push(re),
                Err(e) => return Err(format!("Invalid regex pattern '{}': {}", p, e)),
            }
        }
        *script_patterns = next;
        Ok(())
    }

    async fn toggle_scripting(self, enabled: bool) -> Result<(), String> {
        self.state.scripting_enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    async fn submit_script_result(
        self,
        script_id: String,
        result: ScriptResult,
    ) -> Result<(), String> {
        let id_u64: u64 = script_id.parse().map_err(|e| format!("Invalid script_id: {}", e))?;
        let mut pending = self.state.script_pending.lock().await;
        if let Some(tx) = pending.remove(&id_u64) {
            let _ = tx.send(result);
            Ok(())
        } else {
            Err("Request ID not found or already timed out".into())
        }
    }
}
