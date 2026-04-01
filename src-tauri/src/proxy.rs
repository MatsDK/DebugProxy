use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{header, Request, Response, StatusCode, body::Bytes},
    rcgen::{CertificateParams, KeyPair, Issuer},
    HttpContext, HttpHandler, Proxy, RequestOrResponse,
};
use http_body_util::{Full, BodyExt};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use std::net::SocketAddr;
use std::collections::HashMap;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::{Regex, RegexBuilder};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Clone, serde::Serialize)]
pub struct ProxyEvent {
    pub id: u64,
    pub script_id: u64,
    pub timestamp: u64,
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub is_response: bool,
    pub status: Option<u16>,
    pub body_base64: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ScriptResult {
    pub headers: Option<Vec<(String, String)>>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub body_base64: Option<String>,
    pub dropped: bool,
}

pub struct ProxyState {
    pub is_running: Arc<AtomicBool>,
    pub ca_cert_pem: Arc<Mutex<Option<String>>>,
    pub port: Arc<Mutex<Option<u16>>>,
    pub proxy_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub intercept_ssl: Arc<AtomicBool>,
    pub shutdown_signal: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub next_id: Arc<AtomicU64>,
    pub next_script_id: Arc<AtomicU64>,
    pub scripting_enabled: Arc<AtomicBool>,
    pub script_pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ScriptResult>>>>,
    pub script_patterns: Arc<std::sync::RwLock<Vec<Regex>>>,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            ca_cert_pem: Arc::new(Mutex::new(None)),
            port: Arc::new(Mutex::new(None)),
            proxy_task: Arc::new(Mutex::new(None)),
            intercept_ssl: Arc::new(AtomicBool::new(true)),
            shutdown_signal: Arc::new(Mutex::new(None)),
            next_id: Arc::new(AtomicU64::new(1)),
            next_script_id: Arc::new(AtomicU64::new(1)),
            scripting_enabled: Arc::new(AtomicBool::new(true)),
            script_pending: Arc::new(Mutex::new(HashMap::new())),
            script_patterns: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }
}

#[derive(Clone)]
pub struct ProxyHandler {
    app_handle: AppHandle,
    intercept_ssl: Arc<AtomicBool>,
    is_running: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
    next_script_id: Arc<AtomicU64>,
    scripting_enabled: Arc<AtomicBool>,
    script_pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ScriptResult>>>>,
    script_patterns: Arc<std::sync::RwLock<Vec<Regex>>>,
    ca_cert_pem: String,
    request_id: Option<u64>,
    request_timestamp: Option<u64>,
    request_method: Option<String>,
    request_uri: Option<String>,
}

impl ProxyHandler {
    pub fn new(
        app_handle: AppHandle, 
        intercept_ssl: Arc<AtomicBool>, 
        is_running: Arc<AtomicBool>, 
        next_id: Arc<AtomicU64>,
        next_script_id: Arc<AtomicU64>,
        scripting_enabled: Arc<AtomicBool>,
        script_pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ScriptResult>>>>,
        script_patterns: Arc<std::sync::RwLock<Vec<Regex>>>,
        ca_cert_pem: String,
    ) -> Self {
        Self {
            app_handle,
            intercept_ssl,
            is_running,
            next_id,
            next_script_id,
            scripting_enabled,
            script_pending,
            script_patterns,
            ca_cert_pem,
            request_id: None,
            request_timestamp: None,
            request_method: None,
            request_uri: None,
        }
    }

    fn generate_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn generate_script_id(&self) -> u64 {
        self.next_script_id.fetch_add(1, Ordering::Relaxed)
    }
}

