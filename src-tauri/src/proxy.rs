use hudsucker::{
    certificate_authority::RcgenAuthority,
    hyper::{header, Request, Response, StatusCode},
    rcgen::{CertificateParams, KeyPair, Issuer},
    rustls::crypto::ring,
    HttpContext, HttpHandler, Proxy, RequestOrResponse,
};
use hudsucker::hyper::body::Bytes;
use http_body_util::Full;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use std::net::SocketAddr;

#[derive(Clone)]
pub struct ProxyHandler {
    app_handle: AppHandle,
    ca_cert_pem: String,
    request_map: Arc<Mutex<std::collections::HashMap<std::net::SocketAddr, Vec<u64>>>>,
}

#[derive(Clone, Serialize)]
pub struct ProxyEvent {
    pub id: u64,
    pub timestamp: u64,
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub is_response: bool,
    pub status: Option<u16>,
    pub body_base64: Option<String>,
}

impl ProxyHandler {
    pub fn new(app_handle: AppHandle, ca_cert_pem: String) -> Self {
        Self {
            app_handle,
            ca_cert_pem,
            request_map: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn generate_id() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

use http_body_util::BodyExt;
use base64::{Engine as _, engine::general_purpose::STANDARD};

impl HttpHandler for ProxyHandler {
    async fn handle_request(
        &mut self,
        _ctx: &HttpContext,
        req: Request<hudsucker::Body>,
    ) -> RequestOrResponse {
        // Serve a landing page on the root or /cert to download the CA
         if req.uri().path() == "/" || req.uri().path() == "/cert" {
            let html = format!(
                "<html>
                <head><title>Antigravity Proxy</title><meta name='viewport' content='width=device-width, initial-scale=1.0'></head>
                <body style='font-family: sans-serif; text-align: center; margin-top: 50px;'>
                    <h2>Debugging Proxy CA</h2>
                    <p>Install this root certificate to allow HTTPS decryption.</p>
                    <a href='/ca.crt' style='display:inline-block; padding: 15px 25px; background: #6366f1; color: white; text-decoration: none; border-radius: 8px; font-weight: bold;'>Download CA Certificate</a>
                </body>
                </html>"
            );
            return RequestOrResponse::Response(
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(hudsucker::Body::from(Full::new(Bytes::from(html))))
                    .unwrap()
            );
        }

        if req.uri().path() == "/ca.crt" {
            let res = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/x-x509-ca-cert")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"antigravity-proxy-ca.crt\"")
                .body(hudsucker::Body::from(Full::new(Bytes::from(self.ca_cert_pem.clone()))))
                .unwrap();
            return RequestOrResponse::Response(res);
        }

        let mut req = req;
        let mut headers = Vec::new();
        for (k, v) in req.headers().iter() {
            headers.push((k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).to_string()));
        }

        // Remove Accept-Encoding so the server sends uncompressed bodies
        req.headers_mut().remove(hudsucker::hyper::header::ACCEPT_ENCODING);

        let method = req.method().to_string();
        let uri = req.uri().to_string();
        let id = Self::generate_id();
        let is_connect = method == "CONNECT";

        if !is_connect {
            let mut map = self.request_map.lock().await;
            map.entry(_ctx.client_addr).or_default().push(id);
        }
        let is_upgrade = req.headers().get(header::UPGRADE).is_some();

        let (body_base64, req) = if is_connect || is_upgrade {
            (None, req) // Pass through directly, do not break the streaming tunnel
        } else {
            let (parts, body) = req.into_parts();
            let body_bytes = match body.collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => Bytes::new(), // Error reading body
            };
            let b64 = if body_bytes.is_empty() { None } else { Some(STANDARD.encode(&*body_bytes)) };
            let new_req = Request::from_parts(parts, hudsucker::Body::from(Full::new(body_bytes)));
            (b64, new_req)
        };

        let event = ProxyEvent {
            id,
            timestamp: id / 1_000_000,
            method,
            uri,
            headers,
            is_response: false,
            status: None,
            body_base64,
        };

        let _ = self.app_handle.emit("proxy_request", &event);
        
        RequestOrResponse::Request(req)
    }

    async fn handle_response(
        &mut self,
        ctx: &HttpContext,
        res: Response<hudsucker::Body>,
    ) -> Response<hudsucker::Body> {
        let mut headers = Vec::new();
        for (k, v) in res.headers().iter() {
            headers.push((k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).to_string()));
        }

