use rcgen::{CertificateParams, KeyPair};
use tauri::{AppHandle, Manager, Runtime};

fn ca_paths<R: Runtime>(app: &AppHandle<R>) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok((data_dir.join("ca.crt"), data_dir.join("ca.key")))
}

pub fn load_or_create_ca<R: Runtime>(app: &AppHandle<R>) -> Result<(String, String), String> {
    let (cert_path, key_path) = ca_paths(app)?;

    if cert_path.exists() && key_path.exists() {
        let cert_pem = std::fs::read_to_string(&cert_path).map_err(|e| e.to_string())?;
        let key_pem = std::fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
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
    let key_pem = key_pair.serialize_pem();

    std::fs::write(&cert_path, &cert_pem).map_err(|e| e.to_string())?;
    std::fs::write(&key_path, &key_pem).map_err(|e| e.to_string())?;

    Ok((cert_pem, key_pem))
}