impl HttpHandler for ProxyHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<hudsucker::Body>,
    ) -> RequestOrResponse {
        if !self.is_running.load(Ordering::Relaxed) {
            return RequestOrResponse::Request(req);
        }

        if req.uri().host() == Some("proxy.local") {
            println!("Get ca certificate from proxy.local");
            let (body, content_type, disposition) = match req.uri().path() {
                "/ca.crt" => (self.ca_cert_pem.clone(), "application/x-x509-ca-cert", Some("attachment; filename=ca.crt")),
                _ => (
                    "<html><body style='font-family:sans-serif;padding:40px;max-width:600px;margin:auto'><h1>Debug Proxy</h1><p><a href='/ca.crt' style='display:inline-block;padding:10px 20px;background:#6366f1;color:white;text-decoration:none;border-radius:6px'>Download Root CA</a></p><p>Install this certificate to intercept HTTPS traffic.</p></body></html>".to_string(),
                    "text/html",
                    None,
                ),
            };
            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type);
            
            if let Some(disp) = disposition {
                builder = builder.header(header::CONTENT_DISPOSITION, disp);
            }

            return RequestOrResponse::Response(
                builder.body(hudsucker::Body::from(Full::new(Bytes::from(body))))
                    .unwrap()
            );
        }

        let id = self.generate_id();
        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let headers: Vec<(String, String)> = req.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.request_id = Some(id);
        self.request_timestamp = Some(timestamp);
        self.request_method = Some(method.clone());
        self.request_uri = Some(uri.clone());

        // Selective Buffering Check
        let method_ref = req.method();
        let is_script_matched = if self.scripting_enabled.load(Ordering::Relaxed) {
             let patterns = self.script_patterns.read().unwrap();
             if !patterns.is_empty() {
                println!("[Proxy] Testing URI: \"{}\" against {} script patterns", uri, patterns.len());
                let any_match = patterns.iter().any(|re| {
                    let m = re.is_match(&uri);
                    println!("  - Pattern \"{}\" -> {}", re.as_str(), m);
                    m
                });
                any_match
             } else {
                println!("[Proxy] No script patterns active in backend. Match skipped.");
                false
             }
        } else {
             false
        };

        let req_content_length = req.headers().get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        
        let req_is_large = req_content_length > 5 * 1024 * 1024;

        // We only buffer if it's NOT a CONNECT/GET/HEAD and not a gigant blob
        // OR if it's a script match.
        let should_buffer = is_script_matched || (method_ref != hudsucker::hyper::Method::GET && 
                          method_ref != hudsucker::hyper::Method::HEAD && 
                          method_ref != hudsucker::hyper::Method::CONNECT &&
                          !req_is_large);

        let (body_b64, mut req) = if should_buffer {
            let (parts, body) = req.into_parts();
            // Faster timeout for log-only collection (1s), longer for scripts (5s)
            let timeout_dur = if is_script_matched { std::time::Duration::from_secs(10) } else { std::time::Duration::from_secs(2) };
            
            match tokio::time::timeout(timeout_dur, body.collect()).await {
                Ok(Ok(collected)) => {
                    let b = collected.to_bytes();
                    (Some(STANDARD.encode(&*b)), Request::from_parts(parts, hudsucker::Body::from(Full::new(b))))
                },
                _ => (None, Request::from_parts(parts, hudsucker::Body::empty())) 
            }
        } else {
            (None, req)
        };

        // Emit request event
        let event = ProxyEvent {
            id,
            script_id: 0,
            timestamp,
            method: method.clone(),
            uri: uri.clone(),
            headers: headers.clone(),
            is_response: false,
            status: None,
            body_base64: body_b64.clone(),
        };
        let _ = self.app_handle.emit("proxy_request", &event);

        // Scripting Hook
        if is_script_matched {
            let script_id = self.generate_script_id();
            let mut script_event = event;
            script_event.script_id = script_id;
            script_event.body_base64 = body_b64;

            let (tx, rx) = oneshot::channel();
            self.script_pending.lock().await.insert(script_id, tx);
            let _ = self.app_handle.emit("proxy_script_request", &script_event);
            
            if let Ok(Ok(result)) = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
                if result.dropped {
                    return RequestOrResponse::Response(
                        Response::builder()
                            .status(StatusCode::FORBIDDEN)
                            .body(hudsucker::Body::empty())
                            .unwrap()
                    );
                }
                if let Some(new_uri) = result.uri {
                    if let Ok(parsed_uri) = new_uri.parse::<hudsucker::hyper::Uri>() {
                        *req.uri_mut() = parsed_uri;
                    }
                }
                if let Some(new_headers) = result.headers {
                    req.headers_mut().clear();
                    for (k, v) in new_headers {
                        if let (Ok(name), Ok(value)) = (hudsucker::hyper::header::HeaderName::from_bytes(k.as_bytes()), hudsucker::hyper::header::HeaderValue::from_str(&v)) {
                            req.headers_mut().insert(name, value);
                        }
                    }
                }
                if let Some(new_body_b64) = result.body_base64 {
                    if let Ok(bytes) = STANDARD.decode(new_body_b64) {
                        *req.body_mut() = hudsucker::Body::from(Full::new(Bytes::from(bytes)));
                    }
                }
            } else {
                self.script_pending.lock().await.remove(&script_id);
            }
        }

        RequestOrResponse::Request(req)
    }

    async fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        res: Response<hudsucker::Body>,
    ) -> Response<hudsucker::Body> {
        let id = self.request_id.unwrap_or(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let headers: Vec<(String, String)> = res.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let method = self.request_method.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        let uri = self.request_uri.clone().unwrap_or_else(|| "UNKNOWN".to_string());

        // Selective Buffering Check
        let is_script_matched = if self.scripting_enabled.load(Ordering::Relaxed) {
             let patterns = self.script_patterns.read().unwrap();
             !patterns.is_empty() && patterns.iter().any(|re| re.is_match(&uri))
        } else {
             false
        };

        let content_length = res.headers().get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let is_large = content_length > 5 * 1024 * 1024;

        let content_type = res.headers().get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        
        let is_media = content_type.starts_with("image/") ||
                       content_type.starts_with("video/") ||
                       content_type.starts_with("audio/") ||
                       content_type.starts_with("font/");

        let should_buffer = (!is_media && !is_large) || is_script_matched;
// ... (rest of handle_response logic remains same)
        let (body_b64, mut res) = if should_buffer {
            let (parts, body) = res.into_parts();
            let timeout_dur = if is_script_matched { std::time::Duration::from_secs(10) } else { std::time::Duration::from_secs(2) };
            
            match tokio::time::timeout(timeout_dur, body.collect()).await {
                Ok(Ok(collected)) => {
                    let b = collected.to_bytes();
                    (Some(STANDARD.encode(&*b)), Response::from_parts(parts, hudsucker::Body::from(Full::new(b))))
                },
                _ => (None, Response::from_parts(parts, hudsucker::Body::empty()))
            }
        } else {
            (None, res)
        };

        // Emit response event
        let event = ProxyEvent {
            id,
            script_id: 0,
            timestamp,
            method,
            uri,
            headers,
            is_response: true,
            status: Some(res.status().as_u16()),
            body_base64: body_b64.clone(), 
        };
        let _ = self.app_handle.emit("proxy_response", &event);

        // Scripting Hook
        if is_script_matched {
            let script_id = self.generate_script_id();
            let mut script_event = event;
            script_event.script_id = script_id;
            script_event.body_base64 = body_b64;

            let (tx, rx) = oneshot::channel();
            self.script_pending.lock().await.insert(script_id, tx);
            let _ = self.app_handle.emit("proxy_script_response", &script_event);
            
            if let Ok(Ok(result)) = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
                if let Some(new_status) = result.status {
                    if let Ok(status) = hudsucker::hyper::StatusCode::from_u16(new_status) {
                        *res.status_mut() = status;
                    }
                }
                if let Some(new_headers) = result.headers {
                    res.headers_mut().clear();
                    for (k, v) in new_headers {
                        if let (Ok(name), Ok(value)) = (hudsucker::hyper::header::HeaderName::from_bytes(k.as_bytes()), hudsucker::hyper::header::HeaderValue::from_str(&v)) {
                            res.headers_mut().insert(name, value);
                        }
                    }
                }
                if let Some(new_body_b64) = result.body_base64 {
                    if let Ok(bytes) = STANDARD.decode(new_body_b64) {
                        *res.body_mut() = hudsucker::Body::from(Full::new(Bytes::from(bytes)));
                    }
                }
            } else {
                self.script_pending.lock().await.remove(&script_id);
            }
        }
        
        res
    }

    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<hudsucker::Body>) -> bool {
        let host = req.uri().host().unwrap_or("");
        if host == "proxy.local" {
            return true;
        }

        if !self.is_running.load(Ordering::Relaxed) {
            return false;
        }

        // if req.uri().scheme_str() == Some("http") {
        //     return true;
        // }

        if !self.intercept_ssl.load(Ordering::Relaxed) {
            println!("[Proxy] Bypassing SSL for host: {} (SSL Intercept disabled)", host);
            return false;
        }

        println!("[Proxy] Intercepting request for host: {}", host);
        true
    }
}

