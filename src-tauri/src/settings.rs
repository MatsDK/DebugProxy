use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FilterConfig {
    pub id: String,
    pub filter_protocol: String, // "http" | "https" | "all"
    pub filter_host: String,
    pub filter_port: String,
    pub filter_path: String,
    pub filter_query: String,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScriptConfig {
    pub id: String,
    pub name: String,
    pub pattern: String,
    #[serde(default)]
    pub description: Option<String>,
    pub code: String,
    pub enabled: bool,
    #[serde(default)]
    pub compile_error: Option<String>,
    pub filters: Vec<FilterConfig>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub port: u16,
    pub intercept_ssl: bool,
    pub is_blocked: bool,
    pub theme: String,
    pub scripts: Vec<ScriptConfig>,
    pub ssl_bypass_hosts: Vec<String>,
    pub scripts_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            port: 8080,
            intercept_ssl: true,
            is_blocked: false,
            theme: "light".to_string(),
            scripts: vec![],
            ssl_bypass_hosts: vec!["*.apple.com".to_string()],
            scripts_enabled: true,
        }
    }
}

pub struct SettingsManager {
    config_path: PathBuf,
}

impl SettingsManager {
    pub fn new<R: tauri::Runtime>(app: &AppHandle<R>) -> Self {
        let mut path = app.path().app_config_dir().expect("Failed to get config dir");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.push("settings.json");
        Self { config_path: path }
    }

    pub fn load(&self) -> AppSettings {
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
        let default_settings = AppSettings::default();
        self.save(&default_settings);
        default_settings
    }

    pub fn save(&self, settings: &AppSettings) {
        if let Ok(json) = serde_json::to_string_pretty(settings) {
            if let Err(e) = fs::write(&self.config_path, json) {
                log::error!("Failed to write settings to {:?}: {}", self.config_path, e);
            }
        }
    }
}
