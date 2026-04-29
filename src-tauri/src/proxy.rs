
use http_body_util::Full;
use hudsucker::{
    hyper::{body::Bytes, header, Request, Response, StatusCode},
    Body, HttpContext, HttpHandler, RequestOrResponse,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Runtime};
use tokio::sync::{oneshot, Mutex};

fn serialize_u64_as_string<S>(val: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&val.to_string())
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(serde::de::Error::custom)
}

#[derive(Clone, serde::Serialize, serde::Deserialize, specta::Type, Debug)]
pub struct ProxyEvent {
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    #[specta(type = String)]
    pub id: u64,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    #[specta(type = String)]
    pub script_id: u64,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    #[specta(type = String)]
    pub timestamp: u64,
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub is_response: bool,
    pub status: Option<u16>,
    pub body: Option<Vec<u8>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, specta::Type)]
pub struct ScriptResult {
    pub headers: Option<Vec<(String, String)>>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub body: Option<Vec<u8>>,
    pub dropped: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, specta::Type)]
pub struct HistoryEntry {
    pub request: ProxyEvent,
    pub response: Option<ProxyEvent>,
}

#[derive(Clone)]
pub struct ProxyState {
    pub is_running: Arc<AtomicBool>,
    pub ca_cert_pem: Arc<Mutex<Option<String>>>,
    pub port: Arc<Mutex<Option<u16>>>,
    pub proxy_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub intercept_ssl: Arc<AtomicBool>,
    /// Channel to trigger graceful shutdown of the proxy server
    pub shutdown_signal: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Monotonic counter for assigning unique IDs to every intercepted request
    pub next_id: Arc<AtomicU64>,
    /// Counter for generating unique IDs for script execution contexts
    pub next_script_id: Arc<AtomicU64>,
    pub scripting_enabled: Arc<AtomicBool>,
    /// Registry of pending script results, keyed by execution ID
    pub script_pending: Arc<Mutex<HashMap<u64, oneshot::Sender<ScriptResult>>>>,
    /// Compiled regex patterns used to determine which requests trigger scripts
    pub script_patterns: Arc<std::sync::RwLock<Vec<Regex>>>,
    pub is_blocked: Arc<AtomicBool>,
    /// In-memory cache of recent traffic for detached inspector windows
    pub history: Arc<Mutex<HashMap<String, HistoryEntry>>>,
    pub app_handle: Arc<std::sync::Mutex<Option<AppHandle>>>,
    /// Patterns for hosts that should bypass SSL decryption
    pub ssl_exception_patterns: Arc<std::sync::RwLock<Vec<Regex>>>,
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
            is_blocked: Arc::new(AtomicBool::new(false)),
            history: Arc::new(Mutex::new(HashMap::new())),
            app_handle: Arc::new(std::sync::Mutex::new(None)),
            ssl_exception_patterns: Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }
}

pub struct ProxyHandler<R: Runtime> {
    app_handle: AppHandle<R>,
    state: ProxyState,
    /// Cached CA cert for this session to avoid repeated mutex locking
    ca_cert_pem_cache: String,
    // Fields for current request context
    request_id: Option<u64>,
    request_timestamp: Option<u64>,
    request_method: Option<String>,
    request_uri: Option<String>,
}

impl<R: Runtime> Clone for ProxyHandler<R> {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            state: self.state.clone(),
            ca_cert_pem_cache: self.ca_cert_pem_cache.clone(),
            request_id: None,
            request_timestamp: None,
            request_method: None,
            request_uri: None,
        }
    }
}

impl<R: Runtime> ProxyHandler<R> {
    pub fn new(
        app_handle: AppHandle<R>,
        state: ProxyState,
        ca_cert_pem_cache: String,
    ) -> Self {
        Self {
            app_handle,
            state,
            ca_cert_pem_cache,
            request_id: None,
            request_timestamp: None,
            request_method: None,
            request_uri: None,
        }
    }

    async fn emit_event(&self, event: ProxyEvent) {
        let trigger = crate::procs::AppEvents::new(self.app_handle.clone());
        let _ = trigger.proxy_event(event);
    }

    async fn collect_body(body: Body) -> Vec<u8> {
        use http_body_util::BodyExt;
        body.collect().await.map(|c| c.to_bytes().to_vec()).unwrap_or_default()
    }

