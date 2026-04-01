use base64::{engine::general_purpose::STANDARD, Engine as _};
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
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::{oneshot, Mutex};

#[derive(Clone, serde::Serialize, specta::Type)]
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

#[derive(Clone, Serialize, Deserialize, Debug, specta::Type)]
pub struct ScriptResult {
    pub headers: Option<Vec<(String, String)>>,
    pub uri: Option<String>,
    pub status: Option<u16>,
    pub body_base64: Option<String>,
    pub dropped: bool,
}

#[derive(Clone)]
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

pub struct ProxyHandler<R: Runtime> {
    app_handle: AppHandle<R>,
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

impl<R: Runtime> Clone for ProxyHandler<R> {
    fn clone(&self) -> Self {
        Self {
            app_handle: self.app_handle.clone(),
            intercept_ssl: self.intercept_ssl.clone(),
            is_running: self.is_running.clone(),
            next_id: self.next_id.clone(),
            next_script_id: self.next_script_id.clone(),
            scripting_enabled: self.scripting_enabled.clone(),
            script_pending: self.script_pending.clone(),
            script_patterns: self.script_patterns.clone(),
            ca_cert_pem: self.ca_cert_pem.clone(),
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

    async fn emit_event(&self, event: ProxyEvent) {
        let _ = self.app_handle.emit("proxy-event", event);
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
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
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
        self.request_uri = Some(uri.clone());

        // Always buffer so the inspector can display the body
        let (parts, body) = req.into_parts();
        let body_bytes = Self::collect_body(body).await;
        
        // Decompress for display in UI/Scripts if needed
        let decompressed = Self::decompress_body(&headers, &body_bytes);
        let body_base64 = if decompressed.is_empty() {
            None
        } else {
            Some(STANDARD.encode(&decompressed))
        };

        let should_script = self.scripting_enabled.load(Ordering::Relaxed) && {
            let patterns = self.script_patterns.read().unwrap();
            patterns.iter().any(|re| re.is_match(&uri))
        };

        let script_id = if should_script {
            self.next_script_id.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };

        let event = ProxyEvent {
            id,
            script_id,
            timestamp,
            method: method.clone(),
            uri: uri.clone(),
            headers: headers.clone(),
            is_response: false,
            status: None,
            body_base64,
        };

        if should_script {
            let (tx, rx) = oneshot::channel();
            self.script_pending.lock().await.insert(script_id, tx);
            self.emit_event(event).await;

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

                    let mut builder = Request::builder()
                        .method(parts.method.clone())
                        .version(parts.version);

                    if let Some(new_uri) = result.uri {
                        builder = builder.uri(new_uri);
                    } else {
                        builder = builder.uri(parts.uri.clone());
                    }

                    let mut new_headers = parts.headers.clone();
                    if let Some(h) = result.headers {
                        new_headers.clear();
                        for (k, v) in h {
                            if let (Ok(name), Ok(val)) = (
                                header::HeaderName::from_bytes(k.as_bytes()),
                                header::HeaderValue::from_str(&v),
                            ) {
                                new_headers.insert(name, val);
                            }
                        }
                    }
                    *builder.headers_mut().unwrap() = new_headers;

                    let final_bytes = if let Some(b64) = result.body_base64 {
                        STANDARD.decode(b64).unwrap_or(body_bytes)
                    } else {
                        body_bytes
                    };

                    RequestOrResponse::Request(
                        builder
                            .body(Body::from(Full::new(Bytes::from(final_bytes))))
                            .unwrap(),
                    )
                }
                Err(_) => RequestOrResponse::Request(Request::from_parts(
                    parts,
                    Body::from(Full::new(Bytes::from(body_bytes))),
                )),
            }
        } else {
            self.emit_event(event).await;
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

        // Always buffer for display
        let (parts, body) = res.into_parts();
        let body_bytes = Self::collect_body(body).await;

        // Decompress for display in UI/Scripts if needed
        let decompressed = Self::decompress_body(&headers, &body_bytes);
        let body_base64 = if decompressed.is_empty() {
            None
        } else {
            Some(STANDARD.encode(&decompressed))
        };

        let should_script = self.scripting_enabled.load(Ordering::Relaxed) && {
            let patterns = self.script_patterns.read().unwrap();
            patterns.iter().any(|re| re.is_match(&uri))
        };

        let script_id = if should_script {
            self.next_script_id.fetch_add(1, Ordering::Relaxed)
        } else {
            0
        };

        let event = ProxyEvent {
            id,
            script_id,
            timestamp,
            method: self.request_method.clone().unwrap_or_default(),
            uri: self.request_uri.clone().unwrap_or_default(),
            headers: headers.clone(),
            is_response: true,
            status: Some(status),
            body_base64,
        };

        if should_script {
            let (tx, rx) = oneshot::channel();
            self.script_pending.lock().await.insert(script_id, tx);
            self.emit_event(event).await;

            match rx.await {
                Ok(result) => {
                    let new_status = result.status.unwrap_or(status);
                    let mut builder = Response::builder()
                        .status(new_status)
                        .version(parts.version);

                    let mut new_headers = parts.headers.clone();
                    if let Some(h) = result.headers {
                        new_headers.clear();
                        for (k, v) in h {
                            if let (Ok(name), Ok(val)) = (
                                header::HeaderName::from_bytes(k.as_bytes()),
                                header::HeaderValue::from_str(&v),
                            ) {
                                new_headers.insert(name, val);
                            }
                        }
                    }
                    *builder.headers_mut().unwrap() = new_headers;

                    let final_bytes = if let Some(b64) = result.body_base64 {
                        STANDARD.decode(b64).unwrap_or(body_bytes)
                    } else {
                        body_bytes
                    };

                    builder
                        .body(Body::from(Full::new(Bytes::from(final_bytes))))
                        .unwrap()
                }
                Err(_) => Response::from_parts(
                    parts,
                    Body::from(Full::new(Bytes::from(body_bytes))),
                ),
            }
        } else {
            self.emit_event(event).await;
            Response::from_parts(parts, Body::from(Full::new(Bytes::from(body_bytes))))
        }
    }

    async fn should_intercept(&mut self, _ctx: &HttpContext, _req: &Request<Body>) -> bool {
        self.intercept_ssl.load(Ordering::Relaxed)
    }
}
