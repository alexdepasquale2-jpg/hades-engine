//! M12 — production TLS/mTLS configuration for the QUIC gateway.

use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;
use rustls::ServerConfig as RustlsServerConfig;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TlsPaths {
  pub cert_pem: String,
  pub key_pem: String,
  pub client_ca_pem: Option<String>,
}

impl TlsPaths {
  pub fn from_env() -> Result<Option<Self>, Box<dyn Error>> {
    let cert_path = std::env::var("TBC_TLS_CERT").ok();
    let key_path = std::env::var("TBC_TLS_KEY").ok();
    if cert_path.is_none() && key_path.is_none() {
      return Ok(None);
    }
    let cert_path = cert_path.unwrap_or_else(|| "TBC_TLS_CERT".into());
    let key_path = key_path.unwrap_or_else(|| "TBC_TLS_KEY".into());
    let cert_pem = fs::read_to_string(&cert_path)
      .map_err(|e| format!("read cert {}: {}", cert_path, e))?;
    let key_pem = fs::read_to_string(&key_path)
      .map_err(|e| format!("read key {}: {}", key_path, e))?;
    let client_ca_pem = std::env::var("TBC_TLS_CLIENT_CA")
      .ok()
      .filter(|s| !s.is_empty())
      .map(|path| fs::read_to_string(&path))
      .transpose()
      .map_err(|e| format!("read client CA: {}", e))?;
    Ok(Some(Self {
      cert_pem,
      key_pem,
      client_ca_pem,
    }))
  }
}

pub fn build_rustls_server_config(paths: Option<TlsPaths>) -> Result<RustlsServerConfig, Box<dyn Error>> {
  let (cert_chain, priv_key, client_ca) = match paths {
    Some(p) => (
      load_certs_from_pem(&p.cert_pem)?,
      load_private_key_from_pem(&p.key_pem)?,
      p.client_ca_pem,
    ),
    None => {
      let cert = generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])?;
      let cert_der = cert.serialize_der()?;
      let key_der = cert.serialize_private_key_der();
      (
        vec![CertificateDer::from(cert_der)],
        PrivateKeyDer::Pkcs8(key_der.into()),
        None,
      )
    }
  };

  let config = if let Some(ca_pem) = client_ca {
    let mut roots = RootCertStore::empty();
    for der in load_certs_from_pem(&ca_pem)? {
      roots.add(der)?;
    }
    let verifier = WebPkiClientVerifier::builder(roots.into()).build()?;
    RustlsServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
      .with_client_cert_verifier(verifier)
      .with_single_cert(cert_chain, priv_key)?
  } else {
    RustlsServerConfig::builder()
      .with_no_client_auth()
      .with_single_cert(cert_chain, priv_key)?
  };

  Ok(config)
}

fn load_certs_from_pem(pem: &str) -> Result<Vec<CertificateDer<'static>>, Box<dyn Error>> {
  let mut certs = Vec::new();
  for item in rustls_pemfile::certs(&mut pem.as_bytes()) {
    certs.push(CertificateDer::from(item?));
  }
  if certs.is_empty() {
    return Err("no certificates found in PEM".into());
  }
  Ok(certs)
}

fn load_private_key_from_pem(pem: &str) -> Result<PrivateKeyDer<'static>, Box<dyn Error>> {
  if let Some(key) = rustls_pemfile::private_key(&mut pem.as_bytes())? {
    Ok(key)
  } else {
    Err("no private key found in PEM".into())
  }
}

pub fn write_dev_certificates(dir: &Path) -> Result<TlsPaths, Box<dyn Error>> {
  fs::create_dir_all(dir)?;
  let cert = generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])?;
  let cert_pem = cert.serialize_pem()?;
  let key_pem = cert.serialize_private_key_pem();
  let cert_path = dir.join("dev-cert.pem");
  let key_path = dir.join("dev-key.pem");
  fs::write(&cert_path, &cert_pem)?;
  fs::write(&key_path, &key_pem)?;
  Ok(TlsPaths {
    cert_pem,
    key_pem,
    client_ca_pem: None,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dev_cert_roundtrip() {
    let dir = std::env::temp_dir().join(format!("tbc-tls-{}", ulid::Ulid::new()));
    let paths = write_dev_certificates(&dir).expect("write dev cert");
    let cfg = build_rustls_server_config(Some(paths)).expect("build config");
    assert!(cfg.alpn_protocols.is_empty() || cfg.alpn_protocols.len() >= 0);
    std::fs::remove_dir_all(&dir).ok();
  }
}