    fn decompress_body(headers: &[(String, String)], body: &[u8]) -> Vec<u8> {
        let encoding = headers.iter()
            .find(|(k, _)| k.to_lowercase() == "content-encoding")
            .map(|(_, v)| v.to_lowercase())
            .unwrap_or_default();

        match encoding.as_str() {
            "gzip" => {
                use flate2::read::GzDecoder;
                use std::io::Read;
                let mut d = GzDecoder::new(body);
                let mut buffer = Vec::new();
                if d.read_to_end(&mut buffer).is_ok() {
                    buffer
                } else {
                    body.to_vec()
                }
            }
            "deflate" => {
                use flate2::read::ZlibDecoder;
                use std::io::Read;
                let mut d = ZlibDecoder::new(body);
                let mut buffer = Vec::new();
                if d.read_to_end(&mut buffer).is_ok() {
                    buffer
                } else {
                    body.to_vec()
                }
            }
            _ => body.to_vec(),
        }
    }
}

impl<R: Runtime> HttpHandler for ProxyHandler<R> {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> RequestOrResponse {
        if self.state.is_blocked.load(Ordering::Relaxed) {
            // Simulating BLOCKED: Dropping request
            return RequestOrResponse::Response(
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from(Full::new(Bytes::from("Blocked"))))
                    .unwrap()
            );
        }

        let id = self.state.next_id.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        self.request_id = Some(id);
        self.request_timestamp = Some(timestamp);
        self.request_method = Some(method.clone());

        let mut absolute_uri = uri.clone();
        if !absolute_uri.contains("://") {
            if let Some(host) = headers.iter().find(|(k, _)| k.to_lowercase() == "host").map(|(_, v)| v) {
                // If it doesn't contain ://, it's likely an intercepted HTTPS request 
                // OR a plain HTTP request with a relative URI (uncommon in proxying but possible)
                let proto = if host.ends_with(":80") || (!host.contains(':') && !self.state.intercept_ssl.load(Ordering::Relaxed)) {
                    "http"
                } else {
                    "https"
                };
                absolute_uri = format!("{}://{}{}", proto, host, uri);
            }
        }
        self.request_uri = Some(absolute_uri.clone());

        if uri.contains("proxy.local") || uri == "/proxy.local" {
            return RequestOrResponse::Response(
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/x-x509-ca-cert")
                    .header("Content-Disposition", "attachment; filename=debug_proxy_ca.crt")
                    .body(Body::from(self.ca_cert_pem_cache.clone()))
                    .unwrap(),
            );
        }

        // Special handling for CONNECT Requests:
        // We MUST NOT consume the body of a CONNECT request, as it would break
        // the underlying stream used for tunneling. We log it and return early.
        if req.method() == hudsucker::hyper::Method::CONNECT {
            let event = ProxyEvent {
                id,
                script_id: 0,
                timestamp,
                method: method.clone(),
                uri: uri.clone(),
                headers: headers.clone(),
                is_response: false,
                status: None,
                body: None,
            };

            // Cache in history
            {
                let mut history = self.state.history.lock().await;
                history.insert(id.to_string(), HistoryEntry {
                    request: event.clone(),
                    response: None,
                });
            }

            self.emit_event(event).await;
            return RequestOrResponse::Request(req);
        }

        // Always buffer so the inspector can display the body
        let (parts, body) = req.into_parts();
        let body_bytes = Self::collect_body(body).await;
        
        // Body for display in UI/Scripts (already decompressed)
        let headers_vec: Vec<(String, String)> = parts.headers.iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();
        let decompressed = Self::decompress_body(&headers_vec, &body_bytes);
        let body = if decompressed.is_empty() {
            None
        } else {
            Some(decompressed)
        };

        let scripting_enabled = self.state.scripting_enabled.load(Ordering::Relaxed);
        let pattern_match = {
            let patterns = self.state.script_patterns.read().unwrap();
            patterns.iter().any(|re| re.is_match(&absolute_uri))
        };
        
        let should_script = scripting_enabled && pattern_match && method != "CONNECT";

        let script_id = if should_script {
            self.state.next_script_id.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };

        let event = ProxyEvent {
            id,
            script_id,
            timestamp,
            method: method.clone(),
            uri: absolute_uri.clone(),
            headers: headers.clone(),
            is_response: false,
            status: None,
            body,
        };