#[tauri::command]
pub async fn get_local_ip() -> Result<String, String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_ssl_intercept(state: tauri::State<'_, ProxyState>, enabled: bool) -> Result<(), String> {
    state.intercept_ssl.store(enabled, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn is_ssl_intercept_enabled(state: tauri::State<'_, ProxyState>) -> Result<bool, String> {
    Ok(state.intercept_ssl.load(Ordering::Relaxed))
}

fn ca_paths(app: &AppHandle) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok((data_dir.join("ca.crt"), data_dir.join("ca.key")))
}

fn load_or_create_ca(app: &AppHandle) -> Result<(String, String), String> {
    let (cert_path, key_path) = ca_paths(app)?;

    if cert_path.exists() && key_path.exists() {
        let cert_pem = std::fs::read_to_string(&cert_path).map_err(|e| e.to_string())?;
        let key_pem  = std::fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
        if KeyPair::from_pem(&key_pem).is_ok() {
            return Ok((cert_pem, key_pem));
        }
    }

    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    let mut dn = hudsucker::rcgen::DistinguishedName::new();
    dn.push(hudsucker::rcgen::DnType::CommonName, "Debug Proxy Root CA");
    dn.push(hudsucker::rcgen::DnType::OrganizationName, "MDK");
    params.distinguished_name = dn;
    params.is_ca = hudsucker::rcgen::IsCa::Ca(hudsucker::rcgen::BasicConstraints::Unconstrained);
    params.key_usages = vec![
        hudsucker::rcgen::KeyUsagePurpose::KeyCertSign,
        hudsucker::rcgen::KeyUsagePurpose::DigitalSignature,
        hudsucker::rcgen::KeyUsagePurpose::CrlSign,
    ];

    let key_pair = KeyPair::generate_for(&hudsucker::rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| e.to_string())?;
    let cert = params.self_signed(&key_pair).map_err(|e| e.to_string())?;

    let cert_pem = cert.pem();
    let key_pem  = key_pair.serialize_pem();

    std::fs::write(&cert_path, &cert_pem).map_err(|e| e.to_string())?;
    std::fs::write(&key_path, &key_pem).map_err(|e| e.to_string())?;

    Ok((cert_pem, key_pem))
}

#[tauri::command]
pub async fn start_proxy(
    app: AppHandle,
    state: tauri::State<'_, ProxyState>,
    port: u16,
) -> Result<(), String> {
    if state.is_running.load(Ordering::Relaxed) {
        return Err("Proxy is already running".into());
    }

    let (cert_pem, key_pem) = load_or_create_ca(&app)?;
    let key_pair_parsed = KeyPair::from_pem(&key_pem).map_err(|e| e.to_string())?;
    let issuer = Issuer::from_ca_cert_pem(&cert_pem, key_pair_parsed).map_err(|e| e.to_string())?;

    let ca = RcgenAuthority::new(issuer, 1_000, rustls::crypto::ring::default_provider());
    let handler = ProxyHandler::new(
        app, 
        state.intercept_ssl.clone(), 
        state.is_running.clone(), 
        state.next_id.clone(),
        state.next_script_id.clone(),
        state.scripting_enabled.clone(),
        state.script_pending.clone(),
        state.script_patterns.clone(),
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

    let is_running_clone = state.is_running.clone();
    let handle = tokio::spawn(async move {
        is_running_clone.store(true, Ordering::Relaxed);
        if let Err(e) = proxy.start().await {
            log::error!("Proxy error: {}", e);
        }
        is_running_clone.store(false, Ordering::Relaxed);
    });

    *state.proxy_task.lock().await = Some(handle);
    *state.shutdown_signal.lock().await = Some(tx);
    *state.ca_cert_pem.lock().await = Some(cert_pem);
    *state.port.lock().await = Some(port);

    Ok(())
}

#[tauri::command]
pub async fn stop_proxy(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    if let Some(tx) = state.shutdown_signal.lock().await.take() {
        let _ = tx.send(());
    }

    let mut task_opt = state.proxy_task.lock().await;
    if let Some(handle) = task_opt.take() {
        handle.abort();
        state.is_running.store(false, Ordering::Relaxed);
        *state.port.lock().await = None;
        return Ok(());
    }
    Err("Proxy is not running".into())
}

#[tauri::command]
pub async fn get_ca_cert(state: tauri::State<'_, ProxyState>) -> Result<Option<String>, String> {
    Ok(state.ca_cert_pem.lock().await.clone())
}

#[tauri::command]
pub async fn set_script_patterns(state: tauri::State<'_, ProxyState>, patterns: Vec<String>) -> Result<(), String> {
    println!("[Proxy] Updating script patterns in backend: {:?}", patterns);
    let mut script_patterns = state.script_patterns.write().unwrap();
    let mut next = Vec::new();
    for p in patterns {
        // Build regex with case-insensitivity to match frontend behavior
        match RegexBuilder::new(&p)
            .case_insensitive(true)
            .build() {
            Ok(re) => next.push(re),
            Err(e) => return Err(format!("Invalid regex pattern '{}': {}", p, e)),
        }
    }
    *script_patterns = next;
    Ok(())
}

#[tauri::command]
pub async fn toggle_scripting(state: tauri::State<'_, ProxyState>, enabled: bool) -> Result<(), String> {
    println!("[Proxy] Toggling scripting: {}", enabled);
    state.scripting_enabled.store(enabled, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn submit_script_result(
    state: tauri::State<'_, ProxyState>,
    script_id: u64,
    result: ScriptResult,
) -> Result<(), String> {
    let mut pending = state.script_pending.lock().await;
    if let Some(tx) = pending.remove(&script_id) {
        let _ = tx.send(result);
        Ok(())
    } else {
        Err("Request ID not found or already timed out".into())
    }
}