        let id = {
            let mut map = self.request_map.lock().await;
            if let Some(ids) = map.get_mut(&ctx.client_addr) {
                if !ids.is_empty() {
                    ids.remove(0)
                } else {
                    Self::generate_id()
                }
            } else {
                Self::generate_id()
            }
        };

        let is_upgrade = res.status() == hudsucker::hyper::StatusCode::SWITCHING_PROTOCOLS;
        let is_sse = res.headers().get(header::CONTENT_TYPE)
            .map(|v| v.as_bytes().starts_with(b"text/event-stream"))
            .unwrap_or(false);

        let (body_base64, res) = if is_upgrade || is_sse {
            (None, res)
        } else {
            let (parts, body) = res.into_parts();
            let body_bytes = match body.collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => Bytes::new(),
            };
            let b64 = if body_bytes.is_empty() { None } else { Some(STANDARD.encode(&*body_bytes)) };
            let new_res = Response::from_parts(parts, hudsucker::Body::from(Full::new(body_bytes)));
            (b64, new_res)
        };

        let event = ProxyEvent {
            id,
            timestamp: id / 1_000_000,
            method: "".to_string(), // Can't easily recover method without storing mapping
            uri: "".to_string(),    // or ctx.uri()
            headers,
            is_response: true,
            status: Some(res.status().as_u16()),
            body_base64, 
        };

        let _ = self.app_handle.emit("proxy_response", &event);
        
        res
    }
}

pub struct ProxyState {
    pub is_running: Arc<Mutex<bool>>,
    pub ca_cert_pem: Arc<Mutex<Option<String>>>,
    pub port: Arc<Mutex<Option<u16>>>,
    pub proxy_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

#[tauri::command]
pub async fn get_local_ip() -> Result<String, String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
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
        // Validate that the key is still parseable
        if KeyPair::from_pem(&key_pem).is_ok() {
            log::info!("Loaded existing CA from {:?}", cert_path);
            return Ok((cert_pem, key_pem));
        }
        log::warn!("Existing CA key invalid, regenerating...");
    }

    // Generate a new CA
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
    log::info!("Generated new CA, saved to {:?}", cert_path);

    Ok((cert_pem, key_pem))
}

#[tauri::command]
pub async fn start_proxy(
    app: AppHandle,
    state: tauri::State<'_, ProxyState>,
    port: u16,
) -> Result<(), String> {
    let is_running = state.is_running.lock().await;
    if *is_running {
        return Err("Proxy is already running".into());
    }

    let (cert_pem, key_pem) = load_or_create_ca(&app)?;

    let key_pair_parsed = KeyPair::from_pem(&key_pem).map_err(|e| e.to_string())?;
    let issuer = Issuer::from_ca_cert_pem(&cert_pem, key_pair_parsed).map_err(|e| e.to_string())?;

    let ca = RcgenAuthority::new(issuer, 1_000, ring::default_provider());
    let handler = ProxyHandler::new(app, cert_pem.clone());

    let proxy = Proxy::builder()
        .with_addr(SocketAddr::from(([0, 0, 0, 0], port)))
        .with_ca(ca)
        .with_rustls_connector(ring::default_provider())
        .with_http_handler(handler)
        .build()
        .map_err(|e| e.to_string())?;

    let is_running_clone = state.is_running.clone();
    let handle = tokio::spawn(async move {
        *is_running_clone.lock().await = true;
        if let Err(e) = proxy.start().await {
            log::error!("Proxy error: {}", e);
        }
        *is_running_clone.lock().await = false;
    });

    *state.proxy_task.lock().await = Some(handle);
    *state.ca_cert_pem.lock().await = Some(cert_pem);
    *state.port.lock().await = Some(port);

    Ok(())
}

#[tauri::command]
pub async fn stop_proxy(state: tauri::State<'_, ProxyState>) -> Result<(), String> {
    let mut task_opt = state.proxy_task.lock().await;
    if let Some(handle) = task_opt.take() {
        handle.abort();
        *state.is_running.lock().await = false;
        *state.port.lock().await = None;
        return Ok(());
    }
    Err("Proxy is not running".into())
}

#[tauri::command]
pub async fn get_ca_cert(state: tauri::State<'_, ProxyState>) -> Result<Option<String>, String> {
    Ok(state.ca_cert_pem.lock().await.clone())
}