        // Cache in history for detached windows
        {
            let mut history = self.state.history.lock().await;
            history.insert(id.to_string(), HistoryEntry {
                request: event.clone(),
                response: None,
            });
            if history.len() > 1000 {
                let keys: Vec<String> = history.keys().cloned().collect();
                if let Some(min_key) = keys.iter().min() {
                    let min_key_str: String = min_key.clone();
                    history.remove(&min_key_str);
                }
            }
        }

        self.emit_event(event).await;

        if should_script {
            let (tx, rx) = oneshot::channel();
            self.state.script_pending.lock().await.insert(script_id, tx);

            match rx.await {
                Ok(result) => {
                    if result.dropped {
                        return RequestOrResponse::Response(
                            Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Body::from("Request dropped by script"))
                                .unwrap(),
                        );
                    }

                    let mut req_parts = parts;
                    if let Some(new_uri) = result.uri.clone() {
                        if let Ok(u) = new_uri.parse() {
                            req_parts.uri = u;
                        }
                    }

                    if let Some(h) = result.headers.clone() {
                        req_parts.headers.clear();
                        for (k, v) in h {
                            if let (Ok(name), Ok(val)) = (
                                header::HeaderName::from_bytes(k.as_bytes()),
                                header::HeaderValue::from_str(&v),
                            ) {
                                req_parts.headers.insert(name, val);
                            }
                        }
                    }

                    // Proactively strip Accept-Encoding to prevent server-side compression
                    // if we are likely to modify the body. This simplifies life for the proxy.
                    req_parts.headers.remove(header::ACCEPT_ENCODING);

                    let modified = result.body.is_some();
                    let final_bytes = result.body.clone().unwrap_or(body_bytes);
                    if modified {
                        println!("[PROXY] Sending MODIFIED request body to server ({} bytes)", final_bytes.len());
                        // Remove encoding/length headers as they are now invalid
                        req_parts.headers.remove(header::CONTENT_ENCODING);
                        req_parts.headers.remove(header::TRANSFER_ENCODING);
                        req_parts.headers.insert(header::CONTENT_LENGTH, header::HeaderValue::from(final_bytes.len()));
                    }

                    // Update history with script modifications for detached windows
                    {
                        let mut history = self.state.history.lock().await;
                        if let Some(entry) = history.get_mut(&id.to_string()) {
                            if let Some(new_uri) = result.uri.as_ref() {
                                entry.request.uri = new_uri.clone();
                            }
                            if let Some(new_headers) = result.headers.as_ref() {
                                entry.request.headers = new_headers.clone();
                            }
                            if let Some(new_body) = result.body.as_ref() {
                                entry.request.body = Some(new_body.clone());
                            }
                        }
                    }

                    if let Some(status) = result.status {
                        // Support MOCK responses from handle_request
                        if let Ok(s) = StatusCode::from_u16(status) {
                            return RequestOrResponse::Response(
                                Response::builder()
                                    .status(s)
                                    .version(req_parts.version)
                                    .body(Body::from(Full::new(Bytes::from(final_bytes))))
                                    .unwrap()
                            );
                        }
                    }

                    RequestOrResponse::Request(Request::from_parts(
                        req_parts,
                        Body::from(Full::new(Bytes::from(final_bytes))),
                    ))
                }
                Err(_) => RequestOrResponse::Request(Request::from_parts(
                    parts,
                    Body::from(Full::new(Bytes::from(body_bytes))),
                )),
            }
        } else {
            RequestOrResponse::Request(Request::from_parts(
                parts,
                Body::from(Full::new(Bytes::from(body_bytes))),
            ))
        }
    }

    async fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        res: Response<Body>,
    ) -> Response<Body> {
        let id = self.request_id.unwrap_or(0);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let status = res.status().as_u16();
        let headers: Vec<(String, String)> = res
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let uri = self.request_uri.clone().unwrap_or_default();
        let absolute_uri = uri.clone();
        if !absolute_uri.contains("://") {
             // We don't have the original request headers here easily unless we stored them,
             // but we can try to find the host in the response if it was reflected (unlikely)
             // or just use the cached request_uri if it was already absolute.
             // Actually, handle_request should have stored the absolute one if possible.
        }

        // Always buffer for display
        let (parts, body) = res.into_parts();
        let body_bytes = Self::collect_body(body).await;

        // Body for display in UI/Scripts (already decompressed)
        let decompressed = Self::decompress_body(&headers, &body_bytes);
        let body = if decompressed.is_empty() {
            None
        } else {
            Some(decompressed)
        };

        let scripting_enabled = self.state.scripting_enabled.load(Ordering::Relaxed);
        let pattern_match = {
            let patterns = self.state.script_patterns.read().unwrap();
            patterns.iter().any(|re| re.is_match(&absolute_uri))
        };

        let should_script = scripting_enabled && self.request_method.as_deref() != Some("CONNECT") && pattern_match;

        let script_id = if should_script {
            self.state.next_script_id.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };

        let event = ProxyEvent {
            id,
            script_id,
            timestamp,
            method: self.request_method.clone().unwrap_or_default(),
            uri: absolute_uri,
            headers: headers.clone(),
            is_response: true,
            status: Some(status),
            body,
        };

        // Update history
        {
            let mut history = self.state.history.lock().await;
            if let Some(entry) = history.get_mut(&id.to_string()) {
                entry.response = Some(event.clone());
            } else {
                // If the entry doesn't exist for some reason, we still want it in history
                history.insert(id.to_string(), HistoryEntry {
                    request: ProxyEvent {
                        id,
                        script_id: 0,
                        timestamp: event.timestamp,
                        method: event.method.clone(),
                        uri: event.uri.clone(),
                        headers: Vec::new(),
                        is_response: false,
                        status: None,
                        body: None,
                    },
                    response: Some(event.clone()),
                });
            }
        }

        self.emit_event(event).await;

        if should_script {
            let (tx, rx) = oneshot::channel();
            self.state.script_pending.lock().await.insert(script_id, tx);

            match rx.await {
                Ok(result) => {
                    let mut res_parts = parts;
                    if let Some(status) = result.status {
                        if let Ok(s) = StatusCode::from_u16(status) {
                            res_parts.status = s;
                        }
                    }

                    if let Some(h) = result.headers.clone() {
                        res_parts.headers.clear();
                        for (k, v) in h {
                            if let (Ok(name), Ok(val)) = (
                                header::HeaderName::from_bytes(k.as_bytes()),
                                header::HeaderValue::from_str(&v),
                            ) {
                                res_parts.headers.insert(name, val);
                            }
                        }
                    }

                    let modified = result.body.is_some();
                    let final_bytes = result.body.clone().unwrap_or(body_bytes);
                    if modified {
                        // Remove encoding/length/integrity headers as they are now invalid
                        res_parts.headers.remove(header::CONTENT_ENCODING);
                        res_parts.headers.remove(header::TRANSFER_ENCODING);
                        if let Ok(content_md5) = header::HeaderName::from_lowercase(b"content-md5") {
                            res_parts.headers.remove(content_md5);
                        }
                        res_parts.headers.remove(header::ETAG);
                        res_parts.headers.insert(header::CONTENT_LENGTH, header::HeaderValue::from(final_bytes.len()));
                    }

                    // Update history with script modifications for detached windows
                    {
                        let mut history = self.state.history.lock().await;
                        if let Some(entry) = history.get_mut(&id.to_string()) {
                            if let Some(res_event) = entry.response.as_mut() {
                                if let Some(new_headers) = result.headers.as_ref() {
                                    res_event.headers = new_headers.clone();
                                }
                                if let Some(new_body) = result.body.as_ref() {
                                    res_event.body = Some(new_body.clone());
                                }
                                if let Some(new_status) = result.status {
                                    res_event.status = Some(new_status);
                                }
                            }
                        }
                    }
                    
                    Response::from_parts(
                        res_parts,
                        Body::from(Full::new(Bytes::from(final_bytes))),
                    )
                }
                Err(_) => Response::from_parts(
                    parts,
                    Body::from(Full::new(Bytes::from(body_bytes))),
                ),
            }
        } else {
            Response::from_parts(parts, Body::from(Full::new(Bytes::from(body_bytes))))
        }
    }

    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        if !self.state.intercept_ssl.load(Ordering::Relaxed) {
            return false;
        }

        let host = req.uri().host()
            .or_else(|| {
                req.headers()
                    .get(hudsucker::hyper::header::HOST)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|h| h.split(':').next())
            })
            .unwrap_or_default();

        if !host.is_empty() {
            let patterns = self.state.ssl_exception_patterns.read().unwrap();
            for re in patterns.iter() {
                if re.is_match(host) {
                    // log::info!("[Proxy] Bypassing SSL for host: {}", host);
                    return false;
                }
            }
        }

        true
    }
}
